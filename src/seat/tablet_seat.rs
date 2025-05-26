use wayland_client::{
    event_created_child,
    Connection,
    Dispatch,
    QueueHandle,
};
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_seat_v2::{self, ZwpTabletSeatV2, EVT_TABLET_ADDED_OPCODE, EVT_TOOL_ADDED_OPCODE, EVT_PAD_ADDED_OPCODE},
    zwp_tablet_tool_v2::ZwpTabletToolV2,
    zwp_tablet_v2::ZwpTabletV2,
    zwp_tablet_pad_v2::ZwpTabletPadV2,
};

/// Handler for the tablet seat.
///
/// The `*_added` methods announce the creation of objects before they’re ready for use.
/// If you might have multiple seats and want to associate devices with their tablet seat,
/// then you can implement them, but otherwise you can just leave them blank.
///
/// What you *actually* care about is the corresponding handler’s `info` method.
/// That’s when it’s ready to use.
#[allow(unused_variables)]  // ← For all the trait method arguments
pub trait Handler: Sized {
    /// A tablet has been added.
    ///
    /// [`tablet::Handler::info`](super::tablet::Handler::info) will be called when it is ready for use.
    fn tablet_added(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet_seat: &ZwpTabletSeatV2,
        id: ZwpTabletV2,
    ) {}

    /// A tablet tool has been added.
    ///
    /// [`tablet_tool::Handler::info`](super::tablet_tool::Handler::info) will be called when it is ready for use.
    fn tool_added(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet_seat: &ZwpTabletSeatV2,
        id: ZwpTabletToolV2,
    ) {}

    /// A tablet pad has been added.
    ///
    /// [`tablet_pad::Handler::info`](super::tablet_pad::Handler::info) will be called when it is ready for use.
    fn pad_added(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet_seat: &ZwpTabletSeatV2,
        id: ZwpTabletPadV2,
    ) {}
}

impl<D> Dispatch<ZwpTabletSeatV2, (), D>
    for super::TabletManager
where
    D: Dispatch<ZwpTabletSeatV2, ()>
     + Dispatch<ZwpTabletV2, super::tablet::Data>
     + Dispatch<ZwpTabletToolV2, super::tablet_tool::Data>
     + Dispatch<ZwpTabletPadV2, super::tablet_pad::Data>
     + Handler
     + 'static,
{
    event_created_child!(D, ZwpTabletSeatV2, [
        EVT_TABLET_ADDED_OPCODE => (ZwpTabletV2, super::tablet::Data::new()),
        EVT_TOOL_ADDED_OPCODE => (ZwpTabletToolV2, super::tablet_tool::Data::new()),
        EVT_PAD_ADDED_OPCODE => (ZwpTabletPadV2, super::tablet_pad::Data::new()),
    ]);

    fn event(
        data: &mut D,
        tablet_seat: &ZwpTabletSeatV2,
        event: zwp_tablet_seat_v2::Event,
        _udata: &(),
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zwp_tablet_seat_v2::Event::TabletAdded { id } => {
                data.tablet_added(conn, qh, tablet_seat, id);
            },
            zwp_tablet_seat_v2::Event::ToolAdded { id } => {
                data.tool_added(conn, qh, tablet_seat, id);
            },
            zwp_tablet_seat_v2::Event::PadAdded { id } => {
                data.pad_added(conn, qh, tablet_seat, id);
            },
            _ => unreachable!(),
        }
    }
}
