pub mod buffer;
pub mod multi;
pub mod raw;
pub mod slot;

use std::io;

use nix::errno::Errno;
use wayland_backend::client::InvalidId;

use crate::error::GlobalError;

/// An error that may occur when creating a pool.
#[derive(Debug, thiserror::Error)]
pub enum CreatePoolError {
    /// The wl_shm global is not bound.
    #[error(transparent)]
    Global(#[from] GlobalError),

    /// Error while allocating the shared memory.
    #[error(transparent)]
    Create(#[from] io::Error),

    #[error(transparent)]
    InvalidId(#[from] InvalidId),
}

impl From<Errno> for CreatePoolError {
    fn from(errno: Errno) -> Self {
        Into::<io::Error>::into(errno).into()
    }
}
