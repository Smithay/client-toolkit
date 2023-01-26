use std::mem;
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
    pub mime_types: Vec<String>,
    pub accepted_mime_type: Option<String>,
    pub source_actions: DndAction,
}
impl PartialEq for UndeterminedOffer {
    fn eq(&self, other: &Self) -> bool {
        self.data_offer == other.data_offer
    }
}

impl UndeterminedOffer {
    pub fn accept(&mut self, serial: u32, mime_type: Option<String>) {
        self.accepted_mime_type = mime_type.clone();
        if let Some(offer) = self.data_offer.as_ref() {
            offer.accept(serial, mime_type);
        }
    }

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
    pub(crate) data_offer: Option<WlDataOffer>,
    /// the serial for this data offer
    pub serial: u32,
    /// the surface that this DnD is active on
    pub surface: WlSurface,
    /// the x position on the surface
    pub x: f64,
    /// the y position on this surface
    pub y: f64,
    /// the timestamp a motion event was received in millisecond granularity
    pub time: Option<u32>,
    /// the mime types of the data offer
    pub mime_types: Vec<String>,
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
        if let Some(ref data_offer) = self.data_offer {
            if data_offer.version() >= 3 {
                data_offer.finish();
            }
        }
    }

    pub fn set_actions(&self, actions: DndAction, preferred_action: DndAction) {
        if let Some(ref data_offer) = self.data_offer {
            if data_offer.version() >= 3 {
                data_offer.set_actions(actions, preferred_action);
            }
        }
    }

    pub fn receive(&self, mime_type: String) -> std::io::Result<ReadPipe> {
        if let Some(o) = self.data_offer.as_ref() {
            receive(o, mime_type)
        } else {
            panic!("offer is not valid"); // TODO error
        }
    }

    pub fn accept_mime_type(&mut self, serial: u32, mime_type: Option<String>) {
        if let Some(ref data_offer) = self.data_offer {
            self.accepted_mime_type = mime_type.clone();
            data_offer.accept(serial, mime_type);
        }
    }

    pub fn destroy(&self) {
        if let Some(ref data_offer) = self.data_offer {
            data_offer.destroy();
        }
    }

    pub fn inner(&self) -> Option<&WlDataOffer> {
        self.data_offer.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct SelectionOffer {
    /// the wl_data offer
    pub(crate) data_offer: Option<WlDataOffer>,
    /// the mime types of the data offer
    pub mime_types: Vec<String>,
    /// accepted mime type
    pub accepted_mime_type: Option<String>,
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
        if let Some(o) = self.data_offer.as_ref() {
            receive(o, mime_type).map_err(DataOfferError::Io)
        } else {
            Err(DataOfferError::InvalidReceive)
        }
    }

    pub fn accept(&mut self, serial: u32, mime_type: Option<String>) {
        if let Some(ref data_offer) = self.data_offer {
            self.accepted_mime_type = mime_type.clone();
            data_offer.accept(serial, mime_type);
        }
    }

    pub fn destroy(&self) {
        if let Some(ref data_offer) = self.data_offer {
            data_offer.destroy();
        }
    }

    pub fn inner(&self) -> Option<&WlDataOffer> {
        self.data_offer.as_ref()
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
        DataDeviceOffer::Undetermined(UndeterminedOffer {
            data_offer: None,
            mime_types: Vec::new(),
            accepted_mime_type: None,
            source_actions: DndAction::empty(),
        })
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
        match self {
            DataDeviceOffer::Drag(o) => unsafe {
                receive_to_fd(o.data_offer.as_ref().unwrap(), mime_type, fd.as_raw_fd());
                Ok(())
            },
            DataDeviceOffer::Selection(o) => unsafe {
                receive_to_fd(o.data_offer.as_ref().unwrap(), mime_type, fd.as_raw_fd());
                Ok(())
            },
            DataDeviceOffer::Undetermined(_) => Err(DataOfferError::InvalidReceive), // error?
        }
    }

    pub fn receive(&self, mime_type: String) -> Result<ReadPipe, DataOfferError> {
        let inner = match self {
            DataDeviceOffer::Drag(o) => o.inner(),
            DataDeviceOffer::Selection(o) => o.inner(),
            DataDeviceOffer::Undetermined(_) => return Err(DataOfferError::InvalidReceive), // error?
        };

        if let Some(o) = inner {
            receive(o, mime_type).map_err(DataOfferError::Io)
        } else {
            Err(DataOfferError::InvalidReceive)
        }
    }

    pub fn accept_mime_type(&mut self, serial: u32, mime_type: Option<String>) {
        match self {
            DataDeviceOffer::Drag(o) => o.accept_mime_type(serial, mime_type),
            DataDeviceOffer::Selection(o) => o.accept(serial, mime_type),
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
    pub(crate) inner: Arc<Mutex<DataDeviceOffer>>,
}

impl DataOfferData {
    pub(crate) fn push_mime_type(&self, mime_type: String) {
        let mut inner = self.inner.lock().unwrap();
        match inner.deref_mut() {
            DataDeviceOffer::Drag(ref mut o) => o.mime_types.push(mime_type),
            DataDeviceOffer::Selection(ref mut o) => o.mime_types.push(mime_type),
            DataDeviceOffer::Undetermined(ref mut o) => o.mime_types.push(mime_type),
        }
    }

    pub(crate) fn set_source_action(&self, action: DndAction) {
        let mut inner = self.inner.lock().unwrap();
        match inner.deref_mut() {
            DataDeviceOffer::Drag(ref mut o) => o.source_actions = action,
            DataDeviceOffer::Selection(_) => {}
            DataDeviceOffer::Undetermined(ref mut o) => o.source_actions = action,
        };
    }

    pub(crate) fn set_selected_action(&self, action: DndAction) {
        let mut inner = self.inner.lock().unwrap();
        match inner.deref_mut() {
            DataDeviceOffer::Drag(ref mut o) => o.selected_action = action,
            DataDeviceOffer::Selection(_) => {}    // error?
            DataDeviceOffer::Undetermined(_) => {} // error?
        };
    }

    pub(crate) fn to_selection_offer(&self) {
        let mut inner = self.inner.lock().unwrap();
        match inner.deref_mut() {
            DataDeviceOffer::Drag(o) => {
                *inner = DataDeviceOffer::Selection(SelectionOffer {
                    data_offer: o.data_offer.take(),
                    mime_types: mem::take(&mut o.mime_types),
                    accepted_mime_type: o.accepted_mime_type.take(),
                });
            }
            DataDeviceOffer::Selection(_) => {}
            DataDeviceOffer::Undetermined(o) => {
                *inner = DataDeviceOffer::Selection(SelectionOffer {
                    data_offer: o.data_offer.take(),
                    mime_types: mem::take(&mut o.mime_types),
                    accepted_mime_type: o.accepted_mime_type.take(),
                });
            }
        }
    }

    pub(crate) fn init_undetermined_offer(&self, offer: &WlDataOffer) {
        let mut inner = self.inner.lock().unwrap();
        match inner.deref_mut() {
            DataDeviceOffer::Drag(o) => {
                *inner = DataDeviceOffer::Undetermined(UndeterminedOffer {
                    data_offer: Some(offer.clone()),
                    mime_types: mem::take(&mut o.mime_types),
                    accepted_mime_type: o.accepted_mime_type.take(),
                    source_actions: o.source_actions,
                });
            }
            DataDeviceOffer::Selection(o) => {
                *inner = DataDeviceOffer::Undetermined(UndeterminedOffer {
                    data_offer: Some(offer.clone()),
                    mime_types: mem::take(&mut o.mime_types),
                    accepted_mime_type: o.accepted_mime_type.take(),
                    source_actions: DndAction::empty(),
                });
            }
            DataDeviceOffer::Undetermined(o) => {
                o.data_offer.replace(offer.clone());
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
        match inner.deref_mut() {
            DataDeviceOffer::Drag(_) => {}
            DataDeviceOffer::Selection(o) => {
                *inner = DataDeviceOffer::Drag(DragOffer {
                    data_offer: o.data_offer.take(),
                    mime_types: mem::take(&mut o.mime_types),
                    accepted_mime_type: o.accepted_mime_type.take(),
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
                *inner = DataDeviceOffer::Drag(DragOffer {
                    data_offer: o.data_offer.take(),
                    mime_types: mem::take(&mut o.mime_types),
                    accepted_mime_type: o.accepted_mime_type.take(),
                    source_actions: o.source_actions,
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
        match inner.deref_mut() {
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
        match self.inner.lock().unwrap().deref() {
            DataDeviceOffer::Drag(o) => Some(o.clone()),
            _ => None,
        }
    }

    fn as_selection_offer(&self) -> Option<SelectionOffer> {
        match self.inner.lock().unwrap().deref() {
            DataDeviceOffer::Selection(o) => Some(o.clone()),
            _ => None,
        }
    }

    fn mime_types(&self) -> Vec<String> {
        match self.inner.lock().unwrap().deref() {
            DataDeviceOffer::Drag(o) => o.mime_types.clone(),
            DataDeviceOffer::Selection(o) => o.mime_types.clone(),
            DataDeviceOffer::Undetermined(o) => o.mime_types.clone(),
        }
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

    /// Called to advertise the available DnD Actions as set by the source
    fn source_actions(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        offer: &mut DataDeviceOffer,
        actions: DndAction,
    );

    /// Called to advertise the action selected by the compositor after matching the source/destination side actions.
    /// Only one action or none will be selected in the actions sent by the compositor
    /// This may be called multiple times during a DnD operation
    /// The most recent DndAction is the only valid one.
    ///
    /// At the time of a `drop` event on the data device, this action must be used except in the case of an ask action.
    /// In the case that the last action received is `ask`, the destination asks the user for their preference, then calls set_actions & accept each one last time
    /// Finally, the destination may then request data to be sent and finishing the data offer
    ///
    fn actions(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        offer: &mut DataDeviceOffer,
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
                state.offer(conn, qh, &mut data.inner.lock().unwrap(), mime_type);
            }
            wl_data_offer::Event::SourceActions { source_actions } => {
                match source_actions {
                    wayland_client::WEnum::Value(a) => {
                        data.set_source_action(a);
                        state.source_actions(conn, qh, &mut data.inner.lock().unwrap(), a);
                    }
                    wayland_client::WEnum::Unknown(_) => {} // ignore
                }
            }
            wl_data_offer::Event::Action { dnd_action } => {
                match dnd_action {
                    wayland_client::WEnum::Value(a) => {
                        data.set_selected_action(a);
                        state.actions(conn, qh, &mut data.inner.lock().unwrap(), a);
                    }
                    wayland_client::WEnum::Unknown(_) => {} // ignore
                }
            }
            _ => unimplemented!(),
        };
    }
}

/// Request to receive the data of a given mime type
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
