use std::{
    os::unix::io::{AsFd, OwnedFd},
    sync::Mutex,
};

use crate::reexports::client::{Connection, Dispatch, QueueHandle, Proxy};
use crate::reexports::protocols::wp::primary_selection::zv1::client::zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1;

use crate::data_device_manager::ReadPipe;

use super::PrimarySelectionManagerState;

/// Wrapper around the [`ZwpPrimarySelectionOfferV1`].
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
        use rustix::pipe::{pipe_with, PipeFlags};
        // create a pipe
        let (readfd, writefd) = pipe_with(PipeFlags::CLOEXEC)?;

        self.receive_to_fd(mime_type, writefd);

        Ok(ReadPipe::from(readfd))
    }

    /// Request to receive the data of a given mime type, writen to `writefd`.
    ///
    /// The provided file destructor must be a valid FD for writing, and will be closed
    /// once the contents are written.
    pub fn receive_to_fd(&self, mime_type: String, writefd: OwnedFd) {
        self.offer.receive(mime_type, writefd.as_fd());
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
