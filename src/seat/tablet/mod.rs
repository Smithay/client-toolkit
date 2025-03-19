use wayland_client::{
    globals::GlobalList,
    protocol::wl_seat::WlSeat,
    Connection,
    Dispatch,
    QueueHandle,
};

use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2::{self, ZwpTabletManagerV2},
    zwp_tablet_seat_v2::ZwpTabletSeatV2,
};

use crate::{error::GlobalError, globals::GlobalData, registry::GlobalProxy};

pub mod seat;
pub mod tablet;
pub mod tool;
pub mod pad;

#[derive(Debug)]
pub struct TabletState {
    tablet_manager: GlobalProxy<ZwpTabletManagerV2>,
}

impl TabletState {
    /// Bind `zwp_tablet_manager_v2` global, if it exists
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ZwpTabletManagerV2, GlobalData> + 'static,
    {
        Self {
            tablet_manager: GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData)),
        }
    }

    pub fn get_tablet_seat<D>(
        &self,
        seat: &WlSeat,
        qh: &QueueHandle<D>,
    ) -> Result<ZwpTabletSeatV2, GlobalError>
    where
        D: Dispatch<ZwpTabletSeatV2, seat::TabletSeatData> + 'static,
    {
        let udata = seat::TabletSeatData { wl_seat: seat.clone() };
        Ok(self.tablet_manager.get()?.get_tablet_seat(seat, qh, udata))
    }
}

impl<D> Dispatch<ZwpTabletManagerV2, GlobalData, D>
    for TabletState
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
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_seat_v2::ZwpTabletSeatV2: $crate::seat::tablet::seat::TabletSeatData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_v2::ZwpTabletV2: $crate::seat::tablet::tablet::TabletData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_tool_v2::ZwpTabletToolV2: $crate::seat::tablet::tool::TabletToolData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_v2::ZwpTabletPadV2: $crate::seat::tablet::pad::TabletPadData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2: $crate::seat::tablet::pad::TabletPadGroupData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2: $crate::seat::tablet::pad::TabletPadRingData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2: $crate::seat::tablet::pad::TabletPadStripData
        ] => $crate::seat::tablet::TabletState);
    };
}
