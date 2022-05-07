pub mod raw;

use std::io;

use nix::errno::Errno;

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
}

impl From<Errno> for CreatePoolError {
    fn from(errno: Errno) -> Self {
        Into::<io::Error>::into(errno).into()
    }
}
