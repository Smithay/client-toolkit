//! A pool implementation which automatically frees buffers when released.

use std::io;

use wayland_client::ConnectionHandle;

use super::raw::RawPool;

#[derive(Debug)]
pub struct SimplePool {
    pub(crate) inner: RawPool,
}

impl SimplePool {
    /// Resizes the memory pool, notifying the server the pool has changed in size.
    ///
    /// The wl_shm protocol only allows the pool to be made bigger. If the new size is smaller than the
    /// current size of the pool, this function will do nothing.
    pub fn resize(&mut self, size: usize, conn: &mut ConnectionHandle) -> io::Result<()> {
        self.inner.resize(size, conn)
    }
}
