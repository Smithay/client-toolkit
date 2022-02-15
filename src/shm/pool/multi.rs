//! A pool implementation which automatically manage buffers.
//!
//!	This pool is built on the [`RawPool`].
//!
//!	The [`MultiPool`] takes a key which is used to identify buffers and tries to return the buffer associated to the key
//!	if possible. If no buffer in the pool is associated to the key, it will create a new one.
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
//! use smithay_client_toolkit::shm::pool::PoolHandle;
//!
//! struct WlFoo {
//! 	// The surface we'll draw on and the index of buffer associated to it
//! 	surface: (WlSurface, usize),
//! 	pool: MultiPool<(WlSurface, usize)>
//! }
//!
//!	impl AsPool<MultiPool<(WlSurface, usize)>> for WlFoo {
//!	    fn pool_handle(&self) -> PoolHandle<MultiPool<(WlSurface, usize)>> {
//!	        PoolHandle::Ref(&self.pool)
//!	    }
//!	}
//!
//! impl WlFoo {
//! 	fn draw(&mut self, conn: &mut ConnectionHandle, qh: &QueueHandle<WlFoo>) {
//! 		let surface = &surface.1;
//! 		// We'll increment "i" until the pool can create a new buffer
//! 		// if there's no buffer associated with our surface and "i" or if
//! 		// a buffer with the obuffer associated with our surface and "i" is free for use.
//! 		//
//! 		// There's no limit to the amount of buffers we can allocate to our surface but since
//! 		// shm buffers are released fairly fast, it's unlikely we'll need more than double buffering.
//! 		for i in 0..2 {
//! 			match self.pool.create_buffer(
//! 				100,
//! 				100,
//! 				&self.surface,
//! 				Format::Argb8888,
//! 				(),
//! 				conn,
//! 				qh
//! 			) {
//! 				Some((offset, buffer, slice)) => {
//! 					draw(slice);
//! 					surface.attach(conn, Some(&wl_buffer), 0, 0);
//! 					surface.commit(conn);
//! 					// We exit the function after the draw.
//! 					return;
//! 				}
//! 				None => self.surface.1 = i,
//! 			}
//! 		}
//! 		// If there's no buffer available we'll request a frame callback
//! 		// were this function will be called again.
//! 		surface.frame(conn, qh);
//! 		surface.commit(conn);
//! 	}
//! }
//!
//! fn draw(slice: &mut [u8]) {
//! 	todo!()
//! }
//!
//! ```
//!

use std::io;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use wayland_client::ConnectionHandle;
use wayland_client::protocol::{
    wl_buffer,wl_shm
};
use wayland_client::{QueueHandle, Dispatch, DelegateDispatchBase, DelegateDispatch};

use super::raw::RawPool;
use crate::shm::pool::PoolHandle;

/// This pool manages buffers associated with keys.
/// Only one buffer can be attributed to a given key.
#[derive(Debug)]
pub struct MultiPool<K: PartialEq + Clone> {
    buffer_list: Vec<Buffer<K>>,
    pub(crate) inner: RawPool,
}

#[derive(Debug)]
struct Buffer<K: PartialEq + Clone> {
    free: AtomicBool,
    size: usize,
    offset: usize,
    buffer: wl_buffer::WlBuffer,
    key: K,
}

impl<E: PartialEq + Clone> From<RawPool> for MultiPool<E> {
    fn from(inner: RawPool) -> Self {
        Self {
            buffer_list: Vec::new(),
            inner
        }
    }
}

impl<K: PartialEq + Clone> MultiPool<K> {
    /// Resizes the memory pool, notifying the server the pool has changed in size.
    ///
    /// The wl_shm protocol only allows the pool to be made bigger. If the new size is smaller than the
    /// current size of the pool, this function will do nothing.
    pub fn resize(&mut self, size: usize, conn: &mut ConnectionHandle) -> io::Result<()> {
        self.inner.resize(size, conn)
    }
    fn free(&self, buffer: &wl_buffer::WlBuffer) {
        if let Some(buffer) = self.buffer_list.iter().find(|b| b.buffer.eq(buffer)) {
            buffer.free.swap(true, Ordering::Relaxed);
        }
    }
    /// Removes the buffer with the given key from the pool and rearranges the others
    pub fn remove(&mut self, key: &K) {
        if let Some((i, buffer)) =
            self.buffer_list
            .iter()
            .enumerate()
            .find(|b| b.1.key.eq(key)) {
            let mut offset = buffer.offset;
            self.buffer_list.remove(i);
            for buffer in &mut self.buffer_list {
                if buffer.offset > offset && buffer.free.load(Ordering::Relaxed) {
                    let l_offset = buffer.offset;
                    buffer.offset = offset;
                    offset = l_offset;
                } else {
                    break;
                }
            }
        }
    }
    /// Returns the buffer associated with the given key and its offset (usize) in the mempool.
    ///
    /// The offset can be used to determine whether or not a buffer was moved in the mempool
    /// and by consequence if it should be damaged partially or fully.
    ///
    /// When it's not possible to use the buffer associated with the key, None is returned.
    pub fn create_buffer<D, U>(
        &mut self,
        width: i32,
        stride: i32,
        height: i32,
        key: &K,
        format: wl_shm::Format,
        udata: U,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) -> Option<(usize, wl_buffer::WlBuffer, &mut [u8])>
    where
        D: Dispatch<wl_buffer::WlBuffer, UserData = U> + 'static,
        U: Send + Sync + 'static,
    {
        let mut found_key = false;
        let mut offset = 0;
        let size = (stride * height) as usize;
        let mut index = None;

        // This loop serves to found the buffer associated to the key.
        for (i, buffer) in self.buffer_list.iter_mut().enumerate() {
            if buffer.key.eq(key) {
                found_key = true;
                if buffer.free.load(Ordering::Relaxed) {
                    // Increases the size of the Buffer if it's too small and add 5% padding.
                    // It is possible this buffer overlaps the following but the else if
                    // statement prevents this buffer from being returned if that's the case.
                    buffer.size = buffer.size.max(size + size / 20);
                    index = Some(i);
                }
            // If a buffer is resized, it is likely that the followings might overlap
            } else if offset > buffer.offset {
                // When the buffer is free, it's safe to shift it because we know the compositor won't try to read it.
                if buffer.free.load(Ordering::Relaxed) {
                    buffer.offset = offset;
                } else {
                    // If one of the overlapping buffers is busy, then no buffer can be returned because it could result in a data race.
                    index = None;
                }
            } else if found_key {
                break;
            }
            offset += buffer.size;
        }

        if found_key {
            // Sets the offset to the one of our chosen buffer
            offset = self.buffer_list[index?].offset;
        } else if let Some(b) = self.buffer_list.last() {
            // Adds 5% padding between the last and new buffer
            offset += b.size / 20;
        }

		// Resize the pool if it isn't large enough to fit all our buffers
        if offset + size >= self.inner.len()
        && self.resize(offset + size + size / 20, conn).is_err() {
            return None
        }

        let buffer = self.inner.create_buffer(
            offset as i32,
            width,
            height,
            stride,
            format, udata, conn,
            qh
        ).ok()?;

        if found_key {
            self.buffer_list[index?].buffer = buffer.clone();
        } else if index.is_none() {
            index = Some(self.buffer_list.len());
            let buffer = Buffer {
                offset,
                free: AtomicBool::new(true),
                buffer: buffer.clone(),
                size,
                key: key.clone()
            };
            self.buffer_list.push(buffer);
        }

        let slice = &mut self.inner.mmap()[offset..][..size];

        self.buffer_list[index?].free.swap(false, Ordering::Relaxed);

        Some((offset, buffer, slice))
    }
}

impl<K: PartialEq + Clone> DelegateDispatchBase<wl_buffer::WlBuffer> for MultiPool<K> {
    type UserData = ();
}

impl<D, K: PartialEq + Clone> DelegateDispatch<wl_buffer::WlBuffer, D> for MultiPool<K>
where
    D: Dispatch<wl_buffer::WlBuffer, UserData = Self::UserData>,
    for<'p> &'p mut D: Into<PoolHandle<'p, MultiPool<K>>>
{
    fn event(
        data: &mut D,
        buffer: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _: &Self::UserData,
        conn: &mut ConnectionHandle,
        _: &QueueHandle<D>
    ) {
        if let wl_buffer::Event::Release = event {
            buffer.destroy(conn);
            match data.into() {
                PoolHandle::Ref(pool) => {
                    pool.free(buffer);
                }
                PoolHandle::Slice(pools) => {
                    for pool in pools.iter() {
                        pool.free(buffer);
                    }
                }
                PoolHandle::Vec(pools) => {
                    for pool in pools.iter() {
                        pool.free(buffer);
                    }
                }
            }
        }
    }
}

