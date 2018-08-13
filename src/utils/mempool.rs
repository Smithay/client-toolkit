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
use std::sync::{Arc, Mutex};

use rand::prelude::*;

use wayland_client::commons::Implementation;
use wayland_client::protocol::{wl_buffer, wl_shm, wl_shm_pool};
use wayland_client::Proxy;

use wayland_client::protocol::wl_buffer::RequestsTrait;
use wayland_client::protocol::wl_shm::RequestsTrait as ShmRequests;
use wayland_client::protocol::wl_shm_pool::RequestsTrait as PoolRequests;

/// A Double memory pool, for convenient double-buffering
///
/// This type wraps two internal memory pool, and can be
/// use for conveniently implementing double-buffering in your
/// apps.
///
/// DoubleMemPool requires a implementation that is called when
/// one of the two internal memory pools becomes free after None
/// was returned from the `pool()` method.
pub struct DoubleMemPool {
    pool1: MemPool,
    pool2: MemPool,
    free: Arc<Mutex<bool>>,
}

impl DoubleMemPool {
    /// Create a double memory pool
    pub fn new<Impl>(shm: &Proxy<wl_shm::WlShm>, implementation: Impl) -> io::Result<DoubleMemPool>
    where
        Impl: Implementation<(), ()> + Send,
    {
        let free = Arc::new(Mutex::new(true));
        let implementation = Arc::new(Mutex::new(implementation));
        let my_free = free.clone();
        let my_implementation = implementation.clone();
        let pool1 = MemPool::new(shm, move |_, _| {
            let mut my_free = my_free.lock().unwrap();
            if !*my_free {
                my_implementation.lock().unwrap().receive((), ());
                *my_free = true
            }
        })?;
        let my_free = free.clone();
        let my_implementation = implementation.clone();
        let pool2 = MemPool::new(shm, move |_, _| {
            let mut my_free = my_free.lock().unwrap();
            if !*my_free {
                my_implementation.lock().unwrap().receive((), ());
                *my_free = true
            }
        })?;
        Ok(DoubleMemPool { pool1, pool2, free })
    }

    /// This method checks both its internal memory pools and returns
    /// one if that pool does not contain any buffers that are still in use
    /// by the server. If both the memory pools contain buffers that are currently
    /// in use by the server None will be returned.
    pub fn pool(&mut self) -> Option<&mut MemPool> {
        if !self.pool1.is_used() {
            Some(&mut self.pool1)
        } else if !self.pool2.is_used() {
            Some(&mut self.pool2)
        } else {
            *self.free.lock().unwrap() = false;
            None
        }
    }
}

/// A wrapper handling an SHM memory pool backed by a shared memory file
///
/// This wrapper handles for you the creation of the shared memory file and its synchronisation
/// with the protocol.
///
/// Mempool internally tracks the lifetime of all buffers created from it and to ensure that
/// this buffer count is correct all buffers must be attached to a surface. Once a buffer is attached to
/// a surface it must be immediately commited to that surface before another buffer is attached.
///
/// Mempool will also handle the destruction of buffers and as such the `destroy()` method should not
/// be used on buffers created from Mempool.
///
/// Overwriting the contents of the memory pool before it is completely freed may cause graphical
/// glitches due to the possible corruption of data while the compositor is reading it.
///
/// Mempool requires an implementation that will be called when the pool becomes free, this
/// happens when all the pools buffers are released by the server.
pub struct MemPool {
    file: File,
    len: usize,
    pool: Proxy<wl_shm_pool::WlShmPool>,
    buffer_count: Arc<Mutex<u32>>,
    implementation: Arc<Mutex<Implementation<(), ()> + Send>>,
}

impl MemPool {
    /// Create a new memory pool associated with given shm
    pub fn new<Impl>(shm: &Proxy<wl_shm::WlShm>, implementation: Impl) -> io::Result<MemPool>
    where
        Impl: Implementation<(), ()> + Send,
    {
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
            pool,
            buffer_count: Arc::new(Mutex::new(0)),
            implementation: Arc::new(Mutex::new(implementation)),
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
    ) -> Proxy<wl_buffer::WlBuffer> {
        *self.buffer_count.lock().unwrap() += 1;
        let my_buffer_count = self.buffer_count.clone();
        let my_implementation = self.implementation.clone();
        self.pool
            .create_buffer(offset, width, height, stride, format)
            .unwrap()
            .implement(
                move |event, buffer: Proxy<wl_buffer::WlBuffer>| match event {
                    wl_buffer::Event::Release => {
                        buffer.destroy();
                        let mut my_buffer_count = my_buffer_count.lock().unwrap();
                        *my_buffer_count -= 1;
                        if *my_buffer_count == 0 {
                            my_implementation.lock().unwrap().receive((), ());
                        }
                    }
                },
            )
    }

    /// Retuns true if the pool contains buffers that are currently in use by the server otherwise it returns
    /// false
    pub fn is_used(&self) -> bool {
        *self.buffer_count.lock().unwrap() != 0
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
            Err(nix::Error::Sys(Errno::ENOSYS)) => break,
            Err(nix::Error::Sys(errno)) => return Err(io::Error::from(errno)),
            Err(err) => unreachable!(err),
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
            Err(err) => unreachable!(err),
        }
    }
}
