use std::collections::{hash_map, HashMap};
use std::fmt;
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

pub use zwp_tablet_tool_v2::{Capability, Type, Event};

#[derive(Debug)]
pub enum InitEvent {
    Type(Type),
    HardwareSerial {
        hi: u32,
        lo: u32,
    },
    HardwareIdWacom {
        hi: u32,
        lo: u32,
    },
    Capability(Capability),
}

// Just a named tuple.
/// A hardware identifier, just two `u32`s with names `hi` and `lo`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct HardwareSerialOrId {
    pub hi: u32,
    pub lo: u32,
}

bitflags::bitflags! {
    /// What the tool is capable of, beyond basic X/Y coordinates.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Capabilities: u8 {
        /// Whether the tool supports tilt.
        const TILT     = 0b00000001;
        /// Whether the tool supports pressure.
        const PRESSURE = 0b00000010;
        /// Whether the tool can track its distance from the surface.
        const DISTANCE = 0b00000100;
        /// Whether the tool can measure z-axis rotation.
        const ROTATION = 0b00001000;
        /// Whether the tool has a slider.
        const SLIDER   = 0b00010000;
        /// Whether the tool has a wheel.
        const WHEEL    = 0b00100000;

        // Reserve them, but don’t make them part of the public interface.
        const _        = 0b01000000;
        const _        = 0b10000000;
    }
}
const HARDWARE_SERIAL:   Capabilities = Capabilities::from_bits_retain(0b01000000);
const HARDWARE_ID_WACOM: Capabilities = Capabilities::from_bits_retain(0b10000000);

/// Static information about the tool and its capabilities.
#[derive(Clone, PartialEq, Eq)]
pub struct Description {
    // Wish this was #[repr(u8)]… it’s wasting four bytes.
    r#type: Type,
    // These are really Option<_>, but I squeezed their None discriminant into capabilities,
    // as it had two spare bits. This saves eight bytes. You’re welcome. 😛
    hardware_serial: HardwareSerialOrId,
    hardware_id_wacom: HardwareSerialOrId,
    // Could have used bitflags here—it is already a dep—but we don’t need its complexity.
    // Only real loss from this simplicity is meaningful Debug.
    capabilities: Capabilities,
}

// Manual to Option<…> hardware_serial and hardware_id_wacom.
impl fmt::Debug for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Description")
            .field("r#type", &self.r#type)
            .field("hardware_serial", &self.hardware_serial())
            .field("hardware_id_wacom", &self.hardware_id_wacom())
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl Default for Description {
    fn default() -> Description {
        Description {
            // I get the impression type is mandatory,
            // so this should be overwritten with the correct value.
            // But if not… meh, Pen would be the default anyway.
            r#type: Type::Pen,
            hardware_serial: HardwareSerialOrId { hi: 0, lo: 0 },
            hardware_id_wacom: HardwareSerialOrId { hi: 0, lo: 0 },
            capabilities: Capabilities::empty(),
        }
    }
}

impl Description {
    /// The type of tool.
    pub fn r#type(&self) -> Type { self.r#type }

    /// What the hardware serial number of the tool is, if any.
    pub fn hardware_serial(&self) -> Option<HardwareSerialOrId> {
        if self.capabilities.contains(HARDWARE_SERIAL) {
            Some(self.hardware_serial)
        } else {
            None
        }
    }

    /// What the Wacom hardware ID of the tool is, if any.
    pub fn hardware_id_wacom(&self) -> Option<HardwareSerialOrId> {
        if self.capabilities.contains(HARDWARE_ID_WACOM) {
            Some(self.hardware_id_wacom)
        } else {
            None
        }
    }

    /// What the tool is capable of, beyond basic X/Y coordinates.
    pub fn capabilities(&self) -> Capabilities { self.capabilities.clone() }
    /// Whether the tool supports tilt.
    pub fn supports_tilt(&self)     -> bool { self.capabilities.contains(Capabilities::TILT) }
    /// Whether the tool supports pressure.
    pub fn supports_pressure(&self) -> bool { self.capabilities.contains(Capabilities::PRESSURE) }
    /// Whether the tool can track its distance from the surface.
    pub fn supports_distance(&self) -> bool { self.capabilities.contains(Capabilities::DISTANCE) }
    /// Whether the tool can measure z-axis rotation.
    pub fn supports_rotation(&self) -> bool { self.capabilities.contains(Capabilities::ROTATION) }
    /// Whether the tool has a slider.
    pub fn supports_slider(&self)   -> bool { self.capabilities.contains(Capabilities::SLIDER) }
    /// Whether the tool has a wheel.
    pub fn supports_wheel(&self)    -> bool { self.capabilities.contains(Capabilities::WHEEL) }
}

#[derive(Debug)]
pub struct InfoAndState {
    pub description: Description,
    /// The time the last frame was sent,
    /// or zero if no frames have come yet.
    pub last_frame_time: u32,
    /// The current state of the tool, if in proximity.
    pub state: Option<State>,
}

impl InfoAndState {
    /// Get the pressure according to the web’s Pointer Events API:
    /// scaled in the range \[0, 1\],
    /// and set to 0.5 when down if pressure isn’t supported.
    pub fn pressure_web(&self) -> f64 {
        match (self.description.supports_pressure(), &self.state) {
            (true, &Some(State { pressure, .. })) => pressure as f64 / 65535.0,
            (false, Some(State { down_serial: Some(_), .. })) => 0.5,
            _ => 0.0,
        }
    }
}

impl From<Description> for InfoAndState {
    fn from(description: Description) -> InfoAndState {
        InfoAndState {
            description,
            last_frame_time: 0,
            state: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct Tools {
    map: HashMap<ZwpTabletToolV2, InfoAndState>,
}

impl Tools {
    pub fn new() -> Tools {
        Tools::default()
    }

    pub fn get(&self, tool: &ZwpTabletToolV2) -> Option<&InfoAndState> {
        self.map.get(tool)
    }

    pub fn get_mut(&mut self, tool: &ZwpTabletToolV2) -> Option<&mut InfoAndState> {
        self.map.get_mut(tool)
    }

    pub fn add(&mut self, tool: ZwpTabletToolV2, description: Description) {
        self.map.insert(tool.clone(), InfoAndState {
            description,
            last_frame_time: 0,
            state: None,
        });
    }

    pub fn remove(&mut self, tool: &ZwpTabletToolV2) {
        self.map.remove(tool);
    }

    /// Apply the events to the record state.
    ///
    /// Returns it in order to save a superfluous [`.get()`](Self::get) call.
    pub fn ingest_frame(&mut self, tool: &ZwpTabletToolV2, events: &[Event]) -> &InfoAndState {
        let mut events = events.into_iter();
        let ias = self.map.get_mut(tool).unwrap();
        let state = ias.state.get_or_insert_with(|| {
            let Some(Event::ProximityIn { serial, tablet, surface }) = events.next()
            else {
                panic!("First zwp_tablet_tool_v2 frame didn’t start with a proximity_in event");
            };
            State::from_proximity_in(*serial, tablet.clone(), surface.clone())
        });
        let Some(Event::Frame { time }) = events.next_back() else { unreachable!() };
        ias.last_frame_time = *time;

        for event in events {
            state.apply_event(&event);
            if let Event::ProximityOut = event {
                ias.state = None;
                // Given that a frame is supposed to represent a single hardware event,
                // I think you can fairly say it’d be mad to proximity_out and
                // immediately proximity_in in the same frame.
                // So I think we’re OK to just break.
                break;
            }
        }
        &*ias
    }

    pub fn values(&self) -> hash_map::Values<'_, ZwpTabletToolV2, InfoAndState> {
        self.map.values()
    }
}

impl<'a> IntoIterator for &'a Tools {
    type Item = (&'a ZwpTabletToolV2, &'a InfoAndState);
    type IntoIter = hash_map::Iter<'a, ZwpTabletToolV2, InfoAndState>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

// TODO: this isn’t the way, but what is?
impl Handler for Tools {
    fn init_done(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        tool: &ZwpTabletToolV2,
        description: Description,
    ) {
        self.add(tool.clone(), description);
    }

    fn removed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        tool: &ZwpTabletToolV2,
    ) {
        self.map.remove(tool);
    }

    fn tablet_tool_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        tool: &ZwpTabletToolV2,
        events: &[Event],
    ) {
        self.ingest_frame(tool, &events);
    }
}

/// The current state of the tool, while in proximity.
///
/// For many applications, when you receive a frame,
/// you don’t so much care about the events,
/// as about capturing the tool’s current total state.
///
/// This lets you do that.
///
/// Caveats:
///
/// - Although the wheel information is captured as completely as possible here,
///   it should probably be perceived through the event stream;
///   tablet tool wheels are inherently delta-based,
///   so error would accumulate if you try to treat them absolutely.
///
/// - This only gives a limited view of buttons,
///   only capturing BTN_STYLUS, BTN_STYLUS2 and BTN_STYLUS3 pressed state,
///   not serials or even whether they’ve ever been seen.
///   This is because it makes the interface a good deal nicer,
///   takes less effort to implement efficiently,
///   other buttons are extremely improbable
///   (nothing but stylus inputs have been available since the early-/mid-2010s),
///   and if you care about serials or other buttons you will surely prefer events.
//
// At 176 bytes on a 64-bit platform, this is much larger than it fundamentally *need* be.
// With effort, you could losslessly encode all but wheel in less than 37 bytes,
// and with negligible loss you could shrink it to 29 bytes.
// But we’re trying to make it usefully accessible, not packed ultra-tight.
// So it’s mildly painfully wasteful instead.
#[derive(Debug, Clone)]
pub struct State {
    // ProximityIn
    /// The `Event::ProximityIn.serial` value,
    /// needed for [`ZwpTabletToolV2::set_cursor`] requests.
    pub proximity_in_serial: u32,
    pub tablet: ZwpTabletV2,
    pub surface: WlSurface,

    // Down, Up
    /// Whether the tool is in logical contact or not.
    ///
    /// When down, it carries the serial of the last down event.
    pub down_serial: Option<u32>,

    // Motion
    /// The horizontal position, in surface coordinates.
    pub x: f64,
    /// The vertical position, in surface coordinates.
    pub y: f64,

    // Pressure
    /// The pressure, scaled from 0–65535, if capable (else 0).
    pub pressure: u16,

    // Distance
    /// The pressure, scaled from 0–65535, if capable (else 0).
    pub distance: u16,

    // Tilt
    /// The pen’s tilt in the positive X axis, in degrees (−90 to 90), if capable.
    pub tilt_x: f64,
    /// The pen’s tilt in the positive X axis, in degrees (−90 to 90), if capable.
    pub tilt_y: f64,

    // Rotation
    /// The z-axis rotation of the tool, if capable (else 0.0).
    /// The rotation value is in degrees clockwise from the tool's logical neutral position.
    pub rotation_degrees: f64,

    // Slider
    /// The slider position, if capable (else 0).
    /// The value is normalized between -65535 and 65535,
    /// with 0 as the logical neutral position of the slider.
    pub slider_position: i32,

    // Wheel
    /// The wheel delta in degrees.
    ///
    /// See [Event::Wheel] for more information,
    /// and guidance on using wheel values.
    ///
    /// You will probably prefer to consume events,
    /// rather than consuming this value.
    pub wheel_degrees: f64,
    /// The wheel delta in discrete clicks.
    ///
    /// See [Event::Wheel] for more information,
    /// and guidance on using wheel values.
    ///
    /// You will probably prefer to consume events,
    /// rather than consuming this value.
    pub wheel_clicks: i32,

    // Button
    /// Whether [`BTN_STYLUS`] is pressed.
    pub stylus_button_1_pressed: bool,
    /// Whether [`BTN_STYLUS2`] is pressed.
    pub stylus_button_2_pressed: bool,
    /// Whether [`BTN_STYLUS3`] is pressed.
    pub stylus_button_3_pressed: bool,
}

impl State {
    pub fn is_down(&self) -> bool {
        self.down_serial.is_some()
    }
}

pub fn button_state_to_bool(state: WEnum<zwp_tablet_tool_v2::ButtonState>) -> bool {
    matches!(state, WEnum::Value(zwp_tablet_tool_v2::ButtonState::Pressed))
}

impl State {
    pub fn from_proximity_in(serial: u32, tablet: ZwpTabletV2, surface: WlSurface) -> State {
        State {
            proximity_in_serial: serial,
            tablet,
            surface,
            // Initialise the rest to the most meaningful values possible.
            down_serial: None,
            x: 0.0,
            y: 0.0,
            pressure: 0,
            distance: 0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            rotation_degrees: 0.0,
            slider_position: 0,
            wheel_degrees: 0.0,
            wheel_clicks: 0,
            stylus_button_1_pressed: false,
            stylus_button_2_pressed: false,
            stylus_button_3_pressed: false,
        }
    }

    /// Apply the change described in an event to this state.
    ///
    /// Certain events are ignored:
    ///
    /// - Static description events (e.g. `Type` and `Done`) aren’t applicable.
    /// - `ProximityOut` invalidates this entirely and needs to be applied at a higher level.
    /// - `ProximityIn` should already have been consumed by the caller
    ///   (it was what led to creating this object).
    /// - `Frame` should be consumed by the caller
    ///   (you can’t store frame time inside here,
    ///   since it will be deleted by `ProximityOut`,
    ///   but you might still care about its frame time).
    ///
    /// For clarity:
    /// `ProximityOut` is expected to be passed here (at present), though it’s no-op,
    /// but `ProximityIn` and `Frame` should both have been consumed.
    ///
    /// Also, `Button` is ignored for anything other than the stylus buttons,
    /// and button serials are discarded.
    pub fn apply_event(&mut self, event: &Event) {
        match *event {
            Event::ProximityOut => {
                // This invalidates `self`, and must be handled by the caller.
            },
            Event::Down { serial } => {
                self.down_serial = Some(serial);
            },
            Event::Up => {
                self.down_serial = None;
            },
            Event::Motion { x, y } => {
                self.x = x;
                self.y = y;
            },
            Event::Pressure { pressure } => {
                // “The value of this event is normalized to a value between 0 and 65535.”
                // But the Wayland wire format only supports 32-bit integers,
                // so we cast it here. We might as well, I reckon.
                self.pressure = pressure as u16;
            },
            Event::Distance { distance } => {
                // Same deal, “normalized to a value between 0 and 65535”.
                self.distance = distance as u16;
            },
            Event::Tilt { tilt_x, tilt_y } => {
                self.tilt_x = tilt_x;
                self.tilt_y = tilt_y;
            },
            Event::Rotation { degrees } => {
                self.rotation_degrees = degrees;
            },
            Event::Slider { position } => {
                // This one is “normalized between -65535 and 65535”, 17 bits, so it stays i32.
                self.slider_position = position;
            },
            Event::Wheel { degrees, clicks } => {
                // These ones use += because they’re deltas, unlike the rest.
                self.wheel_degrees += degrees;
                self.wheel_clicks += clicks;
            },
            Event::Button { serial: _, button: BTN_STYLUS, state } => {
                self.stylus_button_1_pressed = button_state_to_bool(state);
            },
            Event::Button { serial: _, button: BTN_STYLUS2, state } => {
                self.stylus_button_2_pressed = button_state_to_bool(state);
            },
            Event::Button { serial: _, button: BTN_STYLUS3, state } => {
                self.stylus_button_3_pressed = button_state_to_bool(state);
            },
            Event::Button { .. } => {
                // Ignored; see doc comment.
            },
            Event::ProximityIn { .. } | Event::Frame { .. } | _ => {
                // Should be unreachable; see doc comment.
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    /// Button state: `true` if pressed, `false` if released.
    pub pressed: bool,
    /// The serial for the most recent state change.
    pub serial: u32,
}

// From linux/input-event-codes.h.
// There isn’t a good definition of which buttons tablet tools might use,
// which I think is why it wasn’t made an enum like type was.
// (The mailing list thread about this stuff seems a bit inconclusive;
// I should ask follow-up questions.
// <https://lists.freedesktop.org/archives/wayland-devel/2016-November/031734.html>)
//
// Most tools are stylus variants, and are likely to use these if they have buttons:
/// The first button on a stylus.
pub const BTN_STYLUS: u32 = 0x14b;
/// The second button on a stylus.
pub const BTN_STYLUS2: u32 = 0x14c;
/// The third button on a stylus.
pub const BTN_STYLUS3: u32 = 0x149;
//
// Mouse toools will use BTN_LEFT et al.; but they’re already exported from crate::seat::pointer,
// and do mouse tools even exist any more?
// Finger, I have no idea, and I bet they just come through as touchscreen anyway.
// (Seriously, pen and eraser are probably the only two types of tool that will ever be used.)
// BTN_0 to BTN_9, I don’t particularly expect such things, that sounds like a pad, not a tool.
// So, I’ll leave it at just these three stylus constants.

pub trait Handler: Sized {
    /// This is fired at the time of the `zwp_tablet_tool_v2.done` event,
    /// and collects any preceding `name`, `id` and `path` `type`, `hardware_serial`,
    /// `hardware_serial_wacom` and `capability` events into a [`Description`].
    fn init_done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet: &ZwpTabletToolV2,
        description: Description,
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
    /// The last event in the list passed will always be a `Frame` event,
    /// and there will only be that one frame.
    fn tablet_tool_frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet: &ZwpTabletToolV2,
        events: &[Event],
    );
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Data {
    //seat: WlSeat,
    //tablet_seat: ZwpTabletToolSeatV2,
    inner: Mutex<DataInner>,
}

impl Data {
    pub fn new() -> Self {
        Self { inner: Default::default() }
    }
}

#[derive(Debug, Default)]
struct DataInner {
    /// An accumulation of pending init-time events, flushed when a `done` event comes in,
    /// after which it will be perpetually empty.
    description: Description,

    /// List of pending events, flushed when a `frame` event comes in.
    pending: EventList,
}

/// A list of data frame events.
//
// Whereas PointerHandler gets given &[PointerEvent],
// I’ve chosen to expose SmallVec here for practicality,
// and to skip unnecessary potential clones of `ProximityIn`’s tablet and surface.
// If we wanted to keep this but not expose SmallVec,
// we’d newtype smallvec::Drain, duplicating all its surface area.
// I might have done that, but when it doesn’t implement as_slice like Vec’s,
// I become less enthusiastic.
// I’m not convinced it’s worth the bother.
//
// Explanation on chosen sizing:
//  • There will always be at least two events: one axis change, and a Frame.
//  • Three is fundamentally common, when you have proximity and tip events.
//  • Pressure will be almost ubiquitous, so add one for that.
//  • Tilt will be very common too.
// My opinion, unmeasured save by eyeballing an event stream on a tilt+pressure-capable pen,
// is that four is probably the sweet spot.
// Ability to tweak that number would be a good reason to encapsulate this…!
pub type EventList = SmallVec<[Event; 4]>;

impl<D> Dispatch<ZwpTabletToolV2, Data, D>
    for super::TabletManager
where
    D: Dispatch<ZwpTabletToolV2, Data> + Handler,
{
    fn event(
        data: &mut D,
        tool: &ZwpTabletToolV2,
        event: zwp_tablet_tool_v2::Event,
        udata: &Data,
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
                guard.description.r#type = match tool_type {
                    WEnum::Value(tool_type) => tool_type,
                    WEnum::Unknown(unknown) => {
                        log::warn!(target: "sctk", "{}: invalid tablet tool type: {:x}", tool.id(), unknown);
                        return;
                    },
                };
            },
            zwp_tablet_tool_v2::Event::HardwareSerial { hardware_serial_hi: hi, hardware_serial_lo: lo } => {
                guard.description.hardware_serial = HardwareSerialOrId { hi, lo };
                guard.description.capabilities |= HARDWARE_SERIAL;
            },
            zwp_tablet_tool_v2::Event::HardwareIdWacom { hardware_id_hi: hi, hardware_id_lo: lo } => {
                guard.description.hardware_id_wacom = HardwareSerialOrId { hi, lo };
                guard.description.capabilities |= HARDWARE_ID_WACOM;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Tilt) } => {
                guard.description.capabilities |= Capabilities::TILT;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Pressure) } => {
                guard.description.capabilities |= Capabilities::PRESSURE;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Distance) } => {
                guard.description.capabilities |= Capabilities::DISTANCE;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Rotation) } => {
                guard.description.capabilities |= Capabilities::ROTATION;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Slider) } => {
                guard.description.capabilities |= Capabilities::SLIDER;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Wheel) } => {
                guard.description.capabilities |= Capabilities::WHEEL;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(_) } => (),
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Unknown(unknown) } => {
                log::warn!(target: "sctk", "{}: invalid tablet tool type: {:x}", tool.id(), unknown);
                return;
            },
            zwp_tablet_tool_v2::Event::Done => {
                let description = mem::take(&mut guard.description);
                drop(guard);
                data.init_done(conn, qh, tool, description);
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

            zwp_tablet_tool_v2::Event::Frame { .. } => {
                let mut events = mem::take(&mut guard.pending);
                drop(guard);
                events.push(event);
                data.tablet_tool_frame(conn, qh, tool, &events);
            },
            // Could enumerate all the events,
            // but honestly it’s just easier to do this,
            // since we’re passing it through,
            // not reframing in any way.
            _ => guard.pending.push(event),

        }
    }
}
