use std::sync::Mutex;

use super::{
    data_offer::{DataOfferData, DataOfferDataExt, DataOfferHandler, DragOffer, SelectionOffer},
    DataDeviceManagerState,
};
use std::sync::Arc;
use wayland_client::{
    event_created_child,
    protocol::{
        wl_data_device::{self, WlDataDevice},
        wl_data_offer::{self, WlDataOffer},
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

#[derive(Debug, Clone)]
pub struct DataDevice {
    pub(crate) device: WlDataDevice,
}

impl DataDevice {
    pub fn release(&self) {
        if self.device.version() >= 2 {
            self.device.release()
        }
    }

    pub fn data(&self) -> Option<&DataDeviceData> {
        self.device.data()
    }

    /// Unset the selection of the provided data device as a response to the event with with provided serial.
    pub fn unset_selection(&self, serial: u32) {
        self.device.set_selection(None, serial);
    }
}

#[derive(Debug, Default)]
pub struct DataDeviceInner {
    /// the active dnd offer and its data
    pub drag_offer: Arc<Mutex<Option<WlDataOffer>>>,
    /// the active selection offer and its data
    pub selection_offer: Arc<Mutex<Option<WlDataOffer>>>,
    /// the active undetermined offers and their data
    pub undetermined_offers: Arc<Mutex<Vec<WlDataOffer>>>,
}

#[derive(Debug, Default)]
pub struct DataDeviceData {
    pub(super) inner: Arc<Mutex<DataDeviceInner>>,
}

pub trait DataDeviceDataExt: Send + Sync {
    type DataOfferInner: DataOfferDataExt + Send + Sync + 'static;

    fn data_device_data(&self) -> &DataDeviceData;

    fn selection_mime_types(&self) -> Vec<String> {
        let inner = self.data_device_data();
        inner
            .inner
            .lock()
            .unwrap()
            .selection_offer
            .lock()
            .unwrap()
            .as_ref()
            .map(|offer| {
                let data = offer.data::<Self::DataOfferInner>().unwrap();
                data.mime_types()
            })
            .unwrap_or_default()
    }

    fn drag_mime_types(&self) -> Vec<String> {
        let inner = self.data_device_data();
        inner
            .inner
            .lock()
            .unwrap()
            .drag_offer
            .lock()
            .unwrap()
            .as_ref()
            .map(|offer| {
                let data = offer.data::<Self::DataOfferInner>().unwrap();
                data.mime_types()
            })
            .unwrap_or_default()
    }

    /// Get the active dnd offer if it exists.
    fn drag_offer(&self) -> Option<DragOffer> {
        let inner = self.data_device_data();
        inner.inner.lock().unwrap().drag_offer.lock().unwrap().as_ref().and_then(|offer| {
            let data = offer.data::<Self::DataOfferInner>().unwrap();
            data.as_drag_offer()
        })
    }

    /// Get the active selection offer if it exists.
    fn selection_offer(&self) -> Option<SelectionOffer> {
        let inner = self.data_device_data();
        inner.inner.lock().unwrap().selection_offer.lock().unwrap().as_ref().and_then(|offer| {
            let data = offer.data::<Self::DataOfferInner>().unwrap();
            data.as_selection_offer()
        })
    }
}

impl DataDeviceDataExt for DataDevice {
    type DataOfferInner = DataOfferData;
    fn data_device_data(&self) -> &DataDeviceData {
        self.device.data().unwrap()
    }
}

impl DataDeviceDataExt for DataDeviceData {
    type DataOfferInner = DataOfferData;
    fn data_device_data(&self) -> &DataDeviceData {
        self
    }
}

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
    fn enter(&mut self, conn: &Connection, qh: &QueueHandle<Self>, data_device: DataDevice);

    /// The drag and drop pointer has left the surface and the session ends.
    /// The offer will be destroyed.
    fn leave(&mut self, conn: &Connection, qh: &QueueHandle<Self>, data_device: DataDevice);

    /// Drag and Drop motion.
    fn motion(&mut self, conn: &Connection, qh: &QueueHandle<Self>, data_device: DataDevice);

    /// Advertises a new selection.
    fn selection(&mut self, conn: &Connection, qh: &QueueHandle<Self>, data_device: DataDevice);

    /// Drop performed.
    /// After the next data offer action event, data may be able to be received, unless the action is "ask".
    fn drop_performed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        data_device: DataDevice,
    );
}

impl<D, U, V> Dispatch<wl_data_device::WlDataDevice, U, D> for DataDeviceManagerState<V>
where
    D: Dispatch<wl_data_device::WlDataDevice, U>
        + Dispatch<wl_data_offer::WlDataOffer, V>
        + DataDeviceHandler
        + DataOfferHandler
        + 'static,
    U: DataDeviceDataExt,
    V: DataOfferDataExt + Default + 'static + Send + Sync,
{
    event_created_child!(D, WlDataDevice, [
        0 => (WlDataOffer, V::default())
    ]);

    fn event(
        state: &mut D,
        data_device: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        data: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let data = data.data_device_data();
        let inner = data.inner.lock().unwrap();

        match event {
            wayland_client::protocol::wl_data_device::Event::DataOffer { id } => {
                // XXX Drop done here to prevent Mutex deadlocks.S

                inner.undetermined_offers.lock().unwrap().push(id.clone());
                let data = id.data::<V>().unwrap().data_offer_data();
                data.init_undetermined_offer(&id);

                // Append the data offer to our list of offers.
                drop(inner);
            }
            wayland_client::protocol::wl_data_device::Event::Enter {
                serial,
                surface,
                x,
                y,
                id,
            } => {
                let mut drag_offer = inner.drag_offer.lock().unwrap();

                if let Some(offer) = id {
                    let mut undetermined = inner.undetermined_offers.lock().unwrap();
                    if let Some(i) = undetermined.iter().position(|o| o == &offer) {
                        undetermined.remove(i);
                    }
                    drop(undetermined);

                    let data = offer.data::<V>().unwrap().data_offer_data();
                    data.to_dnd_offer(serial, surface, x, y, None);

                    // XXX Drop done here to prevent Mutex deadlocks.
                    *drag_offer = Some(offer.clone());
                    drop(drag_offer);
                    drop(inner);
                    state.enter(conn, qh, DataDevice { device: data_device.clone() });
                } else {
                    *drag_offer = None;
                }
            }
            wayland_client::protocol::wl_data_device::Event::Leave => {
                // XXX Drop done here to prevent Mutex deadlocks.
                inner.drag_offer.lock().unwrap().take();
                drop(inner);
                state.leave(conn, qh, DataDevice { device: data_device.clone() });
            }
            wayland_client::protocol::wl_data_device::Event::Motion { time, x, y } => {
                let mut drag = inner.drag_offer.lock().unwrap();
                if let Some(offer) = drag.take() {
                    let data = offer.data::<V>().unwrap().data_offer_data();
                    data.motion(x, y, time);
                    *drag = Some(offer);
                }
                // Update the data offer location.
                // XXX Drop done here to prevent Mutex deadlocks.
                drop(drag);
                drop(inner);
                state.motion(conn, qh, DataDevice { device: data_device.clone() });
            }
            wayland_client::protocol::wl_data_device::Event::Drop => {
                // XXX Drop done here to prevent Mutex deadlocks.
                // Pass the info about the drop to the user.
                drop(inner);
                state.drop_performed(conn, qh, DataDevice { device: data_device.clone() });
            }
            wayland_client::protocol::wl_data_device::Event::Selection { id } => {
                let mut selection_offer = inner.selection_offer.lock().unwrap();

                if let Some(offer) = id {
                    let mut undetermined = inner.undetermined_offers.lock().unwrap();
                    if let Some(i) = undetermined.iter().position(|o| o == &offer) {
                        undetermined.remove(i);
                    }
                    drop(undetermined);

                    let data = offer.data::<V>().unwrap().data_offer_data();
                    data.to_selection_offer();
                    // XXX Drop done here to prevent Mutex deadlocks.
                    *selection_offer = Some(offer.clone());
                    drop(selection_offer);
                    drop(inner);
                    state.selection(conn, qh, DataDevice { device: data_device.clone() });
                } else {
                    *selection_offer = None;
                }
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_data_device {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, udata: [$($udata: ty),*$(,)?]) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_data_device::WlDataDevice: $udata,
            ] => $crate::data_device_manager::DataDeviceManagerState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_data_device::WlDataDevice: $crate::data_device_manager::data_device::DataDeviceData
            ] => $crate::data_device_manager::DataDeviceManagerState
        );
    };
}
