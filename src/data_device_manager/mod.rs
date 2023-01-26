pub mod data_device;
pub mod data_offer;
pub mod data_source;

use std::{
    fs, io,
    marker::PhantomData,
    os::unix::prelude::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
};

use wayland_backend::io_lifetimes::{AsFd, FromFd, OwnedFd};
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

/// A file descriptor that can only be read from
///
/// If the `calloop` cargo feature is enabled, this can be used
/// as an `EventSource` in a calloop event loop.
#[must_use]
#[derive(Debug)]
pub struct ReadPipe {
    #[cfg(feature = "calloop")]
    file: calloop::generic::Generic<fs::File>,
    #[cfg(not(feature = "calloop"))]
    file: fs::File,
}

#[cfg(feature = "calloop")]
impl io::Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.file.read(buf)
    }
}

#[cfg(not(feature = "calloop"))]
impl io::Read for ReadPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

#[cfg(feature = "calloop")]
impl FromRawFd for ReadPipe {
    unsafe fn from_raw_fd(fd: RawFd) -> ReadPipe {
        ReadPipe {
            file: calloop::generic::Generic::new(
                unsafe { FromRawFd::from_raw_fd(fd) },
                calloop::Interest::READ,
                calloop::Mode::Level,
            ),
        }
    }
}

#[cfg(feature = "calloop")]
impl FromFd for ReadPipe {
    fn from_fd(owned: OwnedFd) -> Self {
        ReadPipe {
            file: calloop::generic::Generic::new(
                owned.into(),
                calloop::Interest::READ,
                calloop::Mode::Level,
            ),
        }
    }
}

#[cfg(not(feature = "calloop"))]
impl FromRawFd for ReadPipe {
    unsafe fn from_raw_fd(fd: RawFd) -> ReadPipe {
        ReadPipe { file: unsafe { FromRawFd::from_raw_fd(fd) } }
    }
}

#[cfg(not(feature = "calloop"))]
impl FromFd for ReadPipe {
    fn from_fd(owned: OwnedFd) -> Self {
        ReadPipe { file: owned.into() }
    }
}

#[cfg(feature = "calloop")]
impl AsRawFd for ReadPipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.file.as_raw_fd()
    }
}

#[cfg(feature = "calloop")]
impl AsFd for ReadPipe {
    fn as_fd(&self) -> wayland_backend::io_lifetimes::BorrowedFd<'_> {
        self.file.file.as_fd()
    }
}

#[cfg(not(feature = "calloop"))]
impl AsRawFd for ReadPipe {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}
#[cfg(not(feature = "calloop"))]

impl AsFd for ReadPipe {
    fn as_fd(&self) -> wayland_backend::io_lifetimes::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

#[cfg(feature = "calloop")]
impl IntoRawFd for ReadPipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.file.into_raw_fd()
    }
}

#[cfg(feature = "calloop")]
impl From<ReadPipe> for OwnedFd {
    fn from(read_pipe: ReadPipe) -> Self {
        read_pipe.file.file.into()
    }
}

#[cfg(not(feature = "calloop"))]
impl IntoRawFd for ReadPipe {
    fn into_raw_fd(self) -> RawFd {
        self.file.into_raw_fd()
    }
}

#[cfg(not(feature = "calloop"))]
impl From<ReadPipe> for OwnedFd {
    fn from(read_pipe: ReadPipe) -> Self {
        read_pipe.file.into()
    }
}

#[cfg(feature = "calloop")]
impl calloop::EventSource for ReadPipe {
    type Event = ();
    type Error = std::io::Error;
    type Metadata = fs::File;
    type Ret = ();

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> std::io::Result<calloop::PostAction>
    where
        F: FnMut((), &mut fs::File),
    {
        self.file.process_events(readiness, token, |_, file| {
            callback((), file);
            Ok(calloop::PostAction::Continue)
        })
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.file.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.file.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> calloop::Result<()> {
        self.file.unregister(poll)
    }
}
