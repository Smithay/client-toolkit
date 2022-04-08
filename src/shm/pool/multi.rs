//! A pool implementation which automatically manage buffers.
//!
//! This pool is built on the [`RawPool`].
//!
//! The [`MultiPool`] takes a key which is used to identify buffers and tries to return the buffer associated to the key
//! if possible. If no buffer in the pool is associated to the key, it will create a new one.
//!
//! # Example
//!
//! ```rust
//! use smithay_client_toolkit::reexports::client::{
//!     ConnectionHandle,
//!     QueueHandle,
//!     protocol::wl_surface::WlSurface,
//!     protocol::wl_shm::Format,
//! };
//! use smithay_client_toolkit::shm::pool::multi::MultiPool;
//!
//! struct WlFoo {
//!     // The surface we'll draw on and the index of buffer associated to it
//!     surface: (WlSurface, usize),
//!     pool: MultiPool<(WlSurface, usize)>
//! }
//!
//! impl WlFoo {
//!     fn draw(&mut self, conn: &mut ConnectionHandle, qh: &QueueHandle<WlFoo>) {
//!         let surface = &self.surface.0;
//!         // We'll increment "i" until the pool can create a new buffer
//!         // if there's no buffer associated with our surface and "i" or if
//!         // a buffer with the obuffer associated with our surface and "i" is free for use.
//!         //
//!         // There's no limit to the amount of buffers we can allocate to our surface but since
//!         // shm buffers are released fairly fast, it's unlikely we'll need more than double buffering.
//!         for i in 0..2 {
//!             self.surface.1 = i;
//!             if let Ok((offset, buffer, slice)) = self.pool.create_buffer(
//!                 100,
//!                 100 * 4,
//!                 100,
//!                 &self.surface,
//!                 Format::Argb8888,
//!                 conn,
//!             ) {
//!                 /*
//!                     insert drawing code here
//!                 */
//!                 surface.attach(conn, Some(buffer), 0, 0);
//!                 surface.commit(conn);
//!                 // We exit the function after the draw.
//!                 return;
//!             }
//!         }
//!         /*
//!             If there's no buffer available we can for example request a frame callback
//!             and trigger a redraw when it fires.
//!             (not shown in this example)
//!         */
//!     }
//! }
//!
//! fn draw(slice: &mut [u8]) {
//!     todo!()
//! }
//!
//! ```
//!

use std::io;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use wayland_client::{
    protocol::{wl_buffer, wl_shm, wl_shm_pool},
    ConnectionHandle, Proxy, WEnum,
};

use super::raw::RawPool;

#[derive(Debug)]
pub enum PoolError {
    InUse,
    Overlap,
    NotFound,
}

/// This pool manages buffers associated with keys.
/// Only one buffer can be attributed to a given key.
#[derive(Debug)]
pub struct MultiPool<K: Clone + PartialEq> {
    buffer_list: Vec<BufferSlot<K>>,
    pub(crate) inner: RawPool,
}

#[derive(Debug, thiserror::Error)]
struct BufferSlot<K: Clone + PartialEq> {
    free: Arc<AtomicBool>,
    size: usize,
    used: usize,
    offset: usize,
    buffer: Option<wl_buffer::WlBuffer>,
    key: K,
}

impl<K: Clone + PartialEq> MultiPool<K> {
    pub(crate) fn new(inner: RawPool) -> Self {
        Self { inner, buffer_list: Vec::new() }
    }
    /// Resizes the memory pool, notifying the server the pool has changed in size.
    ///
    /// The wl_shm protocol only allows the pool to be made bigger. If the new size is smaller than the
    /// current size of the pool, this function will do nothing.
    pub fn resize(&mut self, size: usize, conn: &mut ConnectionHandle) -> io::Result<()> {
        self.inner.resize(size, conn)
    }
    /// Removes the buffer with the given key from the pool and rearranges the others.
    pub fn remove<Q>(&mut self, key: &Q, conn: &mut ConnectionHandle)
    where
        Q: Eq,
        K: std::borrow::Borrow<Q>,
    {
        if let Some((i, buf)) =
            self.buffer_list.iter().enumerate().find(|(_, slot)| slot.key.borrow().eq(key))
        {
            let mut offset = buf.offset;
            if let Some(buf) = self.buffer_list.remove(i).buffer {
                buf.destroy(conn);
            }
            for buf_slot in &mut self.buffer_list {
                if buf_slot.offset > offset && buf_slot.free.load(Ordering::Relaxed) {
                    if let Some(buffer) = buf_slot.buffer.take() {
                        buffer.destroy(conn);
                    }
                    std::mem::swap(&mut buf_slot.offset, &mut offset);
                } else {
                    break;
                }
            }
        }
    }
    /// Insert a buffer into the pool.
    pub fn insert(
        &mut self,
        width: i32,
        stride: i32,
        height: i32,
        key: &K,
        format: wl_shm::Format,
        conn: &mut ConnectionHandle,
    ) -> Result<usize, PoolError> {
        let mut offset = 0;
        let mut found_key = false;
        let size = (stride * height) as usize;
        let mut index = Err(PoolError::NotFound);
        let bpp = (stride as f32 / width as f32).ceil() as usize;

        for (i, buf_slot) in self.buffer_list.iter_mut().enumerate() {
            if buf_slot.key.eq(key) {
                found_key = true;
                if buf_slot.free.load(Ordering::Relaxed) {
                    // Destroys the buffer if it's resized
                    if size != buf_slot.used {
                        if let Some(buffer) = buf_slot.buffer.take() {
                            buffer.destroy(conn);
                        }
                    }
                    // Increases the size of the Buffer if it's too small and add 5% padding.
                    // It is possible this buffer overlaps the following but the else if
                    // statement prevents this buffer from being returned if that's the case.
                    buf_slot.size = buf_slot.size.max(size + size / 20);
                    index = Ok(i);
                } else {
                    index = Err(PoolError::InUse);
                }
            // If a buffer is resized, it is likely that the followings might overlap
            } else if offset > buf_slot.offset {
                // When the buffer is free, it's safe to shift it because we know the compositor won't try to read it.
                if buf_slot.free.load(Ordering::Relaxed) {
                    if offset != buf_slot.offset {
                        if let Some(buffer) = buf_slot.buffer.take() {
                            buffer.destroy(conn);
                        }
                    }
                    buf_slot.offset = offset;
                } else {
                    // If one of the overlapping buffers is busy, then no buffer can be returned because it could result in a data race.
                    index = Err(PoolError::InUse);
                }
            } else if found_key {
                break;
            }
            let mut size = buf_slot.size;
            size += size % bpp;
            size -= size % bpp;
            offset += size;
        }

        if !found_key {
            if let Err(err) = index {
                return self
                    .dyn_resize(offset, width, stride, height, key.clone(), format, conn)
                    .map(|_| self.buffer_list.len() - 1)
                    .ok_or(err);
            }
        }

        index
    }
    /// Retreives the buffer associated with the given key.
    pub fn get<Q>(
        &mut self,
        width: i32,
        stride: i32,
        height: i32,
        key: &Q,
        format: wl_shm::Format,
        conn: &mut ConnectionHandle,
    ) -> Option<(usize, &wl_buffer::WlBuffer, &mut [u8])>
    where
        Q: PartialEq,
        K: std::borrow::Borrow<Q>,
    {
        let len = self.inner.len();
        let size = (stride * height) as usize;
        let buf_slot =
            self.buffer_list.iter_mut().find(|buf_slot| buf_slot.key.borrow().eq(key))?;
        buf_slot.used = size;
        let offset = buf_slot.offset;
        if buf_slot.buffer.is_none() {
            if offset + size > len {
                self.inner.resize(offset + size + size / 20, conn).ok()?;
            }
            let free = Arc::new(AtomicBool::new(true));
            let buffer_id = conn
                .send_request(
                    self.inner.pool(),
                    wl_shm_pool::Request::CreateBuffer {
                        offset: offset as i32,
                        width,
                        height,
                        stride,
                        format: WEnum::Value(format),
                    },
                    Some(Arc::new(BufferObjectData { free: free.clone() })),
                )
                .ok()?;
            buf_slot.free = free;
            buf_slot.buffer = Proxy::from_id(conn, buffer_id).ok();
        }
        let buf = buf_slot.buffer.as_ref().unwrap();
        buf_slot.free.store(false, Ordering::Relaxed);
        Some((offset, buf, &mut self.inner.mmap()[offset..][..size]))
    }
    /// Returns the buffer associated with the given key and its offset (usize) in the mempool.
    ///
    /// The offset can be used to determine whether or not a buffer was moved in the mempool
    /// and by consequence if it should be damaged partially or fully.
    pub fn create_buffer(
        &mut self,
        width: i32,
        stride: i32,
        height: i32,
        key: &K,
        format: wl_shm::Format,
        conn: &mut ConnectionHandle,
    ) -> Result<(usize, &wl_buffer::WlBuffer, &mut [u8]), PoolError> {
        let index = self.insert(width, stride, height, key, format, conn)?;
        self.get_at(index, width, stride, height, format, conn).ok_or(PoolError::NotFound)
    }
    /// Retreives the buffer at the given index.
    fn get_at(
        &mut self,
        index: usize,
        width: i32,
        stride: i32,
        height: i32,
        format: wl_shm::Format,
        conn: &mut ConnectionHandle,
    ) -> Option<(usize, &wl_buffer::WlBuffer, &mut [u8])> {
        let len = self.inner.len();
        let size = (stride * height) as usize;
        let buf_slot = self.buffer_list.get_mut(index)?;
        buf_slot.used = size;
        let offset = buf_slot.offset;
        if buf_slot.buffer.is_none() {
            if offset + size > len {
                self.inner.resize(offset + size + size / 20, conn).ok()?;
            }
            let free = Arc::new(AtomicBool::new(true));
            let buffer_id = conn
                .send_request(
                    self.inner.pool(),
                    wl_shm_pool::Request::CreateBuffer {
                        offset: offset as i32,
                        width,
                        height,
                        stride,
                        format: WEnum::Value(format),
                    },
                    Some(Arc::new(BufferObjectData { free: free.clone() })),
                )
                .ok()?;
            buf_slot.free = free;
            buf_slot.buffer = Proxy::from_id(conn, buffer_id).ok();
        }
        buf_slot.free.store(false, Ordering::Relaxed);
        let buf = buf_slot.buffer.as_ref().unwrap();
        Some((offset, buf, &mut self.inner.mmap()[offset..][..size]))
    }
    /// Calcule the offet and size of a buffer based on its stride.
    fn offset(&self, mut offset: i32, width: i32, stride: i32, height: i32) -> (usize, usize) {
        // bytes per pixel
        let bpp = (stride as f32 / width as f32).ceil() as i32;
        let size = stride * height;
        // 5% padding.
        offset += offset / 20;
        offset += offset % bpp;
        offset -= offset % bpp;
        (offset as usize, size as usize)
    }
    /// Resizes the pool and appends a new buffer.
    fn dyn_resize(
        &mut self,
        offset: usize,
        width: i32,
        stride: i32,
        height: i32,
        key: K,
        format: wl_shm::Format,
        conn: &mut ConnectionHandle,
    ) -> Option<()> {
        let (offset, size) = self.offset(offset as i32, width, stride, height);
        if self.inner.len() < offset + size {
            self.resize(offset + size + size / 20, conn).ok()?;
        }
        let free = Arc::new(AtomicBool::new(true));
        let buffer_id = conn
            .send_request(
                self.inner.pool(),
                wl_shm_pool::Request::CreateBuffer {
                    offset: offset as i32,
                    width,
                    height,
                    stride,
                    format: WEnum::Value(format),
                },
                Some(Arc::new(BufferObjectData { free: free.clone() })),
            )
            .ok()?;
        let buffer = Proxy::from_id(conn, buffer_id).ok()?;
        self.buffer_list.push(BufferSlot {
            offset,
            used: 0,
            free,
            buffer: Some(buffer),
            size,
            key,
        });
        Some(())
    }
}

struct BufferObjectData {
    free: Arc<AtomicBool>,
}

impl wayland_client::backend::ObjectData for BufferObjectData {
    fn event(
        self: Arc<Self>,
        _: &mut wayland_backend::client::Handle,
        msg: wayland_backend::protocol::Message<wayland_backend::client::ObjectId>,
    ) -> Option<Arc<dyn wayland_backend::client::ObjectData>> {
        debug_assert!(wayland_client::backend::protocol::same_interface(
            msg.sender_id.interface(),
            wl_buffer::WlBuffer::interface()
        ));
        debug_assert!(msg.opcode == 0);
        // wl_buffer only has a single event: wl_buffer.release
        self.free.store(true, Ordering::Relaxed);
        None
    }

    fn destroyed(&self, _: wayland_backend::client::ObjectId) {}
}
