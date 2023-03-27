use std::os::unix::io::AsRawFd;
use std::{
    ops::{Deref, DerefMut},
    os::unix::prelude::{FromRawFd, RawFd},
    sync::{Arc, Mutex},
};
use wayland_backend::io_lifetimes::BorrowedFd;
use wayland_client::{
    protocol::{
        wl_data_device_manager::DndAction,
        wl_data_offer::{self, WlDataOffer},
        wl_surface::WlSurface,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

use super::{DataDeviceManagerState, ReadPipe};

#[derive(Debug, Clone)]
pub struct UndeterminedOffer {
    pub(crate) data_offer: Option<WlDataOffer>,
}
impl PartialEq for UndeterminedOffer {
    fn eq(&self, other: &Self) -> bool {
        self.data_offer == other.data_offer
    }
}

impl UndeterminedOffer {
    pub fn destroy(&self) {
        if let Some(offer) = self.data_offer.as_ref() {
            offer.destroy();
        }
    }

    pub fn inner(&self) -> Option<&WlDataOffer> {
        self.data_offer.as_ref()
    }

    pub fn set_actions(&self, actions: DndAction, preferred_action: DndAction) {
        if let Some(ref data_offer) = self.data_offer {
            if data_offer.version() >= 3 {
                data_offer.set_actions(actions, preferred_action);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DragOffer {
    /// the wl_data offer if it exists
    pub(crate) data_offer: WlDataOffer,
    /// the serial for this data offer's enter event
    pub serial: u32,
    /// the surface that this DnD is active on
    pub surface: WlSurface,
    /// the x position on the surface
    pub x: f64,
    /// the y position on this surface
    pub y: f64,
    /// the timestamp a motion event was received in millisecond granularity
    pub time: Option<u32>,
    /// accepted mime type
    pub accepted_mime_type: Option<String>,
    /// the advertised drag actions
    pub source_actions: DndAction,
    /// the compositor selected drag action
    pub selected_action: DndAction,
}

impl PartialEq for DragOffer {
    fn eq(&self, other: &Self) -> bool {
        self.data_offer == other.data_offer
    }
}

impl DragOffer {
    pub fn finish(&self) {
        if self.data_offer.version() >= 3 {
            self.data_offer.finish();
        }
    }

    /// Set the accepted and preferred drag and drop actions.
    /// This request determines the final result of the drag-and-drop operation.
    /// If the end result is that no action is accepted, the drag source will receive wl_data_source.cancelled.
    pub fn set_actions(&self, actions: DndAction, preferred_action: DndAction) {
        if self.data_offer.version() >= 3 {
            self.data_offer.set_actions(actions, preferred_action);
        }
    }

    /// Receive data with the given mime type.
    /// This request may happen multiple times for different mime types, both before and after wl_data_device.drop.
    /// Drag-and-drop destination clients may preemptively fetch data or examine it more closely to determine acceptance.
    pub fn receive(&self, mime_type: String) -> std::io::Result<ReadPipe> {
        receive(&self.data_offer, mime_type)
    }

    /// Accept the given mime type, or None to reject the offer.
    /// In version 2, this request is used for feedback, but doesn't affect the final result of the drag-and-drop operation.
    /// In version 3, this request determines the final result of the drag-and-drop operation.
    pub fn accept_mime_type(&mut self, serial: u32, mime_type: Option<String>) {
        self.accepted_mime_type = mime_type.clone();
        self.data_offer.accept(serial, mime_type);
    }

    /// Destroy the data offer.
    pub fn destroy(&self) {
        self.data_offer.destroy();
    }

    /// Retrieve a reference to the inner wl_data_offer.
    pub fn inner(&self) -> &WlDataOffer {
        &self.data_offer
    }
}

#[derive(Debug, Clone)]
pub struct SelectionOffer {
    /// the wl_data offer
    pub(crate) data_offer: WlDataOffer,
}

impl PartialEq for SelectionOffer {
    fn eq(&self, other: &Self) -> bool {
        self.data_offer == other.data_offer
    }
}

/// An error that may occur when working with data offers.
#[derive(Debug, thiserror::Error)]
pub enum DataOfferError {
    /// A compositor global was available, but did not support the given minimum version
    #[error("offer is not valid to receive from yet")]
    InvalidReceive,

    #[error("IO error")]
    Io(std::io::Error),
}

impl SelectionOffer {
    pub fn receive(&self, mime_type: String) -> Result<ReadPipe, DataOfferError> {
        receive(&self.data_offer, mime_type).map_err(DataOfferError::Io)
    }

    pub fn destroy(&self) {
        self.data_offer.destroy();
    }

    pub fn inner(&self) -> &WlDataOffer {
        &self.data_offer
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataDeviceOffer {
    Drag(DragOffer),
    Selection(SelectionOffer),
    Undetermined(UndeterminedOffer),
}

impl Default for DataDeviceOffer {
    fn default() -> Self {
        DataDeviceOffer::Undetermined(UndeterminedOffer { data_offer: None })
    }
}

impl DataDeviceOffer {
    /// # Safety
    ///
    /// The provided file destructor must be a valid FD for writing, and will be closed
    /// once the contents are written.
    pub unsafe fn receive_to_fd(
        &self,
        mime_type: String,
        fd: BorrowedFd,
    ) -> Result<(), DataOfferError> {
        let inner = match self {
            DataDeviceOffer::Drag(o) => o.inner(),
            DataDeviceOffer::Selection(o) => o.inner(),
            DataDeviceOffer::Undetermined(_) => return Err(DataOfferError::InvalidReceive), // error?
        };

        unsafe { receive_to_fd(inner, mime_type, fd.as_raw_fd()) };
        Ok(())
    }

    pub fn receive(&self, mime_type: String) -> Result<ReadPipe, DataOfferError> {
        let inner = match self {
            DataDeviceOffer::Drag(o) => o.inner(),
            DataDeviceOffer::Selection(o) => o.inner(),
            DataDeviceOffer::Undetermined(_) => return Err(DataOfferError::InvalidReceive), // error?
        };

        receive(inner, mime_type).map_err(DataOfferError::Io)
    }

    pub fn accept_mime_type(&mut self, serial: u32, mime_type: Option<String>) {
        match self {
            DataDeviceOffer::Drag(o) => o.accept_mime_type(serial, mime_type),
            DataDeviceOffer::Selection(_) => {}
            DataDeviceOffer::Undetermined(_) => {} // error?
        };
    }

    pub fn set_actions(&mut self, actions: DndAction, preferred_action: DndAction) {
        match self {
            DataDeviceOffer::Drag(o) => o.set_actions(actions, preferred_action),
            DataDeviceOffer::Selection(_) => {}
            DataDeviceOffer::Undetermined(o) => o.set_actions(actions, preferred_action), // error?
        };
    }
}

#[derive(Debug, Default)]
pub struct DataOfferData {
    pub(crate) inner: Arc<Mutex<DataDeviceOfferInner>>,
}

#[derive(Debug, Default)]
pub struct DataDeviceOfferInner {
    pub(crate) offer: DataDeviceOffer,
    pub(crate) mime_types: Vec<String>,
}

impl DataOfferData {
    pub(crate) fn push_mime_type(&self, mime_type: String) {
        self.inner.lock().unwrap().mime_types.push(mime_type);
    }

    pub(crate) fn set_source_action(&self, action: DndAction) {
        let mut inner = self.inner.lock().unwrap();
        match &mut inner.deref_mut().offer {
            DataDeviceOffer::Drag(ref mut o) => o.source_actions = action,
            DataDeviceOffer::Selection(_) => {}
            DataDeviceOffer::Undetermined(_) => {}
        };
    }

    pub(crate) fn set_selected_action(&self, action: DndAction) {
        let mut inner = self.inner.lock().unwrap();
        match &mut inner.deref_mut().offer {
            DataDeviceOffer::Drag(ref mut o) => o.selected_action = action,
            DataDeviceOffer::Selection(_) => {}    // error?
            DataDeviceOffer::Undetermined(_) => {} // error?
        };
    }

    pub(crate) fn to_selection_offer(&self) {
        let mut inner = self.inner.lock().unwrap();
        match &mut inner.deref_mut().offer {
            DataDeviceOffer::Drag(o) => {
                inner.offer =
                    DataDeviceOffer::Selection(SelectionOffer { data_offer: o.data_offer.clone() });
            }
            DataDeviceOffer::Selection(_) => {}
            DataDeviceOffer::Undetermined(o) => {
                inner.offer = DataDeviceOffer::Selection(SelectionOffer {
                    data_offer: o.data_offer.clone().unwrap(),
                });
            }
        }
    }

    pub(crate) fn init_undetermined_offer(&self, offer: &WlDataOffer) {
        let mut inner = self.inner.lock().unwrap();
        match &mut inner.deref_mut().offer {
            DataDeviceOffer::Drag(_) => {
                inner.offer = DataDeviceOffer::Undetermined(UndeterminedOffer {
                    data_offer: Some(offer.clone()),
                });
            }
            DataDeviceOffer::Selection(_) => {
                inner.offer = DataDeviceOffer::Undetermined(UndeterminedOffer {
                    data_offer: Some(offer.clone()),
                });
            }
            DataDeviceOffer::Undetermined(o) => {
                o.data_offer = Some(offer.clone());
            }
        }
    }

    pub(crate) fn to_dnd_offer(
        &self,
        serial: u32,
        surface: WlSurface,
        x: f64,
        y: f64,
        time: Option<u32>,
    ) {
        let mut inner = self.inner.lock().unwrap();
        match &mut inner.deref_mut().offer {
            DataDeviceOffer::Drag(_) => {}
            DataDeviceOffer::Selection(o) => {
                inner.offer = DataDeviceOffer::Drag(DragOffer {
                    data_offer: o.data_offer.clone(),
                    accepted_mime_type: None,
                    source_actions: DndAction::empty(),
                    selected_action: DndAction::empty(),
                    serial,
                    surface,
                    x,
                    y,
                    time,
                });
            }
            DataDeviceOffer::Undetermined(o) => {
                inner.offer = DataDeviceOffer::Drag(DragOffer {
                    data_offer: o.data_offer.clone().unwrap(),
                    accepted_mime_type: None,
                    source_actions: DndAction::empty(),
                    selected_action: DndAction::empty(),
                    serial,
                    surface,
                    x,
                    y,
                    time,
                });
            }
        }
    }

    pub(crate) fn motion(&self, x: f64, y: f64, time: u32) {
        let mut inner = self.inner.lock().unwrap();
        match &mut inner.deref_mut().offer {
            DataDeviceOffer::Drag(o) => {
                o.x = x;
                o.y = y;
                o.time = Some(time);
            }
            DataDeviceOffer::Selection(_) => {}
            DataDeviceOffer::Undetermined(_) => {}
        }
    }
}

impl DataOfferDataExt for DataOfferData {
    fn data_offer_data(&self) -> &DataOfferData {
        self
    }

    fn as_drag_offer(&self) -> Option<DragOffer> {
        match &self.inner.lock().unwrap().deref().offer {
            DataDeviceOffer::Drag(o) => Some(o.clone()),
            _ => None,
        }
    }

    fn as_selection_offer(&self) -> Option<SelectionOffer> {
        match &self.inner.lock().unwrap().deref().offer {
            DataDeviceOffer::Selection(o) => Some(o.clone()),
            _ => None,
        }
    }

    fn mime_types(&self) -> Vec<String> {
        self.inner.lock().unwrap().mime_types.clone()
    }
}

pub trait DataOfferDataExt {
    fn data_offer_data(&self) -> &DataOfferData;
    fn mime_types(&self) -> Vec<String>;
    fn as_drag_offer(&self) -> Option<DragOffer>;
    fn as_selection_offer(&self) -> Option<SelectionOffer>;
}

/// Handler trait for DataOffer events.
///
/// The functions defined in this trait are called as DataOffer events are received from the compositor.
pub trait DataOfferHandler: Sized {
    /// Called for each mime type the data offer advertises.
    fn offer(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        offer: &mut DataDeviceOffer,
        mime_type: String,
    );

    /// Called to advertise the available DnD Actions as set by the source.
    fn source_actions(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        offer: &mut DragOffer,
        actions: DndAction,
    );

    /// Called to advertise the action selected by the compositor after matching the source/destination side actions.
    /// Only one action or none will be selected in the actions sent by the compositor.
    /// This may be called multiple times during a DnD operation.
    /// The most recent DndAction is the only valid one.
    ///
    /// At the time of a `drop` event on the data device, this action must be used except in the case of an ask action.
    /// In the case that the last action received is `ask`, the destination asks the user for their preference, then calls set_actions & accept each one last time.
    /// Finally, the destination may then request data to be sent and finishing the data offer
    ///
    fn selected_action(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        offer: &mut DragOffer,
        actions: DndAction,
    );
}

impl<D, U> Dispatch<wl_data_offer::WlDataOffer, U, D> for DataDeviceManagerState
where
    D: Dispatch<wl_data_offer::WlDataOffer, U> + DataOfferHandler,
    U: DataOfferDataExt,
{
    fn event(
        state: &mut D,
        _offer: &wl_data_offer::WlDataOffer,
        event: <wl_data_offer::WlDataOffer as wayland_client::Proxy>::Event,
        data: &U,
        conn: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<D>,
    ) {
        let data = data.data_offer_data();

        match event {
            wl_data_offer::Event::Offer { mime_type } => {
                data.push_mime_type(mime_type.clone());
                state.offer(conn, qh, &mut data.inner.lock().unwrap().offer, mime_type);
            }
            wl_data_offer::Event::SourceActions { source_actions } => {
                match source_actions {
                    wayland_client::WEnum::Value(a) => {
                        data.set_source_action(a);
                        match &mut data.inner.lock().unwrap().offer {
                            DataDeviceOffer::Drag(o) => {
                                state.source_actions(conn, qh, o, a);
                            }
                            DataDeviceOffer::Selection(_) => {}
                            DataDeviceOffer::Undetermined(_) => {}
                        }
                    }
                    wayland_client::WEnum::Unknown(_) => {} // Ignore
                }
            }
            wl_data_offer::Event::Action { dnd_action } => {
                match dnd_action {
                    wayland_client::WEnum::Value(a) => {
                        data.set_selected_action(a);
                        match &mut data.inner.lock().unwrap().offer {
                            DataDeviceOffer::Drag(o) => {
                                state.selected_action(conn, qh, o, a);
                            }
                            DataDeviceOffer::Selection(_) => {}
                            DataDeviceOffer::Undetermined(_) => {}
                        }
                    }
                    wayland_client::WEnum::Unknown(_) => {} // Ignore
                }
            }
            _ => unimplemented!(),
        };
    }
}

/// Request to receive the data of a given mime type.
///
/// You can do this several times, as a reaction to motion of
/// the dnd cursor, or to inspect the data in order to choose your
/// response.
///
/// Note that you should *not* read the contents right away in a
/// blocking way, as you may deadlock your application doing so.
/// At least make sure you flush your events to the server before
/// doing so.
///
/// Fails if too many file descriptors were already open and a pipe
/// could not be created.
pub fn receive(offer: &WlDataOffer, mime_type: String) -> std::io::Result<ReadPipe> {
    use nix::fcntl::OFlag;
    use nix::unistd::{close, pipe2};
    // create a pipe
    let (readfd, writefd) = pipe2(OFlag::O_CLOEXEC)?;

    offer.receive(mime_type, writefd);

    if let Err(err) = close(writefd) {
        log::warn!("Failed to close write pipe: {}", err);
    }

    Ok(unsafe { FromRawFd::from_raw_fd(readfd) })
}

/// Receive data to the write end of a raw file descriptor. If you have the read end, you can read from it.
///
/// You can do this several times, as a reaction to motion of
/// the dnd cursor, or to inspect the data in order to choose your
/// response.
///
/// Note that you should *not* read the contents right away in a
/// blocking way, as you may deadlock your application doing so.
/// At least make sure you flush your events to the server before
/// doing so.
///
/// # Safety
///
/// The provided file destructor must be a valid FD for writing, and will be closed
/// once the contents are written.
pub unsafe fn receive_to_fd(offer: &WlDataOffer, mime_type: String, writefd: RawFd) {
    use nix::unistd::close;

    offer.receive(mime_type, writefd);

    if let Err(err) = close(writefd) {
        log::warn!("Failed to close write pipe: {}", err);
    }
}

#[macro_export]
macro_rules! delegate_data_offer {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, udata: [$($udata: ty),*$(,)?]) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_data_offer::WlDataOffer: $udata,
            ] => $crate::data_device_manager::DataDeviceManagerState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_data_offer::WlDataOffer: $crate::data_device_manager::data_offer::DataOfferData
            ] => $crate::data_device_manager::DataDeviceManagerState
        );
    };
}
