use crate::dispatch2::Dispatch2;
use crate::globals::GlobalData;
use crate::reexports::client::{
    globals::{BindError, GlobalList},
    protocol::wl_seat::WlSeat,
    Dispatch, QueueHandle,
};
use crate::reexports::protocols::wp::primary_selection::zv1::client::{
    zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
    zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1,
    zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1,
};

pub mod device;
pub mod offer;
pub mod selection;

use self::device::{PrimarySelectionDevice, PrimarySelectionDeviceData};
use selection::PrimarySelectionSource;

#[derive(Debug)]
pub struct PrimarySelectionManagerState {
    manager: ZwpPrimarySelectionDeviceManagerV1,
}

impl PrimarySelectionManagerState {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<ZwpPrimarySelectionDeviceManagerV1, GlobalData, State> + 'static,
    {
        let manager = globals.bind(qh, 1..=1, GlobalData)?;
        Ok(Self { manager })
    }

    /// The underlying wayland object.
    pub fn primary_selection_manager(&self) -> &ZwpPrimarySelectionDeviceManagerV1 {
        &self.manager
    }

    /// Create a primary selection source.
    pub fn create_selection_source<State, I, T>(
        &self,
        qh: &QueueHandle<State>,
        mime_types: I,
    ) -> PrimarySelectionSource
    where
        State: Dispatch<ZwpPrimarySelectionSourceV1, GlobalData, State> + 'static,
        I: IntoIterator<Item = T>,
        T: ToString,
    {
        let source = self.manager.create_source(qh, GlobalData);

        for mime_type in mime_types {
            source.offer(mime_type.to_string());
        }

        PrimarySelectionSource::new(source)
    }

    /// Get the primary selection data device for the given seat.
    pub fn get_selection_device<State>(
        &self,
        qh: &QueueHandle<State>,
        seat: &WlSeat,
    ) -> PrimarySelectionDevice
    where
        State: Dispatch<ZwpPrimarySelectionDeviceV1, PrimarySelectionDeviceData, State> + 'static,
    {
        PrimarySelectionDevice {
            device: self.manager.get_device(
                seat,
                qh,
                PrimarySelectionDeviceData::new(seat.clone()),
            ),
        }
    }
}

impl Drop for PrimarySelectionManagerState {
    fn drop(&mut self) {
        self.manager.destroy();
    }
}

impl<D> Dispatch2<ZwpPrimarySelectionDeviceManagerV1, D> for GlobalData {
    fn event(
        &self,
        _: &mut D,
        _: &ZwpPrimarySelectionDeviceManagerV1,
        _: <ZwpPrimarySelectionDeviceManagerV1 as wayland_client::Proxy>::Event,
        _: &wayland_client::Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zwp_primary_selection_device_manager_v1 has no events")
    }
}
