use wayland_client::{
    event_created_child,
    protocol::wl_seat::WlSeat,
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

pub trait Handler: Sized {
    fn tablet_added(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet_seat: &ZwpTabletSeatV2,
        seat: &WlSeat,
        id: ZwpTabletV2,
    );
    fn tool_added(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet_seat: &ZwpTabletSeatV2,
        seat: &WlSeat,
        id: ZwpTabletToolV2,
    );
    ///// TODO; non-functional stub, just so it doesn’t crash if a pad is present (I hope?).
    ///// Nothing more is hooked up for pads.
    //fn pad_added(
    //    &mut self,
    //    conn: &Connection,
    //    qh: &QueueHandle<Self>,
    //    tablet_seat: &ZwpTabletSeatV2,
    //    seat: &WlSeat,
    //    id: ZwpTabletPadV2,
    //);
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Data {
    pub(crate) wl_seat: WlSeat,
}

impl<D> Dispatch<ZwpTabletSeatV2, Data, D>
    for super::TabletManager
where
    D: Dispatch<ZwpTabletSeatV2, Data>
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
        udata: &Data,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zwp_tablet_seat_v2::Event::TabletAdded { id } => {
                data.tablet_added(conn, qh, tablet_seat, &udata.wl_seat, id);
            },
            zwp_tablet_seat_v2::Event::ToolAdded { id } => {
                data.tool_added(conn, qh, tablet_seat, &udata.wl_seat, id);
            },
            zwp_tablet_seat_v2::Event::PadAdded { id } => {
                log::warn!(target: "sctk", "zwp_tablet_seat_v2.pad_added: unimplemented");
                id.destroy();
                //data.pad_added(conn, qh, tablet_seat, &udata.wl_seat, id);
            },
            _ => unreachable!(),
        }
    }
}
