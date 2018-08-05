use nix;
use nix::errno::Errno;
use nix::fcntl;
use nix::sys::memfd;
use nix::sys::mman;
use nix::sys::stat;
use nix::unistd;
use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd;

use rand::prelude::*;

use wayland_client::protocol::{wl_buffer, wl_shm, wl_shm_pool};
use wayland_client::{NewProxy, Proxy};

use wayland_client::protocol::wl_shm::RequestsTrait as ShmRequests;
use wayland_client::protocol::wl_shm_pool::RequestsTrait as PoolRequests;

/// A Double memory pool, for convenient double-buffering
///
/// This type wraps two internal memory pool, and can be
/// use for conveniently implementing double-buffering in your
/// apps.
///
/// Just access the current drawing pool with the `pool()` method,
/// and swap them using the `swap()` method between two frames.
pub struct DoubleMemPool {
    pool1: MemPool,
    pool2: MemPool,
}

impl DoubleMemPool {
    /// Create a double memory pool
    pub fn new(shm: &Proxy<wl_shm::WlShm>) -> io::Result<DoubleMemPool> {
        let pool1 = MemPool::new(shm)?;
        let pool2 = MemPool::new(shm)?;
        Ok(DoubleMemPool { pool1, pool2 })
    }

    /// Access the current drawing pool
    pub fn pool(&mut self) -> &mut MemPool {
        &mut self.pool1
    }

    /// Swap the pool
    pub fn swap(&mut self) {
        ::std::mem::swap(&mut self.pool1, &mut self.pool2);
    }
}

/// A wrapper handling an SHM memory pool backed by a shared memory file
///
/// This wrapper handles for you the creation of the shared memory file and its synchronisation
/// with the protocol.
pub struct MemPool {
    file: File,
    len: usize,
    pool: Proxy<wl_shm_pool::WlShmPool>,
}

impl MemPool {
    /// Create a new memory pool associated with given shm
    pub fn new(shm: &Proxy<wl_shm::WlShm>) -> io::Result<MemPool> {
        let mem_fd = create_shm_fd()?;
        let mem_file = unsafe { File::from_raw_fd(mem_fd) };
        mem_file.set_len(128)?;

        let pool = shm
            .create_pool(mem_fd, 128)
            .unwrap()
            .implement(|e, _| match e {});

        Ok(MemPool {
            file: mem_file,
            len: 128,
            pool: pool,
        })
    }

    /// Resize the memory pool
    ///
    /// This affect the size as it is seen by the wayland server. Even
    /// if you extend the temporary file size by writing to it, you need to
    /// call this method otherwise the server won't see the new size.
    ///
    /// Memory pools can only be extented, as such this method wll do nothing
    /// if the requested new size is smaller than the current size.
    ///
    /// This method allows you to ensure the underlying pool is large enough to
    /// hold what you want to write to it.
    pub fn resize(&mut self, newsize: usize) -> io::Result<()> {
        if newsize > self.len {
            self.file.set_len(newsize as u64)?;
            self.pool.resize(newsize as i32);
            self.len = newsize;
        }
        Ok(())
    }

    /// Create a new buffer to this pool
    ///
    /// The parameters are:
    ///
    /// - `offset`: the offset (in bytes) from the beggining of the pool at which this
    ///   buffer starts
    /// - `width`: the width of this buffer (in pixels)
    /// - `height`: the height of this buffer (in pixels)
    /// - `stride`: distance (in bytes) between the beggining of a row and the next one
    /// - `format`: the encoding format of the pixels. Using a format that was not
    ///   advertized to the `wl_shm` global by the server is a protocol error and will
    ///   terminate your connexion
    pub fn buffer(
        &self,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
    ) -> NewProxy<wl_buffer::WlBuffer> {
        self.pool
            .create_buffer(offset, width, height, stride, format)
            .unwrap()
    }
}

impl Drop for MemPool {
    fn drop(&mut self) {
        self.pool.destroy();
    }
}

impl io::Write for MemPool {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(&mut self.file, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.file)
    }
}

impl io::Seek for MemPool {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        io::Seek::seek(&mut self.file, pos)
    }
}

fn create_shm_fd() -> io::Result<RawFd> {
    loop {
        match memfd::memfd_create(
            CStr::from_bytes_with_nul(b"smithay-client-toolkit\0").unwrap(),
            memfd::MemFdCreateFlag::MFD_CLOEXEC,
        ) {
            Ok(fd) => return Ok(fd),
            Err(nix::Error::Sys(Errno::EINTR)) => continue,
            Err(nix::Error::Sys(errno)) => return Err(io::Error::from(errno)),
            Err(_) => break,
        }
    }

    // Fallback to using shm_open
    let mut rng = thread_rng();
    let mut mem_file_handle = format!(
        "/smithay-client-toolkit-{}",
        rng.gen_range(0, ::std::u32::MAX)
    );
    loop {
        match mman::shm_open(
            mem_file_handle.as_str(),
            fcntl::OFlag::O_CREAT
                | fcntl::OFlag::O_EXCL
                | fcntl::OFlag::O_RDWR
                | fcntl::OFlag::O_CLOEXEC,
            stat::Mode::S_IRUSR | stat::Mode::S_IWUSR,
        ) {
            Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(nix::Error::Sys(errno)) => match unistd::close(fd) {
                    Ok(_) => return Err(io::Error::from(errno)),
                    Err(nix::Error::Sys(errno)) => return Err(io::Error::from(errno)),
                    Err(err) => panic!(err),
                },
                Err(err) => panic!(err),
            },
            Err(nix::Error::Sys(Errno::EEXIST)) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = format!(
                    "/smithay-client-toolkit-{}",
                    rng.gen_range(0, ::std::u32::MAX)
                );
                continue;
            }
            Err(nix::Error::Sys(Errno::EINTR)) => continue,
            Err(nix::Error::Sys(errno)) => return Err(io::Error::from(errno)),
            Err(err) => panic!(err),
        }
    }
}
