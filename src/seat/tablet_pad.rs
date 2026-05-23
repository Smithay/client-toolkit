#![allow(unused_variables)]

// TODO, stub, just enough so it doesn’t crash if a pad is added (hopefully).
// I, Chris Morgan, hooked all the rest of this stuff up,
// but I don’t have a pad, and don’t see any good dummy device for testing,
// and I won’t benefit from this stuff myself anyway.
// It should be straightforward to implement, but there’s a fair bit of surface area.
// Sorry if you wanted it now.
// Offer to buy me a suitable device, and I’ll be interested.

use wayland_client::{
    event_created_child,
    Connection,
    Dispatch,
    QueueHandle,
};
use wayland_protocols::wp::tablet::zv2::client::{
    // TODO: zwp_tablet_pad_ring_v2, zwp_tablet_pad_strip_v2, zwp_tablet_pad_group_v2.
    zwp_tablet_pad_ring_v2::{self, ZwpTabletPadRingV2},
    zwp_tablet_pad_strip_v2::{self, ZwpTabletPadStripV2},
    zwp_tablet_pad_group_v2::{self, ZwpTabletPadGroupV2, EVT_STRIP_OPCODE, EVT_RING_OPCODE},
    zwp_tablet_pad_v2::{self, ZwpTabletPadV2, EVT_GROUP_OPCODE},
};

#[doc(hidden)]
#[derive(Debug)]
pub struct Data;

impl Data {
    pub fn new() -> Data { Data }
}

// zwp_tablet_pad_v2
// Request: set_feedback
// Request: destroy
// Event: group
// Event: path
// Event: buttons
// Event: done
// Event: button
// Event: enter
// Event: leave
// Event: removed
// Enum: button_state

impl<D> Dispatch<ZwpTabletPadV2, Data, D>
    for super::TabletManager
where
    D: Dispatch<ZwpTabletPadV2, Data>
     + Dispatch<ZwpTabletPadGroupV2, GroupData>
     //+ Handler
     + 'static,
{
    event_created_child!(D, ZwpTabletPadV2, [
        EVT_GROUP_OPCODE => (ZwpTabletPadGroupV2, GroupData),
    ]);

    fn event(
        data: &mut D,
        pad: &ZwpTabletPadV2,
        event: zwp_tablet_pad_v2::Event,
        udata: &Data,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        log::warn!(target: "sctk", "got tablet pad event, unimplemented");
    }
}

impl<D> Dispatch<ZwpTabletPadGroupV2, GroupData, D>
    for super::TabletManager
where
    D: Dispatch<ZwpTabletPadGroupV2, GroupData>
     + Dispatch<ZwpTabletPadRingV2, RingData>
     + Dispatch<ZwpTabletPadStripV2, StripData>
     //+ GroupHandler
     + 'static,
{
    event_created_child!(D, ZwpTabletPadV2, [
        EVT_RING_OPCODE => (ZwpTabletPadRingV2, RingData),
        EVT_STRIP_OPCODE => (ZwpTabletPadStripV2, StripData),
    ]);

    fn event(
        data: &mut D,
        group: &ZwpTabletPadGroupV2,
        event: zwp_tablet_pad_group_v2::Event,
        udata: &GroupData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        log::warn!(target: "sctk", "got tablet pad group event, unimplemented");
    }
}

impl<D> Dispatch<ZwpTabletPadRingV2, RingData, D>
    for super::TabletManager
where
    D: Dispatch<ZwpTabletPadRingV2, RingData>
     //+ RingHandler,
{
    fn event(
        data: &mut D,
        ring: &ZwpTabletPadRingV2,
        event: zwp_tablet_pad_ring_v2::Event,
        udata: &RingData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        log::warn!(target: "sctk", "got tablet pad ring event, unimplemented");
    }
}

impl<D> Dispatch<ZwpTabletPadStripV2, StripData, D>
    for super::TabletManager
where
    D: Dispatch<ZwpTabletPadStripV2, StripData>
     //+ StripHandler,
{
    fn event(
        data: &mut D,
        strip: &ZwpTabletPadStripV2,
        event: zwp_tablet_pad_strip_v2::Event,
        udata: &StripData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        log::warn!(target: "sctk", "got tablet pad strip event, unimplemented");
    }
}

// zwp_tablet_pad_group_v2
// Request: destroy
// Event: buttons
// Event: ring
// Event: strip
// Event: modes
// Event: done
// Event: mode_switch
#[doc(hidden)]
#[derive(Debug)]
pub struct GroupData;

// zwp_tablet_pad_ring_v2
// Request: set_feedback
// Request: destroy
// Event: source
// Event: angle
// Event: stop
// Event: frame
// Enum: source
#[doc(hidden)]
#[derive(Debug)]
pub struct RingData;

// zwp_tablet_pad_strip_v2
// Request: set_feedback
// Request: destroy
// Event: source
// Event: position
// Event: stop
// Event: frame
// Enum: source
#[doc(hidden)]
#[derive(Debug)]
pub struct StripData;
