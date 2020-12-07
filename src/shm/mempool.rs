use std::{
    cell::RefCell,
    ffi::CStr,
    fs::File,
    io,
    os::unix::io::{FromRawFd, RawFd},
    rc::Rc,
    time::SystemTime,
    time::UNIX_EPOCH,
};

#[cfg(target_os = "linux")]
use nix::sys::memfd;
use nix::{
    errno::Errno,
    fcntl,
    sys::{mman, stat},
    unistd,
};

use memmap2::MmapMut;

use wayland_client::{
    protocol::{wl_buffer, wl_shm, wl_shm_pool},
    Attached, Main,
};

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
    free: Rc<RefCell<bool>>,
}

impl DoubleMemPool {
    /// Create a double memory pool
    pub fn new<F>(shm: Attached<wl_shm::WlShm>, callback: F) -> io::Result<DoubleMemPool>
    where
        F: FnMut(wayland_client::DispatchData) + 'static,
    {
        let free = Rc::new(RefCell::new(true));
        let callback = Rc::new(RefCell::new(callback));
        let my_free = free.clone();
        let my_callback = callback.clone();
        let pool1 = MemPool::new(shm.clone(), move |ddata| {
            let signal = {
                let mut my_free = my_free.borrow_mut();
                if !*my_free {
                    *my_free = true;
                    true
                } else {
                    false
                }
            };
            if signal {
                (&mut *my_callback.borrow_mut())(ddata);
            }
        })?;
        let my_free = free.clone();
        let pool2 = MemPool::new(shm, move |ddata| {
            let signal = {
                let mut my_free = my_free.borrow_mut();
                if !*my_free {
                    *my_free = true;
                    true
                } else {
                    false
                }
            };
            if signal {
                (&mut *callback.borrow_mut())(ddata);
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
            *self.free.borrow_mut() = false;
            None
        }
    }
}

/// A wrapper handling an SHM memory pool backed by a shared memory file
///
/// This wrapper handles for you the creation of the shared memory file and its synchronization
/// with the protocol.
///
/// Mempool internally tracks the release of the buffers by the compositor. As such, creating a buffer
/// that is not commited to a surface (and then never released by the server) would cause the Mempool
/// to be stuck believing it is still in use.
///
/// Mempool will also handle the destruction of buffers and as such the `destroy()` method should not
/// be used on buffers created from Mempool.
///
/// Overwriting the contents of the memory pool before it is completely freed may cause graphical
/// glitches due to the possible corruption of data while the compositor is reading it.
///
/// Mempool requires a callback that will be called when the pool becomes free, this
/// happens when all the pools buffers are released by the server.
pub struct MemPool {
    file: File,
    len: usize,
    pool: Main<wl_shm_pool::WlShmPool>,
    buffer_count: Rc<RefCell<u32>>,
    mmap: MmapMut,
    callback: Rc<RefCell<dyn FnMut(wayland_client::DispatchData)>>,
}

impl MemPool {
    /// Create a new memory pool associated with given shm
    pub fn new<F>(shm: Attached<wl_shm::WlShm>, callback: F) -> io::Result<MemPool>
    where
        F: FnMut(wayland_client::DispatchData) + 'static,
    {
        let mem_fd = create_shm_fd()?;
        let mem_file = unsafe { File::from_raw_fd(mem_fd) };
        mem_file.set_len(128)?;

        let pool = shm.create_pool(mem_fd, 128);

        let mmap = unsafe { MmapMut::map_mut(&mem_file).unwrap() };

        Ok(MemPool {
            file: mem_file,
            len: 128,
            pool,
            buffer_count: Rc::new(RefCell::new(0)),
            mmap,
            callback: Rc::new(RefCell::new(callback)),
        })
    }

    /// Resize the memory pool
    ///
    /// This affect the size as it is seen by the wayland server. Even
    /// if you extend the temporary file size by writing to it, you need to
    /// call this method otherwise the server won't see the new size.
    ///
    /// Memory pools can only be extented, as such this method will do nothing
    /// if the requested new size is smaller than the current size.
    ///
    /// This method allows you to ensure the underlying pool is large enough to
    /// hold what you want to write to it.
    pub fn resize(&mut self, newsize: usize) -> io::Result<()> {
        if newsize > self.len {
            self.file.set_len(newsize as u64)?;
            self.pool.resize(newsize as i32);
            self.len = newsize;
            self.mmap = unsafe { MmapMut::map_mut(&self.file).unwrap() };
        }
        Ok(())
    }

    /// Create a new buffer to this pool
    ///
    /// The parameters are:
    ///
    /// - `offset`: the offset (in bytes) from the beginning of the pool at which this
    ///   buffer starts
    /// - `width`: the width of this buffer (in pixels)
    /// - `height`: the height of this buffer (in pixels)
    /// - `stride`: distance (in bytes) between the beginning of a row and the next one
    /// - `format`: the encoding format of the pixels. Using a format that was not
    ///   advertised to the `wl_shm` global by the server is a protocol error and will
    ///   terminate your connection
    pub fn buffer(
        &self,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
    ) -> wl_buffer::WlBuffer {
        *self.buffer_count.borrow_mut() += 1;
        let my_buffer_count = self.buffer_count.clone();
        let my_callback = self.callback.clone();
        let buffer = self.pool.create_buffer(offset, width, height, stride, format);
        buffer.quick_assign(move |buffer, event, dispatch_data| match event {
            wl_buffer::Event::Release => {
                buffer.destroy();
                let new_count = {
                    // borrow the buffer_count for as short as possible, in case
                    // the user wants to create a new buffer from the callback
                    let mut my_buffer_count = my_buffer_count.borrow_mut();
                    *my_buffer_count -= 1;
                    *my_buffer_count
                };
                if new_count == 0 {
                    (&mut *my_callback.borrow_mut())(dispatch_data);
                }
            }
            _ => unreachable!(),
        });
        (*buffer).clone().detach()
    }

    /// Uses the memmap2 crate to map the underlying shared memory file
    pub fn mmap(&mut self) -> &mut MmapMut {
        &mut self.mmap
    }

    /// Returns true if the pool contains buffers that are currently in use by the server
    pub fn is_used(&self) -> bool {
        *self.buffer_count.borrow() != 0
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
    // Only try memfd on linux
    #[cfg(target_os = "linux")]
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
    let sys_time = SystemTime::now();
    let mut mem_file_handle = format!(
        "/smithay-client-toolkit-{}",
        sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
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
                    sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
                );
                continue;
            }
            Err(nix::Error::Sys(Errno::EINTR)) => continue,
            Err(nix::Error::Sys(errno)) => return Err(io::Error::from(errno)),
            Err(err) => unreachable!(err),
        }
    }
}

impl<E> crate::environment::Environment<E>
where
    E: crate::environment::GlobalHandler<wl_shm::WlShm>,
{
    /// Create a simple memory pool
    ///
    /// This memory pool track the usage of the buffers created from it,
    /// and invokes your callback when the compositor has finished using
    /// all of them.
    pub fn create_simple_pool<F>(&self, callback: F) -> io::Result<MemPool>
    where
        F: FnMut(wayland_client::DispatchData) + 'static,
    {
        MemPool::new(self.require_global::<wl_shm::WlShm>(), callback)
    }

    /// Create a double memory pool
    ///
    /// This can be used for double-buffered drawing. The memory pool
    /// is backed by two different SHM segments, which are used in alternance.
    ///
    /// The provided callback is triggered when one of the pools becomes unused again
    /// after you tried to draw while both where in use.
    pub fn create_double_pool<F>(&self, callback: F) -> io::Result<DoubleMemPool>
    where
        F: FnMut(wayland_client::DispatchData) + 'static,
    {
        DoubleMemPool::new(self.require_global::<wl_shm::WlShm>(), callback)
    }
}
