use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::wl_seat::WlSeat,
    Connection,
    Dispatch,
    QueueHandle,
};

use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2::{self, ZwpTabletManagerV2},
    zwp_tablet_seat_v2::ZwpTabletSeatV2,
};

use crate::globals::GlobalData;

#[derive(Debug)]
pub struct TabletManager {
    tablet_manager: ZwpTabletManagerV2,
}

impl TabletManager {
    /// Bind `zwp_tablet_manager_v2` global, if it exists
    pub fn bind<State>(
        globals: &GlobalList,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, BindError>
    where
        State: Dispatch<ZwpTabletManagerV2, GlobalData> + 'static,
    {
        let tablet_manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self { tablet_manager })
    }

    pub(crate) fn from_existing(tablet_manager: ZwpTabletManagerV2) -> Self {
        Self { tablet_manager }
    }

    pub fn get_tablet_seat<D>(
        &self,
        seat: &WlSeat,
        qh: &QueueHandle<D>,
    ) -> ZwpTabletSeatV2
    where
        D: Dispatch<ZwpTabletSeatV2, ()> + 'static,
    {
        self.tablet_manager.get_tablet_seat(seat, qh, ())
    }
}

impl<D> Dispatch<ZwpTabletManagerV2, GlobalData, D>
    for TabletManager
where
    D: Dispatch<ZwpTabletManagerV2, GlobalData>,
{
    fn event(
        _data: &mut D,
        _manager: &ZwpTabletManagerV2,
        _event: zwp_tablet_manager_v2::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

#[macro_export]
macro_rules! delegate_tablet {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_manager_v2::ZwpTabletManagerV2: $crate::globals::GlobalData
        ] => $crate::seat::TabletManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_seat_v2::ZwpTabletSeatV2: ()
        ] => $crate::seat::TabletManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_v2::ZwpTabletV2: $crate::seat::tablet::Data
        ] => $crate::seat::TabletManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_tool_v2::ZwpTabletToolV2: $crate::seat::tablet_tool::Data
        ] => $crate::seat::TabletManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_v2::ZwpTabletPadV2: $crate::seat::tablet_pad::Data
        ] => $crate::seat::TabletManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2: $crate::seat::tablet_pad::GroupData
        ] => $crate::seat::TabletManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2: $crate::seat::tablet_pad::RingData
        ] => $crate::seat::TabletManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2: $crate::seat::tablet_pad::StripData
        ] => $crate::seat::TabletManager);
    };
}
