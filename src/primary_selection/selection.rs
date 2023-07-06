use crate::reexports::client::{Connection, Dispatch, QueueHandle};
use crate::reexports::protocols::wp::primary_selection::zv1::client::zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1;
use crate::{data_device_manager::WritePipe, globals::GlobalData};

use super::{device::PrimarySelectionDevice, PrimarySelectionManagerState};

/// Handler trait for `PrimarySelectionSource` events.
///
/// The functions defined in this trait are called as DataSource events are received from the compositor.
pub trait PrimarySelectionSourceHandler: Sized {
    /// The client has requested the data for this source to be sent.
    /// Send the data, then close the fd.
    fn send_request(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        source: &ZwpPrimarySelectionSourceV1,
        mime: String,
        write_pipe: WritePipe,
    );

    /// The data source is no longer valid
    /// Cleanup & destroy this resource
    fn cancelled(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        source: &ZwpPrimarySelectionSourceV1,
    );
}

/// Wrapper around the [`ZwpPrimarySelectionSourceV1`].
#[derive(Debug, PartialEq, Eq)]
pub struct PrimarySelectionSource {
    source: ZwpPrimarySelectionSourceV1,
}

impl PrimarySelectionSource {
    pub(crate) fn new(source: ZwpPrimarySelectionSourceV1) -> Self {
        Self { source }
    }

    /// Set the selection on the given [`PrimarySelectionDevice`].
    pub fn set_selection(&self, device: &PrimarySelectionDevice, serial: u32) {
        device.device.set_selection(Some(&self.source), serial);
    }

    /// The underlying wayland object.
    pub fn inner(&self) -> &ZwpPrimarySelectionSourceV1 {
        &self.source
    }
}

impl Drop for PrimarySelectionSource {
    fn drop(&mut self) {
        self.source.destroy();
    }
}

impl<State> Dispatch<ZwpPrimarySelectionSourceV1, GlobalData, State>
    for PrimarySelectionManagerState
where
    State: Dispatch<ZwpPrimarySelectionSourceV1, GlobalData> + PrimarySelectionSourceHandler,
{
    fn event(
        state: &mut State,
        proxy: &ZwpPrimarySelectionSourceV1,
        event: <ZwpPrimarySelectionSourceV1 as wayland_client::Proxy>::Event,
        _: &GlobalData,
        conn: &wayland_client::Connection,
        qhandle: &QueueHandle<State>,
    ) {
        use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_source_v1::Event as PrimarySelectionSourceEvent;
        match event {
            PrimarySelectionSourceEvent::Send { mime_type, fd } => {
                state.send_request(conn, qhandle, proxy, mime_type, fd.into())
            }
            PrimarySelectionSourceEvent::Cancelled => state.cancelled(conn, qhandle, proxy),
            _ => unreachable!(),
        }
    }
}
