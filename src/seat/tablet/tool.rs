use std::mem;
use std::sync::Mutex;

use wayland_backend::smallvec::SmallVec;
use wayland_client::{
    protocol::wl_surface::WlSurface,
    Connection,
    Dispatch,
    QueueHandle,
    Proxy,
    WEnum,
};
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_tool_v2::{self, ZwpTabletToolV2},
    zwp_tablet_v2::ZwpTabletV2,
    // TODO: zwp_tablet_pad_ring_v2, zwp_tablet_pad_strip_v2, zwp_tablet_pad_group_v2.
    //zwp_tablet_pad_v2::{self, ZwpTabletPadV2},
};

use super::TabletState;
pub use zwp_tablet_tool_v2::{Capability as ToolCapability, Type as ToolType};

#[derive(Debug)]
pub enum TabletToolInitEvent {
    Type {
        tool_type: ToolType,
    },
    HardwareSerial {
        hardware_serial_hi: u32,
        hardware_serial_lo: u32,
    },
    HardwareIdWacom {
        hardware_id_hi: u32,
        hardware_id_lo: u32,
    },
    Capability {
        capability: ToolCapability,
    },
}

#[derive(Debug)]
pub struct TabletToolEventFrame {
    /// The time of the event with millisecond granularity
    pub time: u32,
    /// All the state changes that have occurred since the previous frame
    pub events: TabletToolEventList,
}

#[derive(Debug)]
pub enum TabletToolEvent {
    /// Notification that this tool is focused on a certain surface.
    ///
    /// This event can be received when the tool has moved from one surface to another,
    /// or when the tool has come back into proximity above the surface.
    ///
    /// If any button is logically down when the tool comes into proximity,
    /// the respective `Button` event is sent after the `ProximityIn` event but within the same frame.
    ProximityIn {
        serial: u32,
        tablet: ZwpTabletV2,
        surface: WlSurface,
    },

    /// Notification that this tool has either left proximity,
    /// or is no longer focused on a certain surface.
    ///
    /// When the tablet tool leaves proximity of the tablet,
    /// button release events are sent for each button that was held down at the time of leaving proximity.
    /// These events are sent before the `ProximityOut` event but within the same frame.
    ///
    /// If the tool stays within proximity of the tablet,
    /// but the focus changes from one surface to another,
    /// a button release event may not be sent until the button is actually released or the tool leaves the proximity of the tablet.
    ProximityOut,

    /// Sent whenever the tablet tool comes in contact with the surface of the tablet.
    ///
    /// If the tool is already in contact with the tablet when entering the input region,
    /// the client owning said region will receive a `ProximityIn` event,
    /// followed by a `Down` event in the same frame.
    ///
    /// Note that this event describes logical contact, not physical contact.
    /// On some devices, a compositor may not consider a tool in logical contact until a minimum physical pressure threshold is exceeded.
    Down {
        serial: u32,
    },

    /// Sent whenever the tablet tool stops making contact with the surface of the tablet,
    /// or when the tablet tool moves out of the input region and the compositor grab (if any) is dismissed.
    ///
    /// If the tablet tool moves out of the input region while in contact with the surface of the tablet and the compositor does not have an ongoing grab on the surface,
    /// the client owning said region will receive an `Up` event,
    /// followed by a `ProximityOut` event in the same frame.
    /// If the compositor has an ongoing grab on this device,
    /// this event sequence is sent whenever the grab is dismissed in the future.
    ///
    /// Note that this event describes logical contact, not physical contact.
    /// On some devices, a compositor may not consider a tool out of logical contact until physical pressure falls below a specific threshold.
    Up,

    /// Sent whenever a tablet tool moves.
    Motion {
        x: f64,
        y: f64,
    },

    /// Sent whenever the pressure axis on a tool changes.
    /// The value of this event is normalized to a value between 0 and 65535.
    ///
    /// Note that pressure may be nonzero even when a tool is not in logical contact.
    /// See the Down and Up events for more details.
    Pressure {
        pressure: u16,
    },

    /// Sent whenever the distance axis on a tool changes.
    /// The value of this event is normalized to a value between 0 and 65535.
    ///
    /// Note that distance may be nonzero even when a tool is not in logical contact.
    /// See the Down and Up events for more details.
    Distance {
        distance: u16,
    },

    /// Sent whenever one or both of the tilt axes on a tool change.
    /// Each tilt value is in degrees, relative to the z-axis of the tablet.
    /// The angle is positive when the top of a tool tilts along the positive x or y axis.
    Tilt {
        tilt_x: f64,
        tilt_y: f64,
    },

    /// Sent whenever the z-rotation axis on the tool changes.
    /// The rotation value is in degrees clockwise from the tool's logical neutral position.
    Rotation {
        degrees: f64,
    },

    /// Sent whenever the slider position on the tool changes.
    /// The value is normalized between -65535 and 65535,
    /// with 0 as the logical neutral position of the slider.
    Slider {
        position: i32,
    },

    /// Sent whenever the wheel on the tool emits an event.
    /// This event contains two values for the same axis change.
    ///
    /// Clients should choose either value and avoid mixing degrees and clicks.
    /// The compositor may accumulate values smaller than a logical click and emulate click events when a certain threshold is met.
    /// Thus, wheel events with non-zero `clicks` values may have different `degrees` values.
    Wheel {
        /// The wheel delta in degrees.
        ///
        /// This value is in the same orientation as the `wl_pointer.vertical_scroll` axis.
        degrees: f64,
        /// The wheel delta in discrete clicks.
        ///
        /// This value is in discrete logical clicks of the mouse wheel,
        /// and may be zero if the movement of the wheel was less than one logical click.
        clicks: i32,
    },

    /// Sent whenever a button on the tool is pressed or released.
    ///
    /// If a button is held down when the tool moves in or out of proximity,
    /// button events are generated by the compositor.
    /// See `ProximityIn` and `ProximityOut` for details.
    Button {
        serial: u32,
        /// The button whose state has changed
        button: u32,
        /// Whether the button was pressed (`true`) or released (`false`)
        pressed: bool,
    },
}

pub trait TabletToolHandler: Sized {
    /// This is fired at the time of the `zwp_tablet_tool_v2.done` event,
    /// and coalesces any `type`, `hardware_serial`, `hardware_serial_wacom` and `capability` events that precede it.
    fn init_done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet: &ZwpTabletToolV2,
        events: TabletToolInitEventList,
    );

    /// Sent when the tablet has been removed from the system.
    /// When a tablet is removed, some tools may be removed.
    ///
    /// This method is responsible for running `tablet.destroy()`.
    fn removed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet: &ZwpTabletToolV2,
    );

    /// A series of axis and/or button updates have been received from the tablet.
    /// All the events within this frame should be considered one hardware event.
    fn tablet_tool_frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet: &ZwpTabletToolV2,
        frame: TabletToolEventFrame,
    );
}

#[doc(hidden)]
#[derive(Debug)]
pub struct TabletToolData {
    //seat: WlSeat,
    //tablet_seat: ZwpTabletToolSeatV2,
    inner: Mutex<TabletToolDataInner>,
}

impl TabletToolData {
    pub fn new() -> Self {
        Self { inner: Default::default() }
    }
}

pub type TabletToolInitEventList = Vec<TabletToolInitEvent>;
pub type TabletToolEventList = SmallVec<[TabletToolEvent; 3]>;

#[derive(Debug, Default)]
struct TabletToolDataInner {
    /// List of pending init-time events, flushed when a `done` event comes in,
    /// after which it will be perpetually empty.
    pending_init: TabletToolInitEventList,

    /// List of pending events, flushed when a `frame` event comes in.
    pending_frame: TabletToolEventList,
}

impl<D> Dispatch<ZwpTabletToolV2, TabletToolData, D>
    for TabletState
where
    D: Dispatch<ZwpTabletToolV2, TabletToolData> + TabletToolHandler,
{
    fn event(
        data: &mut D,
        tool: &ZwpTabletToolV2,
        event: zwp_tablet_tool_v2::Event,
        udata: &TabletToolData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let mut guard = udata.inner.lock().unwrap();
        match event {

            // Initial burst of static description events
            // (one Type,
            // zero or one HardwareSerial,
            // zero or one HardwareIdWacom,
            // zero or more Capability,
            // then finished with Done).

            zwp_tablet_tool_v2::Event::Type { tool_type } => {
                guard.pending_init.push(TabletToolInitEvent::Type {
                    tool_type: match tool_type {
                        WEnum::Value(tool_type) => tool_type,
                        WEnum::Unknown(unknown) => {
                            log::warn!(target: "sctk", "{}: invalid tablet tool type: {:x}", tool.id(), unknown);
                            return;
                        },
                    },
                });
            },
            zwp_tablet_tool_v2::Event::HardwareSerial { hardware_serial_hi, hardware_serial_lo } => {
                guard.pending_init.push(TabletToolInitEvent::HardwareSerial { hardware_serial_hi, hardware_serial_lo });
            },
            zwp_tablet_tool_v2::Event::HardwareIdWacom { hardware_id_hi, hardware_id_lo } => {
                guard.pending_init.push(TabletToolInitEvent::HardwareIdWacom { hardware_id_hi, hardware_id_lo });
            },
            zwp_tablet_tool_v2::Event::Capability { capability } => {
                guard.pending_init.push(TabletToolInitEvent::Capability {
                    capability: match capability {
                        WEnum::Value(capability) => capability,
                        WEnum::Unknown(unknown) => {
                            log::warn!(target: "sctk", "{}: invalid tablet tool capability: {:x}", tool.id(), unknown);
                            return;
                        },
                    },
                });
            },
            zwp_tablet_tool_v2::Event::Done => {
                let events = mem::take(&mut guard.pending_init);
                drop(guard);
                data.init_done(conn, qh, tool, events);
            },

            // Destruction

            zwp_tablet_tool_v2::Event::Removed => {
                data.removed(conn, qh, tool);
            },

            // Burst of frame data events
            // (one or more of ProximityIn, ProximityOut, Down, Up, Motion,
            // Pressure, Distance, Tilt, Rotation, Slider, Wheel, Button,
            // with some restrictions on ordering and such;
            // then finished with Frame).

            zwp_tablet_tool_v2::Event::ProximityIn { serial, tablet, surface } => {
                guard.pending_frame.push(TabletToolEvent::ProximityIn { serial, tablet, surface });
            },
            zwp_tablet_tool_v2::Event::ProximityOut => {
                guard.pending_frame.push(TabletToolEvent::ProximityOut);
            },
            zwp_tablet_tool_v2::Event::Down { serial } => {
                guard.pending_frame.push(TabletToolEvent::Down { serial });
            },
            zwp_tablet_tool_v2::Event::Up => {
                guard.pending_frame.push(TabletToolEvent::Up);
            },
            zwp_tablet_tool_v2::Event::Motion { x, y } => {
                guard.pending_frame.push(TabletToolEvent::Motion { x, y });
            },
            zwp_tablet_tool_v2::Event::Pressure { pressure } => {
                // “The value of this event is normalized to a value between 0 and 65535.”
                // But the wayland Wire format only supports 32-bit integers, so we cast it.
                // <https://wayland.freedesktop.org/docs/html/ch04.html#:~:text=xml.-,Wire%20Format>.
                guard.pending_frame.push(TabletToolEvent::Pressure { pressure: pressure as u16 });
            },
            zwp_tablet_tool_v2::Event::Distance { distance } => {
                // Same deal, “normalized to a value between 0 and 65535”.
                guard.pending_frame.push(TabletToolEvent::Distance { distance: distance as u16 });
            },
            zwp_tablet_tool_v2::Event::Tilt { tilt_x, tilt_y } => {
                guard.pending_frame.push(TabletToolEvent::Tilt { tilt_x, tilt_y });
            },
            zwp_tablet_tool_v2::Event::Rotation { degrees } => {
                guard.pending_frame.push(TabletToolEvent::Rotation { degrees });
            },
            zwp_tablet_tool_v2::Event::Slider { position } => {
                // This one is “normalized between -65535 and 65535”, 17 bits, so it stays i32.
                guard.pending_frame.push(TabletToolEvent::Slider { position });
            },
            zwp_tablet_tool_v2::Event::Wheel { degrees, clicks } => {
                guard.pending_frame.push(TabletToolEvent::Wheel { degrees, clicks });
            },
            zwp_tablet_tool_v2::Event::Button { serial, button, state } => {
                guard.pending_frame.push(TabletToolEvent::Button {
                    serial,
                    button,
                    pressed: match state {
                        WEnum::Value(zwp_tablet_tool_v2::ButtonState::Pressed) => true,
                        WEnum::Value(zwp_tablet_tool_v2::ButtonState::Released) => false,
                        WEnum::Value(_) => unreachable!(),
                        WEnum::Unknown(unknown) => {
                            log::warn!(target: "sctk", "{}: invalid tablet tool button state: {:x}", tool.id(), unknown);
                            return;
                        },
                    },
                });
            },
            zwp_tablet_tool_v2::Event::Frame { time } => {
                // TODO: pass ownership of the events list.
                // I was copying what something else did around here,
                // but since we don’t do anything more with them,
                // it would be better to cede ownership.
                // The only issue is that it exposes SmallVec what is nicer an implementation type.
                let events = mem::take(&mut guard.pending_frame);
                drop(guard);
                data.tablet_tool_frame(conn, qh, tool, TabletToolEventFrame {
                    time,
                    events,
                });
            },
            _ => unreachable!(),
        }
    }
}
