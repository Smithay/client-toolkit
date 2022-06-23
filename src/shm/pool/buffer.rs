//! A pool implementation which manages a single, fixed size buffer.
//!
//! Most clients should use something other than a [`Buffer`]. A [`Buffer`] is useful for clients which need
//! maximum control over buffer allocations or simple clients which only need to display a single, mostly
//! static buffer without worrying about race conditions.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use wayland_backend::{
    client::{Backend, InvalidId, ObjectData, ObjectId},
    protocol::Message,
};
use wayland_client::{
    protocol::{wl_buffer, wl_shm, wl_surface},
    Connection, Proxy,
};

use super::{raw::RawPool, CreatePoolError};

#[derive(Debug, thiserror::Error)]
pub enum ActivateError {
    /// Buffer was already active
    #[error("Buffer was already active")]
    AlreadyActive,

    /// Protocol error.
    #[error(transparent)]
    Protocol(#[from] InvalidId),
}

/// A fixed size SHM buffer.
#[derive(Debug)]
pub struct Buffer {
    wl_buffer: wl_buffer::WlBuffer,
    pool: RawPool,
    width: i32,
    height: i32,
}

impl Buffer {
    /// Get the bytes corresponding to this buffer if drawing is permitted.
    ///
    /// Returns [`None`] if the buffer is in use.
    pub fn canvas(&mut self) -> Option<&mut [u8]> {
        // Ensure the buffer is not in use.
        if !self.data()?.free.load(Ordering::Relaxed) {
            return None;
        }

        let len = self.len();

        Some(&mut self.pool.mmap()[0..][..len])
    }

    /// Attach this buffer to a surface.
    ///
    /// This marks the byffer as active until the server releases the buffer, which will happen
    /// automatically assuming the surface is committed without attaching a different buffer.
    pub fn attach_to(&self, surface: &wl_surface::WlSurface) -> Result<(), ActivateError> {
        self.activate()?;
        surface.attach(Some(&self.wl_buffer), 0, 0);
        Ok(())
    }

    /// Manually mark this buffer as active.
    ///
    /// An active buffer prevents drawing until a Release event is received.
    pub fn activate(&self) -> Result<(), ActivateError> {
        let data = self.data().ok_or(InvalidId)?;

        if !data.free.load(Ordering::Relaxed) {
            return Err(ActivateError::AlreadyActive);
        }

        // Mark the buffer as in use.
        data.free.store(false, Ordering::Relaxed);
        Ok(())
    }

    /// Returns the underlying [`WlBuffer`](wl_buffer::WlBuffer).
    pub fn wl_buffer(&self) -> &wl_buffer::WlBuffer {
        &self.wl_buffer
    }

    /// Returns the width of the buffer.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Returns the height of the buffer.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Returns the size, in bytes, of this pool.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.pool.len()
    }
}

impl Buffer {
    pub(crate) fn new(
        mut pool: RawPool,
        width: i32,
        stride: i32,
        height: i32,
        format: wl_shm::Format,
    ) -> Result<Self, CreatePoolError> {
        let data = BufferData { free: AtomicBool::new(true), dropped: AtomicBool::new(false) };

        let wl_buffer = pool.create_buffer_raw(0, width, height, stride, format, Arc::new(data))?;

        Ok(Self { wl_buffer, pool, width, height })
    }

    fn data(&self) -> Option<&BufferData> {
        self.wl_buffer.object_data()?.downcast_ref()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if let Some(data) = self.data() {
            data.dropped.store(true, Ordering::Relaxed);

            // Destroy the buffer if not in use
            if data.free.load(Ordering::Relaxed) {
                self.wl_buffer.destroy();
            }
        }
    }
}

struct BufferData {
    free: AtomicBool,
    dropped: AtomicBool,
}

impl ObjectData for BufferData {
    fn event(
        self: Arc<Self>,
        backend: &Backend,
        msg: Message<ObjectId>,
    ) -> Option<Arc<dyn ObjectData>> {
        debug_assert!(wayland_client::backend::protocol::same_interface(
            msg.sender_id.interface(),
            wl_buffer::WlBuffer::interface()
        ));
        debug_assert!(msg.opcode == 0);

        self.free.store(true, Ordering::Relaxed);

        // Check if the Buffer was dropped.
        if self.dropped.load(Ordering::Relaxed) {
            if let Ok(wl_buffer) = wl_buffer::WlBuffer::from_id(
                &Connection::from_backend(backend.clone()),
                msg.sender_id,
            ) {
                wl_buffer.destroy();
            } else {
                log::error!(target: "sctk", "Underlying WlBuffer was destroyed before drop")
            }
        }

        None
    }

    fn destroyed(&self, _object_id: ObjectId) {}
}
