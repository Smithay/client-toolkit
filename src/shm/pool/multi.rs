//! A pool implementation which automatically frees buffers when released.

use std::io;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use wayland_client::ConnectionHandle;
use wayland_client::protocol::{
    wl_buffer,wl_shm
};
use wayland_client::{QueueHandle, Dispatch, DelegateDispatchBase, DelegateDispatch};

use super::raw::RawPool;

/// This pool manages multiple buffers associated with a surface.
/// Only one buffer can be attributed to a surface.
#[derive(Debug)]
pub struct MultiPool<I: PartialEq + Clone> {
    buffer_list: Vec<Buffer<I>>,
    pub(crate) inner: RawPool,
}

#[derive(Debug)]
struct Buffer<I: PartialEq + Clone> {
    free: AtomicBool,
    size: usize,
    offset: usize,
    buffer: wl_buffer::WlBuffer,
    identifier: I,
}

impl<E: PartialEq + Clone> From<RawPool> for MultiPool<E> {
    fn from(inner: RawPool) -> Self {
        Self {
            buffer_list: Vec::new(),
            inner
        }
    }
}

impl<I: PartialEq + Clone> MultiPool<I> {
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
    /// Removes the buffer with the given identifier from the pool and rearranges the others
    pub fn remove(&mut self, identifier: &I) {
        if let Some((i, buffer)) =
            self.buffer_list
            .iter()
            .enumerate()
            .find(|b| b.1.identifier.eq(identifier)) {
            let mut offset = buffer.offset;
            self.buffer_list.remove(i);
            for buffer in &mut self.buffer_list {
                if buffer.offset > offset {
                    let l_offset = buffer.offset;
                    buffer.offset = offset;
                    offset = l_offset;
                }
            }
        }
    }
    /// Returns the buffer associated with the given identifier.
    /// If it's not possible to use the buffer associated with the identifier, None is returned.
    pub fn create_buffer<D, U>(
        &mut self,
        width: i32,
        stride: i32,
        height: i32,
        identifier: &I,
        format: wl_shm::Format,
        udata: U,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) -> Option<(usize, wl_buffer::WlBuffer, &mut [u8])>
    where
        D: Dispatch<wl_buffer::WlBuffer, UserData = U> + 'static,
        U: Send + Sync + 'static,
    {
        let mut found_identifier = false;
        let mut offset = 0;
        let size = stride * height;
        let mut index = None;
        for (i, buffer) in self.buffer_list.iter_mut().enumerate() {
            if buffer.identifier.eq(identifier) {
                found_identifier = true;
                if buffer.free.load(Ordering::Relaxed) {
                    buffer.size = buffer.size.max(size as usize);
                    index = Some(i);
                }
            } else if offset > buffer.offset {
                if buffer.free.load(Ordering::Relaxed) {
                    buffer.offset = offset;
                } else {
                    index = None;
                }
            } else if found_identifier {
                break;
            }
            offset += buffer.size;
        }

        if found_identifier {
            offset = self.buffer_list[index?].offset;
        } else if let Some(b) = self.buffer_list.last() {
            offset += b.size;
        }

        if offset + size as usize > self.inner.len
        && self.resize(offset + 2 * size as usize, conn).is_err() {
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

        if found_identifier {
            self.buffer_list[index?].buffer = buffer.clone();
        } else if index.is_none() {
            index = Some(self.buffer_list.len());
            let buffer = Buffer {
                offset,
                free: AtomicBool::new(true),
                buffer: buffer.clone(),
                size: size as usize,
                identifier: identifier.clone()
            };
            self.buffer_list.push(buffer);
        }

        let slice = &mut self.inner.mmap()[offset..][..size as usize];

        self.buffer_list[index?].free.swap(false, Ordering::Relaxed);

        Some((offset, buffer, slice))
    }
}

impl<I: PartialEq + Clone> DelegateDispatchBase<wl_buffer::WlBuffer> for MultiPool<I> {
    type UserData = ();
}

impl<D, I: PartialEq + Clone> DelegateDispatch<wl_buffer::WlBuffer, D> for MultiPool<I>
where
    D: Dispatch<wl_buffer::WlBuffer, UserData = Self::UserData>,
    D: AsMut<MultiPool<I>>
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
            data.as_mut().free(buffer);
        }
    }
}

