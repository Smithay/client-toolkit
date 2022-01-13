//! A pool implementation which automatically frees buffers when released.

use std::io;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use wayland_client::ConnectionHandle;
use wayland_client::protocol::{
    wl_buffer, wl_surface, wl_shm
};
use wayland_client::{QueueHandle, Dispatch, DelegateDispatchBase, DelegateDispatch};

use super::raw::RawPool;

/// This pool manages multiple buffers associated with a surface.
/// Only one buffer can be attributed to a surface.
#[derive(Debug)]
pub struct MultiPool {
    buffer_list: Vec<Buffer>,
    pub(crate) inner: RawPool,
}

#[derive(Debug)]
struct Buffer {
    free: AtomicBool,
    size: usize,
    offset: usize,
    buffer: wl_buffer::WlBuffer,
    surface: wl_surface::WlSurface,
}

impl From<RawPool> for MultiPool {
    fn from(inner: RawPool) -> Self {
        Self {
            buffer_list: Vec::new(),
            inner
        }
    }
}

impl MultiPool {
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
    /// Returns the buffer associated with the given surface.
    /// If the buffer isn't free, it returns None.
    pub fn create_buffer<D, U>(
        &mut self,
        width: i32,
        stride: i32,
        height: i32,
        surface: &wl_surface::WlSurface,
        format: wl_shm::Format,
        udata: U,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) -> Option<(wl_buffer::WlBuffer, &mut [u8])>
    where
        D: Dispatch<wl_buffer::WlBuffer, UserData = U> + 'static,
        U: Send + Sync + 'static,
    {
        let mut found_surface = false;
        let mut offset = 0;
        let size = stride * height;
        if size as usize > self.inner.len {
            return None
        }
        let mut index = None;
        for (i, buffer) in self.buffer_list.iter_mut().enumerate() {
            if buffer.surface.eq(surface) {
                found_surface = true;
                buffer.size = buffer.size.max(size as usize);
                index = Some(i);
            } else if offset > buffer.offset {
                buffer.offset = offset;
                if !buffer.free.load(Ordering::Relaxed) {
                    index = None;
                }
            } else if found_surface {
                break;
            }
            offset += buffer.size;
        }

        if found_surface {
            offset = self.buffer_list[index?].offset;
        }

        let buffer = self.inner.create_buffer(
            offset as i32,
            width,
            height,
            stride,
            format, udata, conn,
            qh
        ).ok()?;

        if found_surface {
            self.buffer_list[index?].buffer = buffer.clone();
        } else if index.is_none() {
            let buffer = Buffer {
                offset,
                free: AtomicBool::new(true),
                buffer: buffer.clone(),
                size: size as usize,
                surface: surface.clone()
            };
            index = Some(self.buffer_list.len());
            self.buffer_list.push(buffer);
        }

        let slice = &mut self.inner.mmap()[offset..][..size as usize];

        self.buffer_list[index?].free.swap(false, Ordering::Relaxed);
        self.buffer_list[index?].size = size as usize;

        Some((buffer, slice))
    }
}

impl DelegateDispatchBase<wl_buffer::WlBuffer> for MultiPool {
    type UserData = ();
}

impl<D> DelegateDispatch<wl_buffer::WlBuffer, D> for MultiPool
where
    D: Dispatch<wl_buffer::WlBuffer, UserData = Self::UserData>,
    D: AsMut<MultiPool>
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

