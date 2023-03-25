pub mod data_device;
pub mod data_offer;
pub mod data_source;
mod read_pipe;
mod write_pipe;

pub use read_pipe::*;
pub use write_pipe::*;

use std::marker::PhantomData;

use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::{
        wl_data_device,
        wl_data_device_manager::{self, DndAction, WlDataDeviceManager},
        wl_data_source::WlDataSource,
        wl_seat::WlSeat,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

use crate::{
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
};

use self::{
    data_device::{DataDevice, DataDeviceData, DataDeviceDataExt},
    data_offer::DataOfferData,
    data_source::{CopyPasteSource, DataSourceData, DataSourceDataExt, DragSource},
};

#[derive(Debug)]
pub struct DataDeviceManagerState<V = DataOfferData> {
    manager: WlDataDeviceManager,
    _phantom: PhantomData<V>,
}

impl DataDeviceManagerState {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<WlDataDeviceManager, GlobalData, State> + 'static,
    {
        let manager = globals.bind(qh, 1..=3, GlobalData)?;
        Ok(Self { manager, _phantom: PhantomData })
    }

    pub fn data_device_manager(&self) -> &WlDataDeviceManager {
        &self.manager
    }

    /// creates a data source for copy paste
    pub fn create_copy_paste_source<'s, D, I>(
        &self,
        qh: &QueueHandle<D>,
        mime_types: I,
    ) -> CopyPasteSource
    where
        D: Dispatch<WlDataSource, DataSourceData> + 'static,
        I: IntoIterator<Item = &'s str>,
    {
        CopyPasteSource { inner: self.create_data_source(qh, mime_types, None) }
    }

    /// creates a data source for drag and drop
    pub fn create_drag_and_drop_source<'s, D, I>(
        &self,
        qh: &QueueHandle<D>,
        mime_types: I,
        dnd_actions: DndAction,
    ) -> DragSource
    where
        D: Dispatch<WlDataSource, DataSourceData> + 'static,
        I: IntoIterator<Item = &'s str>,
    {
        DragSource { inner: self.create_data_source(qh, mime_types, Some(dnd_actions)) }
    }

    /// creates a data source
    fn create_data_source<'s, D, I>(
        &self,
        qh: &QueueHandle<D>,
        mime_types: I,
        dnd_actions: Option<DndAction>,
    ) -> WlDataSource
    where
        D: Dispatch<WlDataSource, DataSourceData> + 'static,
        I: IntoIterator<Item = &'s str>,
    {
        let source = self.create_data_source_with_data(qh, Default::default());

        for mime in mime_types {
            source.offer(mime.to_string());
        }

        if self.manager.version() >= 3 {
            if let Some(dnd_actions) = dnd_actions {
                source.set_actions(dnd_actions);
            }
        }

        source
    }

    /// create a new data source for a given seat with some user data
    pub fn create_data_source_with_data<D, U>(&self, qh: &QueueHandle<D>, data: U) -> WlDataSource
    where
        D: Dispatch<WlDataSource, U> + 'static,
        U: DataSourceDataExt + 'static,
    {
        self.manager.create_data_source(qh, data)
    }

    /// create a new data device for a given seat
    pub fn get_data_device<D>(&self, qh: &QueueHandle<D>, seat: &WlSeat) -> DataDevice
    where
        D: Dispatch<wl_data_device::WlDataDevice, DataDeviceData> + 'static,
    {
        DataDevice { device: self.get_data_device_with_data(qh, seat, Default::default()) }
    }

    /// create a new data device for a given seat with some user data
    pub fn get_data_device_with_data<D, U>(
        &self,
        qh: &QueueHandle<D>,
        seat: &WlSeat,
        data: U,
    ) -> wl_data_device::WlDataDevice
    where
        D: Dispatch<wl_data_device::WlDataDevice, U> + 'static,
        U: DataDeviceDataExt + 'static,
    {
        self.manager.get_data_device(seat, qh, data)
    }
}

impl ProvidesBoundGlobal<WlDataDeviceManager, 3> for DataDeviceManagerState {
    fn bound_global(&self) -> Result<WlDataDeviceManager, GlobalError> {
        Ok(self.manager.clone())
    }
}

impl<D> Dispatch<wl_data_device_manager::WlDataDeviceManager, GlobalData, D>
    for DataDeviceManagerState
where
    D: Dispatch<wl_data_device_manager::WlDataDeviceManager, GlobalData>,
{
    fn event(
        _state: &mut D,
        _proxy: &wl_data_device_manager::WlDataDeviceManager,
        _event: <wl_data_device_manager::WlDataDeviceManager as wayland_client::Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

#[macro_export]
macro_rules! delegate_data_device_manager {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_data_device_manager::WlDataDeviceManager: $crate::globals::GlobalData
            ] => $crate::data_device_manager::DataDeviceManagerState
        );
    };
}
