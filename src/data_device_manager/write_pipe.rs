use std::{
    fs, io,
    os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
};
use wayland_backend::io_lifetimes::{AsFd, OwnedFd};

/// If the `calloop` cargo feature is enabled, this can be used
/// as an `EventSource` in a calloop event loop.
#[must_use]
#[derive(Debug)]
pub struct WritePipe {
    #[cfg(feature = "calloop")]
    file: calloop::generic::Generic<fs::File>,
    #[cfg(not(feature = "calloop"))]
    file: fs::File,
}

#[cfg(feature = "calloop")]
impl io::Write for WritePipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.file.flush()
    }
}

#[cfg(not(feature = "calloop"))]
impl io::Write for WritePipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[cfg(feature = "calloop")]
impl FromRawFd for WritePipe {
    unsafe fn from_raw_fd(fd: RawFd) -> WritePipe {
        WritePipe {
            file: calloop::generic::Generic::new(
                unsafe { FromRawFd::from_raw_fd(fd) },
                calloop::Interest::WRITE,
                calloop::Mode::Level,
            ),
        }
    }
}

#[cfg(feature = "calloop")]
impl From<OwnedFd> for WritePipe {
    fn from(owned: OwnedFd) -> Self {
        WritePipe {
            file: calloop::generic::Generic::new(
                owned.into(),
                calloop::Interest::WRITE,
                calloop::Mode::Level,
            ),
        }
    }
}

#[cfg(not(feature = "calloop"))]
impl FromRawFd for WritePipe {
    unsafe fn from_raw_fd(fd: RawFd) -> WritePipe {
        WritePipe { file: unsafe { FromRawFd::from_raw_fd(fd) } }
    }
}

#[cfg(not(feature = "calloop"))]
impl From<OwnedFd> for WritePipe {
    fn from(owned: OwnedFd) -> Self {
        WritePipe { file: owned.into() }
    }
}

#[cfg(feature = "calloop")]
impl AsRawFd for WritePipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.file.as_raw_fd()
    }
}

#[cfg(feature = "calloop")]
impl AsFd for WritePipe {
    fn as_fd(&self) -> wayland_backend::io_lifetimes::BorrowedFd<'_> {
        self.file.file.as_fd()
    }
}

#[cfg(not(feature = "calloop"))]
impl AsRawFd for WritePipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}
#[cfg(not(feature = "calloop"))]

impl AsFd for WritePipe {
    fn as_fd(&self) -> wayland_backend::io_lifetimes::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

#[cfg(feature = "calloop")]
impl IntoRawFd for WritePipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.file.into_raw_fd()
    }
}

#[cfg(feature = "calloop")]
impl From<WritePipe> for OwnedFd {
    fn from(write_pipe: WritePipe) -> Self {
        write_pipe.file.file.into()
    }
}

#[cfg(not(feature = "calloop"))]
impl IntoRawFd for WritePipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.into_raw_fd()
    }
}

#[cfg(not(feature = "calloop"))]
impl From<WritePipe> for OwnedFd {
    fn from(write_pipe: WritePipe) -> Self {
        write_pipe.file.into()
    }
}

#[cfg(feature = "calloop")]
impl calloop::EventSource for WritePipe {
    type Event = ();
    type Error = std::io::Error;
    type Metadata = fs::File;
    type Ret = ();

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> std::io::Result<calloop::PostAction>
    where
        F: FnMut((), &mut fs::File),
    {
        self.file.process_events(readiness, token, |_, file| {
            callback((), file);
            Ok(calloop::PostAction::Continue)
        })
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.file.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.file.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> calloop::Result<()> {
        self.file.unregister(poll)
    }
}
