use wayland_client::protocol::wl_data_device_manager::DndAction;
use wayland_client::protocol::wl_data_offer;
use wayland_client::{NewProxy, Proxy};

use wayland_client::protocol::wl_data_offer::RequestsTrait as OfferRequests;

use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::sync::{Arc, Mutex};
use std::{fs, io};

struct Inner {
    mime_types: Vec<String>,
    actions: DndAction,
    current_action: DndAction,
    serial: u32,
}

/// A data offer for receiving data though copy/paste or
/// drag and drop
pub struct DataOffer {
    pub(crate) offer: Proxy<wl_data_offer::WlDataOffer>,
    inner: Arc<Mutex<Inner>>,
}

impl DataOffer {
    pub(crate) fn new(offer: NewProxy<wl_data_offer::WlDataOffer>) -> DataOffer {
        let inner = Arc::new(Mutex::new(Inner {
            mime_types: Vec::new(),
            actions: DndAction::None,
            current_action: DndAction::None,
            serial: 0,
        }));
        let inner2 = inner.clone();
        let offer = offer.implement(move |event, _: Proxy<_>| {
            use self::wl_data_offer::Event;
            let mut inner = inner2.lock().unwrap();
            match event {
                Event::Offer { mime_type } => {
                    inner.mime_types.push(mime_type);
                }
                Event::SourceActions { source_actions } => {
                    inner.actions = DndAction::from_bits_truncate(source_actions);
                }
                Event::Action { dnd_action } => {
                    inner.current_action = DndAction::from_bits_truncate(dnd_action);
                }
            }
        });

        DataOffer { offer, inner }
    }

    /// Access the list of mime types proposed by this offer
    pub fn with_mime_types<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&[String]) -> T,
    {
        let inner = self.inner.lock().unwrap();
        f(&inner.mime_types)
    }

    /// Get the list of available actions for this offer
    pub fn get_available_actions(&self) -> DndAction {
        self.inner.lock().unwrap().actions
    }

    /// Get the currently set final action for this offer
    pub fn get_current_action(&self) -> DndAction {
        self.inner.lock().unwrap().current_action
    }

    /// Accept a mime type for receiving data through this offer
    pub fn accept(&self, mime_type: Option<String>) {
        let serial = self.inner.lock().unwrap().serial;
        self.offer.accept(serial, mime_type);
    }

    /// Request to receive the data of a given mime type
    ///
    /// You can do this several times, as a reaction to motion of
    /// the dnd cursor, or to inspect the data in order to choose your
    /// response.
    ///
    /// Note that you should *not* read the contents right way in a
    /// blocking way, as you may deadlock your application doing so.
    /// At least make sure you flush your events to the server before
    /// doing so.
    ///
    /// Fails if too many file descriptors were already open and a pipe
    /// could not be created.
    pub fn receive(&self, mime_type: String) -> Result<ReadPipe, ()> {
        use nix::fcntl::OFlag;
        use nix::unistd::{close, pipe2};
        // create a pipe
        let (readfd, writefd) = pipe2(OFlag::O_CLOEXEC).map_err(|_| ())?;

        self.offer.receive(mime_type, writefd);
        let _ = close(writefd);

        Ok(unsafe { FromRawFd::from_raw_fd(readfd) })
    }

    /// Notify the send and compositor of the dnd actions you accept
    ///
    /// You need to provide the set of supported actions, as well as
    /// a single preferred action.
    pub fn set_actions(&self, supported: DndAction, preferred: DndAction) {
        self.offer
            .set_actions(supported.to_raw(), preferred.to_raw());
    }

    /// Notify that you are finished with this offer, and will no longer
    /// be using it
    ///
    /// Note that it is a protocol error to finish if no action or mime
    /// type was accepted.
    pub fn finish(&self) {
        self.offer.finish();
        self.offer.destroy();
    }
}

impl Drop for DataOffer {
    fn drop(&mut self) {
        self.offer.destroy();
    }
}

/// A file descriptor that can only be written to
pub struct ReadPipe {
    file: fs::File,
}

impl io::Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl FromRawFd for ReadPipe {
    unsafe fn from_raw_fd(fd: RawFd) -> ReadPipe {
        ReadPipe {
            file: FromRawFd::from_raw_fd(fd),
        }
    }
}

impl AsRawFd for ReadPipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl IntoRawFd for ReadPipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.into_raw_fd()
    }
}
