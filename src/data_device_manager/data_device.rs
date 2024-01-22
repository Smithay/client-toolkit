use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use wayland_client::protocol::wl_surface::WlSurface;

use crate::{
    data_device_manager::data_offer::DataDeviceOffer,
    reexports::client::{
        event_created_child,
        protocol::{
            wl_data_device::{self, WlDataDevice},
            wl_data_offer::{self, WlDataOffer},
            wl_seat::WlSeat,
        },
        Connection, Dispatch, Proxy, QueueHandle,
    },
};

use super::{
    data_offer::{DataOfferData, DataOfferHandler, DragOffer, SelectionOffer},
    DataDeviceManagerState,
};

/// Handler trait for DataDevice events.
///
/// The functions defined in this trait are called as DataDevice events are received from the compositor.
pub trait DataDeviceHandler: Sized {
    // Introduces a new data offer
    // ASHLEY left out because the data offer will be introduced to the user once the type is known
    // either through the enter method or the selection method.
    // fn data_offer(
    //     &mut self,
    //     conn: &Connection,
    //     qh: &QueueHandle<Self>,
    //     data_device: DataDevice,
    //     offer: WlDataOffer,
    // );

    /// The data device pointer has entered a surface at the provided location
    fn enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        data_device: &WlDataDevice,
        x: f64,
        y: f64,
        wl_surface: &WlSurface,
    );

    /// The drag and drop pointer has left the surface and the session ends.
    /// The offer will be destroyed unless it was previously dropped.
    /// In the case of a dropped offer, the client must destroy it manually after it is finished.
    fn leave(&mut self, conn: &Connection, qh: &QueueHandle<Self>, data_device: &WlDataDevice);

    /// Drag and Drop motion.
    fn motion(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        data_device: &WlDataDevice,
        x: f64,
        y: f64,
    );

    /// Advertises a new selection.
    fn selection(&mut self, conn: &Connection, qh: &QueueHandle<Self>, data_device: &WlDataDevice);

    /// Drop performed.
    /// After the next data offer action event, data may be able to be received, unless the action is "ask".
    fn drop_performed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        data_device: &WlDataDevice,
    );
}

#[derive(Debug, Eq, PartialEq)]
pub struct DataDevice {
    pub(crate) device: WlDataDevice,
}

impl DataDevice {
    pub fn data(&self) -> &DataDeviceData {
        self.device.data().unwrap()
    }

    /// Unset the selection of the provided data device as a response to the event with with provided serial.
    pub fn unset_selection(&self, serial: u32) {
        self.device.set_selection(None, serial);
    }

    pub fn inner(&self) -> &WlDataDevice {
        &self.device
    }
}

impl Drop for DataDevice {
    fn drop(&mut self) {
        if self.device.version() >= 2 {
            self.device.release()
        }
    }
}

impl<D> Dispatch<wl_data_device::WlDataDevice, DataDeviceData, D> for DataDeviceManagerState
where
    D: Dispatch<wl_data_device::WlDataDevice, DataDeviceData>
        + Dispatch<wl_data_offer::WlDataOffer, DataOfferData>
        + DataDeviceHandler
        + DataOfferHandler
        + 'static,
{
    event_created_child!(D, WlDataDevice, [
        0 => (WlDataOffer, Default::default())
    ]);

    fn event(
        state: &mut D,
        data_device: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        data: &DataDeviceData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        use wayland_client::protocol::wl_data_device::Event;
        let mut inner = data.inner.lock().unwrap();

        match event {
            Event::DataOffer { id } => {
                inner.undetermined_offers.push(id.clone());
                let data = id.data::<DataOfferData>().unwrap();
                data.init_undetermined_offer(&id);
            }
            Event::Enter { serial, surface, x, y, id } => {
                // XXX the spec isn't clear here.
                if let Some(offer) = inner.drag_offer.take() {
                    offer.destroy();
                }

                if let Some(offer) = id {
                    if let Some(i) = inner.undetermined_offers.iter().position(|o| o == &offer) {
                        inner.undetermined_offers.remove(i);
                    }

                    let data = offer.data::<DataOfferData>().unwrap();
                    data.to_dnd_offer(serial, surface.clone(), x, y, None);

                    inner.drag_offer = Some(offer.clone());
                }
                // XXX Drop done here to prevent Mutex deadlocks.
                drop(inner);
                state.enter(conn, qh, data_device, x, y, &surface);
            }
            Event::Leave => {
                // We must destroy the offer we've got on enter.
                if let Some(offer) = inner.drag_offer.take() {
                    let data = offer.data::<DataOfferData>().unwrap();
                    if !data.leave() {
                        inner.drag_offer = Some(offer);
                    }
                }
                // XXX Drop done here to prevent Mutex deadlocks.
                drop(inner);
                state.leave(conn, qh, data_device);
            }
            Event::Motion { time, x, y } => {
                if let Some(offer) = inner.drag_offer.take() {
                    let data = offer.data::<DataOfferData>().unwrap();
                    // Update the data offer location.
                    data.motion(x, y, time);
                    inner.drag_offer = Some(offer);
                }

                // XXX Drop done here to prevent Mutex deadlocks.
                drop(inner);
                state.motion(conn, qh, data_device, x, y);
            }
            Event::Drop => {
                if let Some(offer) = inner.drag_offer.take() {
                    let data = offer.data::<DataOfferData>().unwrap();

                    let mut drag_inner = data.inner.lock().unwrap();

                    if let DataDeviceOffer::Drag(ref mut o) = drag_inner.deref_mut().offer {
                        o.dropped = true;
                    }
                    drop(drag_inner);

                    inner.drag_offer = Some(offer);
                }
                // XXX Drop done here to prevent Mutex deadlocks.
                drop(inner);
                // Pass the info about the drop to the user.
                state.drop_performed(conn, qh, data_device);
            }
            Event::Selection { id } => {
                // We must drop the current offer regardless.
                if let Some(offer) = inner.selection_offer.take() {
                    offer.destroy();
                }

                if let Some(offer) = id {
                    if let Some(i) = inner.undetermined_offers.iter().position(|o| o == &offer) {
                        inner.undetermined_offers.remove(i);
                    }

                    let data = offer.data::<DataOfferData>().unwrap();
                    data.to_selection_offer();
                    inner.selection_offer = Some(offer.clone());
                    // XXX Drop done here to prevent Mutex deadlocks.
                    drop(inner);
                    state.selection(conn, qh, data_device);
                }
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct DataDeviceData {
    /// The seat associated with this device.
    pub(crate) seat: WlSeat,
    /// The inner mutable storage.
    pub(crate) inner: Arc<Mutex<DataDeviceInner>>,
}

impl DataDeviceData {
    pub(crate) fn new(seat: WlSeat) -> Self {
        Self { seat, inner: Default::default() }
    }

    /// Get the seat associated with this data device.
    pub fn seat(&self) -> &WlSeat {
        &self.seat
    }

    /// Get the active dnd offer if it exists.
    pub fn drag_offer(&self) -> Option<DragOffer> {
        self.inner.lock().unwrap().drag_offer.as_ref().and_then(|offer| {
            let data = offer.data::<DataOfferData>().unwrap();
            data.as_drag_offer()
        })
    }

    /// Get the active selection offer if it exists.
    pub fn selection_offer(&self) -> Option<SelectionOffer> {
        self.inner.lock().unwrap().selection_offer.as_ref().and_then(|offer| {
            let data = offer.data::<DataOfferData>().unwrap();
            data.as_selection_offer()
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct DataDeviceInner {
    /// the active dnd offer and its data
    pub drag_offer: Option<WlDataOffer>,
    /// the active selection offer and its data
    pub selection_offer: Option<WlDataOffer>,
    /// the active undetermined offers and their data
    pub undetermined_offers: Vec<WlDataOffer>,
}
