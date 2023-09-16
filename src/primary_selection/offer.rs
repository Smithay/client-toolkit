use std::{
    os::unix::io::{BorrowedFd, FromRawFd, RawFd},
    sync::Mutex,
};

use crate::reexports::client::{Connection, Dispatch, QueueHandle, Proxy};
use crate::reexports::protocols::wp::primary_selection::zv1::client::zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1;

use crate::data_device_manager::ReadPipe;

use super::PrimarySelectionManagerState;

/// RAII wrapper around the [`ZwpPrimarySelectionOfferV1`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimarySelectionOffer {
    pub(crate) offer: ZwpPrimarySelectionOfferV1,
}

impl PrimarySelectionOffer {
    /// Inspect the mime types available on the given offer.
    pub fn with_mime_types<T, F: Fn(&[String]) -> T>(&self, callback: F) -> T {
        let mime_types =
            self.offer.data::<PrimarySelectionOfferData>().unwrap().mimes.lock().unwrap();
        callback(mime_types.as_ref())
    }

    /// Request to receive the data of a given mime type.
    ///
    /// You can call this function several times.
    ///
    /// Note that you should *not* read the contents right away in a
    /// blocking way, as you may deadlock your application doing so.
    /// At least make sure you flush your events to the server before
    /// doing so.
    ///
    /// Fails if too many file descriptors were already open and a pipe
    /// could not be created.
    pub fn receive(&self, mime_type: String) -> std::io::Result<ReadPipe> {
        use nix::fcntl::OFlag;
        use nix::unistd::{close, pipe2};
        // create a pipe
        let (readfd, writefd) = pipe2(OFlag::O_CLOEXEC)?;

        self.offer.receive(mime_type, unsafe { BorrowedFd::borrow_raw(writefd) });

        if let Err(err) = close(writefd) {
            log::warn!("Failed to close write pipe: {}", err);
        }

        Ok(unsafe { FromRawFd::from_raw_fd(readfd) })
    }

    /// # Safety
    ///
    /// The provided file destructor must be a valid FD for writing, and will be closed
    /// once the contents are written.
    pub unsafe fn receive_to_fd(&self, mime_type: String, writefd: RawFd) {
        use nix::unistd::close;

        self.offer.receive(mime_type, unsafe { BorrowedFd::borrow_raw(writefd) });

        if let Err(err) = close(writefd) {
            log::warn!("Failed to close write pipe: {}", err);
        }
    }
}

impl Drop for PrimarySelectionOffer {
    fn drop(&mut self) {
        self.offer.destroy();
    }
}

impl<State> Dispatch<ZwpPrimarySelectionOfferV1, PrimarySelectionOfferData, State>
    for PrimarySelectionManagerState
where
    State: Dispatch<ZwpPrimarySelectionOfferV1, PrimarySelectionOfferData>,
{
    fn event(
        _: &mut State,
        _: &ZwpPrimarySelectionOfferV1,
        event: <ZwpPrimarySelectionOfferV1 as wayland_client::Proxy>::Event,
        data: &PrimarySelectionOfferData,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_offer_v1::Event;
        match event {
            Event::Offer { mime_type } => {
                data.mimes.lock().unwrap().push(mime_type);
            }
            _ => unreachable!(),
        }
    }
}

/// The data associated with the [`ZwpPrimarySelectionOfferV1`].
#[derive(Debug, Default)]
pub struct PrimarySelectionOfferData {
    mimes: Mutex<Vec<String>>,
}
