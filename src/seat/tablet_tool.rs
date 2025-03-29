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
#[non_exhaustive]
pub struct Info {
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
impl fmt::Debug for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Info")
            .field("r#type", &self.r#type)
            .field("hardware_serial", &self.hardware_serial())
            .field("hardware_id_wacom", &self.hardware_id_wacom())
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl Default for Info {
    fn default() -> Info {
        Info {
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

impl Info {
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

/// The current state of the tool.
///
/// For many applications, when you receive a frame,
/// you don’t so much care about the events,
/// as about capturing the tool’s current total state.
///
/// This lets you view it that way, if you choose.
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
// At 184 bytes on a 64-bit platform, this is much larger than it fundamentally *need* be.
// With effort, you could losslessly encode all but wheel in less than 41 bytes,
// and with negligible loss you could shrink it to 33 bytes.
// But we’re trying to make it usefully accessible, not packed ultra-tight.
// So it’s mildly painfully wasteful instead.
#[derive(Debug, Default, Clone)]
pub struct State {
    // ProximityIn, ProximityOut
    /// The proximity information.
    ///
    /// When this is `None` (initially and after a `ProximityOut` event),
    /// tip state will be up and button states will be released,
    /// but all other axes will have stale values.
    pub proximity: Option<Proximity>,

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

    // Frame
    /// The time given in the last [Event::Frame].
    pub time: u32,
}

/// Information from the `ProximityIn` event.
#[derive(Debug, Clone)]
pub struct Proximity {
    /// The `Event::ProximityIn.serial` value,
    /// needed for [`ZwpTabletToolV2::set_cursor`] requests.
    pub serial: u32,
    pub tablet: ZwpTabletV2,
    pub surface: WlSurface,
}

impl State {
    /// Create blank state for a tool not yet in proximity.
    #[inline]
    pub fn new() -> State {
        State::default()
    }

    /// Whether the tool is in logical contact with the tablet.
    #[inline]
    pub fn is_down(&self) -> bool {
        self.down_serial.is_some()
    }

    /// Whether the tool is in proximity to a tablet.
    pub fn is_in_proximity(&self) -> bool {
        self.proximity.is_some()
    }

    /// Apply the events of a frame to this state.
    ///
    /// `Button` events are ignored for anything other than the stylus buttons,
    /// and button serials are discarded.
    /// If you want them, you must examine the event stream yourself.
    pub fn ingest_frame(&mut self, events: &[Event]) {
        for event in events {
            match *event {
                Event::ProximityIn { serial, ref tablet, ref surface } => {
                    self.proximity = Some(Proximity {
                        serial,
                        tablet: tablet.clone(),
                        surface: surface.clone(),
                    });
                },
                Event::ProximityOut => {
                    self.proximity = None;
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
                    // Deliberately ignored. Very unlikely in 2025, but possible.
                },
                Event::Frame { time } => {
                    self.time = time;
                },
                _ => {
                    // Info events, or anything else unknown. Should be unreachable.
                },
            }
        }
    }

    /// Get the pressure according to the web’s Pointer Events API:
    /// scaled in the range \[0.0, 1.0\],
    /// and, if pressure isn’t supported, set to 0.5 when down and 0.0 when up.
    pub fn pressure_web(&self, info: &Info) -> f64 {
        if info.supports_pressure() {
            self.pressure as f64 / 65535.0
        } else if self.is_down() {
            0.5
        } else {
            0.0
        }
    }

    // Pressure wants a special method, because falling back to 0 would be horrible,
    // and there’s an established convention on tip-state-aware fallback,
    // and mixing pressure-aware and pressure-unaware pointers is routine.
    // (Mind you, within tablet tools they’re probably almost all aware.
    // This method is *more* useful when you’ve merged tablet tools, touch and pointers.)
    //
    // Distance might like a special method, as it’s basically the opposite of pressure,
    // distance *from* the surface rather than distance *into* the surface,
    // but it’s nowhere near as commonly supported in hardware,
    // and it’s not common to build stuff on it,
    // and the web’s Pointer Events API doesn’t even expose it,
    // so there’s no established convention.
    // *Could* make its fallback the opposite of pressure’s, 0.0 if down else 0.5,
    // but I think people using distance are likely to want to branch on support,
    // a little more than is the case with pressure.
    //
    // As for the other capabilities, they’re all fine with a static fallback value of zero:
    // for tilt, rotation, slider, it’s the natural or neutral position;
    // and for wheel, it’s all delta anyway so unsupported is equivalent to unused.
}

/// Convert an event’s button state to a `bool` representing whether it’s pressed.
#[inline]
pub fn button_state_to_bool(state: WEnum<zwp_tablet_tool_v2::ButtonState>) -> bool {
    matches!(state, WEnum::Value(zwp_tablet_tool_v2::ButtonState::Pressed))
}

// Based on <https://lists.freedesktop.org/archives/wayland-devel/2025-March/044025.html>:
// BTN_STYLUS, BTN_STYLUS2 and BTN_STYLUS3 are the only codes likely.
// Mouse tools are long gone, finger was a mistake—everything’s a stylus.
/// The first button on a stylus.
pub const BTN_STYLUS: u32 = 0x14b;
/// The second button on a stylus.
pub const BTN_STYLUS2: u32 = 0x14c;
/// The third button on a stylus.
pub const BTN_STYLUS3: u32 = 0x149;

pub trait Handler: Sized {
    /// This is fired at the time of the `zwp_tablet_tool_v2.done` event,
    /// and collects any preceding `name`, `id` and `path` `type`, `hardware_serial`,
    /// `hardware_serial_wacom` and `capability` events into an [`Info`].
    fn info(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet: &ZwpTabletToolV2,
        info: Info,
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
    fn frame(
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
    inner: Mutex<DataInner>,
}

impl Data {
    pub fn new() -> Self {
        Self { inner: Default::default() }
    }
}

// I’d make this an enum, but the overhead of keeping info forever is negligible, ~24 bytes.
#[derive(Debug, Default)]
struct DataInner {
    /// An accumulation of pending init-time events, flushed when a `done` event comes in,
    /// after which it will be perpetually empty.
    info: Info,

    /// List of pending events, flushed when a `frame` event comes in.
    ///
    /// Explanation on chosen inline array sizing:
    ///  • There will always be at least two events: one axis change, and a Frame.
    ///  • Three is fundamentally common, when you have proximity and tip events.
    ///  • Pressure will be almost ubiquitous, so add one for that.
    ///  • Tilt will be very common too.
    /// My opinion, unmeasured save by eyeballing an event stream on a tilt+pressure-capable pen,
    /// is that four is probably the sweet spot.
    /// Ability to tweak that number would be a good reason to encapsulate this…!
    pending: SmallVec<[Event; 4]>,
}

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
                guard.info.r#type = match tool_type {
                    WEnum::Value(tool_type) => tool_type,
                    WEnum::Unknown(unknown) => {
                        log::warn!(target: "sctk", "{}: invalid tablet tool type: {:x}", tool.id(), unknown);
                        return;
                    },
                };
            },
            zwp_tablet_tool_v2::Event::HardwareSerial { hardware_serial_hi: hi, hardware_serial_lo: lo } => {
                guard.info.hardware_serial = HardwareSerialOrId { hi, lo };
                guard.info.capabilities |= HARDWARE_SERIAL;
            },
            zwp_tablet_tool_v2::Event::HardwareIdWacom { hardware_id_hi: hi, hardware_id_lo: lo } => {
                guard.info.hardware_id_wacom = HardwareSerialOrId { hi, lo };
                guard.info.capabilities |= HARDWARE_ID_WACOM;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Tilt) } => {
                guard.info.capabilities |= Capabilities::TILT;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Pressure) } => {
                guard.info.capabilities |= Capabilities::PRESSURE;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Distance) } => {
                guard.info.capabilities |= Capabilities::DISTANCE;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Rotation) } => {
                guard.info.capabilities |= Capabilities::ROTATION;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Slider) } => {
                guard.info.capabilities |= Capabilities::SLIDER;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(Capability::Wheel) } => {
                guard.info.capabilities |= Capabilities::WHEEL;
            },
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Value(_) } => (),
            zwp_tablet_tool_v2::Event::Capability { capability: WEnum::Unknown(unknown) } => {
                log::warn!(target: "sctk", "{}: invalid tablet tool type: {:x}", tool.id(), unknown);
                return;
            },
            zwp_tablet_tool_v2::Event::Done => {
                let info = mem::take(&mut guard.info);
                drop(guard);
                data.info(conn, qh, tool, info);
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
                data.frame(conn, qh, tool, &events);
            },
            // Could enumerate all the events,
            // but honestly it’s just easier to do this,
            // since we’re passing it through,
            // not reframing in any way.
            //
            // Any newly-defined info events will only be fired
            // if we bump our declared version support,
            // so no concerns there, this will only be the frame events.
            _ => guard.pending.push(event),

        }
    }
}
