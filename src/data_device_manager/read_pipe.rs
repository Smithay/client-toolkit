use std::{
    fs, io,
    os::unix::io::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
};

/// If the `calloop` cargo feature is enabled, this can be used
/// as an `EventSource` in a calloop event loop.
#[must_use]
#[derive(Debug)]
pub struct ReadPipe {
    #[cfg(feature = "calloop")]
    file: calloop::generic::Generic<fs::File>,
    #[cfg(not(feature = "calloop"))]
    file: fs::File,
}

#[cfg(feature = "calloop")]
impl io::Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe { self.file.get_mut().read(buf) }
    }
}

#[cfg(not(feature = "calloop"))]
impl io::Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

#[cfg(feature = "calloop")]
impl FromRawFd for ReadPipe {
    unsafe fn from_raw_fd(fd: RawFd) -> ReadPipe {
        ReadPipe {
            file: calloop::generic::Generic::new(
                unsafe { FromRawFd::from_raw_fd(fd) },
                calloop::Interest::READ,
                calloop::Mode::Level,
            ),
        }
    }
}

#[cfg(feature = "calloop")]
impl From<OwnedFd> for ReadPipe {
    fn from(owned: OwnedFd) -> Self {
        ReadPipe {
            file: calloop::generic::Generic::new(
                owned.into(),
                calloop::Interest::READ,
                calloop::Mode::Level,
            ),
        }
    }
}

#[cfg(not(feature = "calloop"))]
impl FromRawFd for ReadPipe {
    unsafe fn from_raw_fd(fd: RawFd) -> ReadPipe {
        ReadPipe { file: unsafe { FromRawFd::from_raw_fd(fd) } }
    }
}

#[cfg(not(feature = "calloop"))]
impl From<OwnedFd> for ReadPipe {
    fn from(owned: OwnedFd) -> Self {
        ReadPipe { file: owned.into() }
    }
}

#[cfg(feature = "calloop")]
impl AsRawFd for ReadPipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.get_ref().as_raw_fd()
    }
}

#[cfg(feature = "calloop")]
impl AsFd for ReadPipe {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.get_ref().as_fd()
    }
}

#[cfg(not(feature = "calloop"))]
impl AsRawFd for ReadPipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}
#[cfg(not(feature = "calloop"))]

impl AsFd for ReadPipe {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

#[cfg(feature = "calloop")]
impl IntoRawFd for ReadPipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.unwrap().as_raw_fd()
    }
}

#[cfg(feature = "calloop")]
impl From<ReadPipe> for OwnedFd {
    fn from(read_pipe: ReadPipe) -> Self {
        read_pipe.file.unwrap().into()
    }
}

#[cfg(not(feature = "calloop"))]
impl IntoRawFd for ReadPipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.into_raw_fd()
    }
}

#[cfg(not(feature = "calloop"))]
impl From<ReadPipe> for OwnedFd {
    fn from(read_pipe: ReadPipe) -> Self {
        read_pipe.file.into()
    }
}

#[cfg(feature = "calloop")]
impl calloop::EventSource for ReadPipe {
    type Event = ();
    type Error = std::io::Error;
    type Metadata = calloop::generic::NoIoDrop<fs::File>;
    type Ret = calloop::PostAction;

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> std::io::Result<calloop::PostAction>
    where
        F: FnMut((), &mut calloop::generic::NoIoDrop<fs::File>) -> Self::Ret,
    {
        self.file.process_events(readiness, token, |_, file| Ok(callback((), file)))
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
