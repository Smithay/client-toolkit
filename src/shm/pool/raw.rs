//! A raw shared memory pool handler.
//!
//! This is intended as a safe building block for higher level shared memory pool abstractions and is not
//! encouraged for most library users.

use std::{
    fs::File,
    io,
    os::unix::prelude::{FromRawFd, RawFd},
    time::{SystemTime, UNIX_EPOCH},
};

use memmap2::MmapMut;
use nix::{
    errno::Errno,
    fcntl,
    sys::{mman, stat},
    unistd,
};
use wayland_client::{
    backend::InvalidId,
    protocol::{wl_buffer, wl_shm, wl_shm_pool},
    ConnectionHandle, Dispatch, QueueHandle,
};

use super::CreatePoolError;

/// A raw handler for file backed shared memory pools.
///
/// This type of pool will create the SHM memory pool and provide a way to resize the pool.
///
/// This pool does not release buffers. If you need this, use one of the higher level pools.
#[derive(Debug)]
pub struct RawPool {
    pool: wl_shm_pool::WlShmPool,
    pub(crate) len: usize,
    mem_file: File,
    mmap: MmapMut,
}

impl RawPool {
    /// Resizes the memory pool, notifying the server the pool has changed in size.
    ///
    /// The wl_shm protocol only allows the pool to be made bigger. If the new size is smaller than the
    /// current size of the pool, this function will do nothing.
    pub fn resize(&mut self, size: usize, conn: &mut ConnectionHandle) -> io::Result<()> {
        if size > self.len {
            self.len = size;
            self.mem_file.set_len(size as u64)?;
            self.pool.resize(conn, size as i32);
            self.mmap = unsafe { MmapMut::map_mut(&self.mem_file) }?;
        }

        Ok(())
    }

    /// Returns a reference to the underlying shared memory file using the memmap2 crate.
    pub fn mmap(&mut self) -> &mut MmapMut {
        &mut self.mmap
    }

    /// Create a new buffer to this pool.
    ///
    /// ## Parameters
    /// - `offset`: the offset (in bytes) from the beginning of the pool at which this buffer starts.
    /// - `width` and `height`: the width and height of the buffer in pixels.
    /// - `stride`: distance (in bytes) between the beginning of a row and the next one.
    /// - `format`: the encoding format of the pixels.
    ///
    /// The encoding format of the pixels must be supported by the compositor or else a protocol error is
    /// risen. You can ensure the format is supported by listening to [`ShmState::formats`](crate::shm::ShmState::formats).
    ///
    /// Note this function only creates the wl_buffer object, you will need to write to the pixels using the
    /// [`io::Write`] implementation or [`RawPool::mmap`].
    #[allow(clippy::too_many_arguments)]
    pub fn create_buffer<D, U>(
        &mut self,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
        udata: U,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) -> Result<wl_buffer::WlBuffer, InvalidId>
    where
        D: Dispatch<wl_buffer::WlBuffer, UserData = U> + 'static,
        U: Send + Sync + 'static,
    {
        let buffer =
            self.pool.create_buffer(conn, offset, width, height, stride, format, qh, udata)?;

        Ok(buffer)
    }

    /// Returns the pool object used to communicate with the server.
    pub fn pool(&self) -> &wl_shm_pool::WlShmPool {
        &self.pool
    }

    /// Destroys this pool.
    ///
    /// This will not free the memory associated with any created buffers. You will need to destroy any
    /// existing buffers created from the pool to free the memory.
    pub fn destroy(self, conn: &mut ConnectionHandle) {
        self.pool.destroy(conn);
    }
}

impl io::Write for RawPool {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(&mut self.mem_file, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.mem_file)
    }
}

impl io::Seek for RawPool {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        io::Seek::seek(&mut self.mem_file, pos)
    }
}

impl RawPool {
    pub fn new<D, U>(
        len: usize,
        shm: &wl_shm::WlShm,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<RawPool, CreatePoolError>
    where
        D: Dispatch<wl_shm_pool::WlShmPool, UserData = U> + 'static,
        U: Send + Sync + 'static,
    {
        let shm_fd = RawPool::create_shm_fd()?;
        let mem_file = unsafe { File::from_raw_fd(shm_fd) };
        mem_file.set_len(len as u64)?;

        let pool = shm.create_pool(conn, shm_fd, len as i32, qh, udata)?;
        let mmap = unsafe { MmapMut::map_mut(&mem_file)? };

        Ok(RawPool { pool, len, mem_file, mmap })
    }

    fn create_shm_fd() -> io::Result<RawFd> {
        #[cfg(target_os = "linux")]
        {
            match RawPool::create_memfd() {
                Ok(fd) => return Ok(fd),

                // Not supported, use fallback.
                Err(Errno::ENOSYS) => (),

                Err(err) => return Err(Into::<io::Error>::into(err)),
            };
        }

        let time = SystemTime::now();
        let mut mem_file_handle = format!(
            "/smithay-client-toolkit-{}",
            time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
        );

        loop {
            let flags = fcntl::OFlag::O_CREAT
                | fcntl::OFlag::O_EXCL
                | fcntl::OFlag::O_RDWR
                | fcntl::OFlag::O_CLOEXEC;

            let mode = stat::Mode::S_IRUSR | stat::Mode::S_IWUSR;

            match mman::shm_open(mem_file_handle.as_str(), flags, mode) {
                Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                    Ok(_) => return Ok(fd),

                    Err(errno) => {
                        unistd::close(fd)?;

                        return Err(errno.into());
                    }
                },

                Err(Errno::EEXIST) => {
                    // Change the handle if we happen to be duplicate.
                    let time = SystemTime::now();

                    mem_file_handle = format!(
                        "/smithay-client-toolkit-{}",
                        time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
                    );

                    continue;
                }

                Err(Errno::EINTR) => continue,

                Err(err) => return Err(err.into()),
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn create_memfd() -> nix::Result<RawFd> {
        use std::ffi::CStr;

        use nix::{
            fcntl::{FcntlArg, SealFlag},
            sys::memfd::{self, MemFdCreateFlag},
        };

        loop {
            let name = CStr::from_bytes_with_nul(b"smithay-client-toolkit\0").unwrap();
            let flags = MemFdCreateFlag::MFD_ALLOW_SEALING | MemFdCreateFlag::MFD_CLOEXEC;

            match memfd::memfd_create(name, flags) {
                Ok(fd) => {
                    let arg =
                        FcntlArg::F_ADD_SEALS(SealFlag::F_SEAL_SHRINK | SealFlag::F_SEAL_SEAL);

                    // We only need to seal for the purposes of optimization, ignore the errors.
                    let _ = fcntl::fcntl(fd, arg);

                    return Ok(fd);
                }

                Err(Errno::EINTR) => continue,

                Err(err) => return Err(err),
            }
        }
    }
}
