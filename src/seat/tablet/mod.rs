// zwp_tablet_manager_v2
mod manager;
pub use manager::TabletState;

// zwp_tablet_seat_v2
mod seat;
pub use seat::{TabletSeatHandler, TabletSeatData};

// zwp_tablet_v2
mod tablet;
pub use tablet::{TabletEvent, TabletHandler, TabletData, TabletEventList};

// zwp_tablet_tool_v2
mod tool;
pub use tool::{
    TabletToolInitEvent,
    TabletToolInitEventList,
    ToolCapability, ToolType,
    TabletToolEventFrame, TabletToolEvent, TabletToolEventList,
    TabletToolHandler,
    TabletToolData,
};

// zwp_tablet_pad_v2
//pub use seat::{TabletPadData, TabletPadHandler};
mod pad;
pub use pad::{
    TabletPadData,
    TabletPadGroupData,
    TabletPadRingData,
    TabletPadStripData,
};

#[macro_export]
macro_rules! delegate_tablet {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_manager_v2::ZwpTabletManagerV2: $crate::globals::GlobalData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_seat_v2::ZwpTabletSeatV2: $crate::seat::tablet::TabletSeatData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_v2::ZwpTabletV2: $crate::seat::tablet::TabletData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_tool_v2::ZwpTabletToolV2: $crate::seat::tablet::TabletToolData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_v2::ZwpTabletPadV2: $crate::seat::tablet::TabletPadData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2: $crate::seat::tablet::TabletPadGroupData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2: $crate::seat::tablet::TabletPadRingData
        ] => $crate::seat::tablet::TabletState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::tablet::zv2::client::zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2: $crate::seat::tablet::TabletPadStripData
        ] => $crate::seat::tablet::TabletState);
    };
}
