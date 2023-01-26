use wayland_backend::io_lifetimes::OwnedFd;
use wayland_client::{
    protocol::{
        wl_data_device_manager::DndAction,
        wl_data_source::{self, WlDataSource},
        wl_surface::WlSurface,
    },
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};

use super::{data_device::DataDevice, DataDeviceManagerState};

#[derive(Debug, Default)]
pub struct DataSourceData {}

pub trait DataSourceDataExt: Send + Sync {
    fn data_source_data(&self) -> &DataSourceData;
}

impl DataSourceDataExt for DataSourceData {
    fn data_source_data(&self) -> &DataSourceData {
        self
    }
}

/// Handler trait for DataSource events.
///
/// The functions defined in this trait are called as DataSource events are received from the compositor.
pub trait DataSourceHandler: Sized {
    /// This may be called multiple times, once for each accepted mime type from the destination, if any
    fn accept_mime(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        source: &WlDataSource,
        mime: Option<String>,
    );

    /// The client has requested the data for this source to be sent.
    /// Send the data, then close the fd.
    fn send_request(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        source: &WlDataSource,
        mime: String,
        fd: OwnedFd,
    );

    /// The data source is no longer valid
    /// Cleanup & destroy this resource
    fn cancelled(&mut self, conn: &Connection, qh: &QueueHandle<Self>, source: &WlDataSource);

    /// A drop was performed
    /// the data source will be used and should not be destroyed yet
    fn dnd_dropped(&mut self, conn: &Connection, qh: &QueueHandle<Self>, source: &WlDataSource);

    /// the dnd finished
    /// the data source may be destroyed
    fn dnd_finished(&mut self, conn: &Connection, qh: &QueueHandle<Self>, source: &WlDataSource);

    /// an action was selected by the compositor
    fn action(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        source: &WlDataSource,
        action: DndAction,
    );
}

impl<D, U> Dispatch<wl_data_source::WlDataSource, U, D> for DataDeviceManagerState
where
    D: Dispatch<wl_data_source::WlDataSource, U> + DataSourceHandler,
    U: DataSourceDataExt,
{
    fn event(
        state: &mut D,
        source: &wl_data_source::WlDataSource,
        event: <wl_data_source::WlDataSource as wayland_client::Proxy>::Event,
        _data: &U,
        conn: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<D>,
    ) {
        match event {
            wl_data_source::Event::Target { mime_type } => {
                state.accept_mime(conn, qh, source, mime_type)
            }
            wl_data_source::Event::Send { mime_type, fd } => {
                state.send_request(conn, qh, source, mime_type, fd);
            }
            wl_data_source::Event::Cancelled => {
                state.cancelled(conn, qh, source);
            }
            wl_data_source::Event::DndDropPerformed => {
                state.dnd_dropped(conn, qh, source);
            }
            wl_data_source::Event::DndFinished => {
                state.dnd_finished(conn, qh, source);
            }
            wl_data_source::Event::Action { dnd_action } => match dnd_action {
                WEnum::Value(dnd_action) => {
                    state.action(conn, qh, source, dnd_action);
                }
                WEnum::Unknown(_) => {}
            },
            _ => unimplemented!(),
        };
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CopyPasteSource {
    pub(crate) inner: WlDataSource,
}

impl CopyPasteSource {
    /// set the selection of the provided data device as a response to the event with with provided serial
    pub fn set_selection(&self, device: &DataDevice, serial: u32) {
        device.device.set_selection(Some(&self.inner), serial);
    }

    /// unset the selection of the provided data device as a response to the event with with provided serial
    pub fn unset_selection(&self, device: &DataDevice, serial: u32) {
        device.device.set_selection(None, serial);
    }

    pub fn inner(&self) -> &WlDataSource {
        &self.inner
    }
}

impl Drop for CopyPasteSource {
    fn drop(&mut self) {
        self.inner.destroy();
    }
}

#[derive(Debug)]
pub struct DragSource {
    pub(crate) inner: WlDataSource,
}

impl DragSource {
    /// start a normal drag and drop operation
    /// this can be used for both intra-client DnD or inter-client Dnd
    /// the drag is cancelled when the DragSource is dropped
    pub fn start_drag(
        &self,
        device: &DataDevice,
        origin: &WlSurface,
        icon: Option<&WlSurface>,
        serial: u32,
    ) {
        device.device.start_drag(Some(&self.inner), origin, icon, serial);
    }

    /// start an internal drag and drop operation
    /// This will pass a NULL source, and the client is expected to handle data passing internally.
    /// Only Enter, Leave, & Motion events will be sent to the client
    pub fn start_internal_drag(
        device: &DataDevice,
        origin: &WlSurface,
        icon: Option<&WlSurface>,
        serial: u32,
    ) {
        device.device.start_drag(None, origin, icon, serial);
    }

    pub fn set_actions(&self, dnd_actions: DndAction) {
        if self.inner.version() >= 3 {
            self.inner.set_actions(dnd_actions);
        }
        self.inner.set_actions(dnd_actions);
    }

    pub fn inner(&self) -> &WlDataSource {
        &self.inner
    }
}

impl Drop for DragSource {
    fn drop(&mut self) {
        self.inner.destroy();
    }
}

#[macro_export]
macro_rules! delegate_data_source {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, udata: [$($udata: ty),*$(,)?]) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_data_source::WlDataSource: $udata,
            ] => $crate::data_device_manager::DataDeviceManagerState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_data_source::WlDataSource: $crate::data_device_manager::data_source::DataSourceData
            ] => $crate::data_device_manager::DataDeviceManagerState
        );
    };
}
