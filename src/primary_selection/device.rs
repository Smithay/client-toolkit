use std::sync::{Arc, Mutex};

use crate::reexports::client::{
    event_created_child, protocol::wl_seat::WlSeat, Connection, Dispatch, Proxy, QueueHandle,
};
use crate::reexports::protocols::wp::primary_selection::zv1::client::{
    zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1,
    zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1,
};

use super::{
    offer::{PrimarySelectionOffer, PrimarySelectionOfferData},
    PrimarySelectionManagerState,
};

pub trait PrimarySelectionDeviceHandler: Sized {
    /// The new selection is received.
    ///
    /// The given primary selection device could be used to identify [`PrimarySelectionDevice`].
    fn selection(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        primary_selection_device: &ZwpPrimarySelectionDeviceV1,
    );
}

#[derive(Debug)]
pub struct PrimarySelectionDevice {
    pub(crate) device: ZwpPrimarySelectionDeviceV1,
}

impl PrimarySelectionDevice {
    /// Remove the currently active selection.
    ///
    /// The passed `serial` is the serial of the input event.
    pub fn unset_selection(&self, serial: u32) {
        self.device.set_selection(None, serial);
    }

    /// Get the underlying data.
    pub fn data(&self) -> &PrimarySelectionDeviceData {
        self.device.data::<PrimarySelectionDeviceData>().unwrap()
    }

    pub fn inner(&self) -> &ZwpPrimarySelectionDeviceV1 {
        &self.device
    }
}

impl Drop for PrimarySelectionDevice {
    fn drop(&mut self) {
        self.device.destroy();
    }
}

impl<State> Dispatch<ZwpPrimarySelectionDeviceV1, PrimarySelectionDeviceData, State>
    for PrimarySelectionManagerState
where
    State: Dispatch<ZwpPrimarySelectionDeviceV1, PrimarySelectionDeviceData>
        + Dispatch<ZwpPrimarySelectionOfferV1, PrimarySelectionOfferData>
        + PrimarySelectionDeviceHandler
        + 'static,
{
    event_created_child!(State, ZwpPrimarySelectionDeviceV1, [
        0 => (ZwpPrimarySelectionOfferV1, PrimarySelectionOfferData::default())
    ]);

    fn event(
        state: &mut State,
        proxy: &ZwpPrimarySelectionDeviceV1,
        event: <ZwpPrimarySelectionDeviceV1 as wayland_client::Proxy>::Event,
        data: &PrimarySelectionDeviceData,
        conn: &Connection,
        qhandle: &QueueHandle<State>,
    ) {
        use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_device_v1::Event;
        let mut data = data.inner.lock().unwrap();
        match event {
            Event::DataOffer { offer } => {
                // Try to resist faulty compositors.
                if let Some(pending_offer) = data.pending_offer.take() {
                    pending_offer.destroy();
                }

                data.pending_offer = Some(offer);
            }
            Event::Selection { id } => {
                // We must drop the current offer regardless.
                if let Some(offer) = data.offer.take() {
                    offer.destroy();
                }

                if id == data.pending_offer {
                    data.offer = data.pending_offer.take();
                } else {
                    // Remove the pending offer, assign the new delivered one.
                    if let Some(offer) = data.pending_offer.take() {
                        offer.destroy()
                    }

                    data.offer = id;
                }

                // Release the user data lock before calling into user.
                drop(data);

                state.selection(conn, qhandle, proxy);
            }
            _ => unreachable!(),
        }
    }
}

/// The user data associated with the [`ZwpPrimarySelectionDeviceV1`].
#[derive(Debug)]
pub struct PrimarySelectionDeviceData {
    /// The seat associated with this device.
    seat: WlSeat,
    /// The inner mutable storage.
    inner: Arc<Mutex<PrimarySelectionDeviceDataInner>>,
}

impl PrimarySelectionDeviceData {
    pub(crate) fn new(seat: WlSeat) -> Self {
        Self { seat, inner: Default::default() }
    }

    /// The seat used to create this primary selection device.
    pub fn seat(&self) -> &WlSeat {
        &self.seat
    }

    /// The active selection offer.
    pub fn selection_offer(&self) -> Option<PrimarySelectionOffer> {
        self.inner
            .lock()
            .unwrap()
            .offer
            .as_ref()
            .map(|offer| PrimarySelectionOffer { offer: offer.clone() })
    }
}

#[derive(Debug, Default)]
struct PrimarySelectionDeviceDataInner {
    /// The offer is valid until either `NULL` or new selection is received via the
    /// `selection` event.
    offer: Option<ZwpPrimarySelectionOfferV1>,
    /// The offer we've got in `offer` event, but not finished it in `selection`.
    pending_offer: Option<ZwpPrimarySelectionOfferV1>,
}
