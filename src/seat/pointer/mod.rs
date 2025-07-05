use std::{
    collections::{hash_map::Entry, HashMap},
    env, iter, mem,
    sync::{Arc, Mutex},
};

use wayland_backend::{client::InvalidId, smallvec::SmallVec};
use wayland_client::{
    protocol::{
        wl_pointer::{self, WlPointer},
        wl_seat::WlSeat,
        wl_shm::WlShm,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_cursor::{Cursor, CursorTheme};
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1;

use crate::{
    compositor::{SurfaceData, SurfaceDataExt},
    error::GlobalError,
};

use super::SeatState;

#[doc(inline)]
pub use cursor_icon::{CursorIcon, ParseError as CursorIconParseError};

pub mod cursor_shape;

use cursor_shape::cursor_icon_to_shape;

/* From linux/input-event-codes.h - the buttons usually used by mice */
pub const BTN_LEFT: u32 = 0x110;
pub const BTN_RIGHT: u32 = 0x111;
pub const BTN_MIDDLE: u32 = 0x112;
/// The fourth non-scroll button, which is often used as "back" in web browsers.
pub const BTN_SIDE: u32 = 0x113;
/// The fifth non-scroll button, which is often used as "forward" in web browsers.
pub const BTN_EXTRA: u32 = 0x114;

/// See also [`BTN_EXTRA`].
pub const BTN_FORWARD: u32 = 0x115;
/// See also [`BTN_SIDE`].
pub const BTN_BACK: u32 = 0x116;
pub const BTN_TASK: u32 = 0x117;

/// Describes a scroll along one axis
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct AxisScroll {
    /// The scroll measured in pixels.
    pub absolute: f64,

    /// The scroll measured in steps.
    ///
    /// Note: this might always be zero if the scrolling is due to a touchpad or other continuous
    /// source.
    ///
    /// This event is deprecated and will be sent only by older compositors.
    pub discrete: i32,

    /// High-resolution wheel scroll information, with each multiple of 120 representing one logical scroll step.
    pub value120: i32,

    /// Relative directional information of the entity causing the axis motion.
    pub relative_direction: Option<wl_pointer::AxisRelativeDirection>,

    /// The scroll was stopped.
    ///
    /// Generally this is encountered when hardware indicates the end of some continuous scrolling.
    pub stop: bool,
}

impl AxisScroll {
    /// Returns true if there was no movement along this axis.
    pub fn is_none(&self) -> bool {
        *self == Self::default()
    }

    /// Combines the magnitudes and stop status of events if the direction hasn't changed in between.
    fn merge(&self, other: &Self) -> Option<Self> {
        // Events which are converted to new AxisScroll instances can carry partial data only.
        // Assuming here that no specified direction means that the frame doesn't contain that event yet and it just needs to be filled in. However, this assumptoin doesn't hold universally. An AxisScroll instance can be created out of merged events across frames. In that case, the direction will be applied retroactively to the previous frame.
        // It doesn't seem likely to me that a direction changes between frames, and the consequences of that are just a glitch in movement, so I'll let it in until it proves to be an issue - solving this properly may require a larger redesign.
        let direction = match (self.relative_direction, other.relative_direction) {
            (None, other) | (other, None) => other,
            (Some(one), Some(other)) => {
                if one != other {
                    return None;
                } else {
                    Some(one)
                }
            }
        };

        let mut ret = *self;
        ret.absolute += other.absolute;
        ret.discrete += other.discrete;
        ret.value120 += other.value120;
        ret.relative_direction = direction;
        ret.stop |= other.stop;
        Some(ret)
    }
}

/// A single pointer event.
#[derive(Debug, Clone)]
pub struct PointerEvent {
    pub surface: WlSurface,
    pub position: (f64, f64),
    pub kind: PointerEventKind,
}

#[derive(Debug, Clone)]
pub enum PointerEventKind {
    Enter {
        serial: u32,
    },
    Leave {
        serial: u32,
    },
    Motion {
        time: u32,
    },
    Press {
        time: u32,
        button: u32,
        serial: u32,
    },
    Release {
        time: u32,
        button: u32,
        serial: u32,
    },
    Axis {
        time: u32,
        horizontal: AxisScroll,
        vertical: AxisScroll,
        source: Option<wl_pointer::AxisSource>,
    },
}

pub trait PointerHandler: Sized {
    /// One or more pointer events are available.
    ///
    /// Multiple related events may be grouped together in a single frame.  Some examples:
    ///
    /// - A drag that terminates outside the surface may send the Release and Leave events as one frame
    /// - Movement from one surface to another may send the Enter and Leave events in one frame
    fn pointer_frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &WlPointer,
        events: &[PointerEvent],
    );
}

#[derive(Debug)]
pub struct PointerData {
    seat: WlSeat,
    pub(crate) inner: Mutex<PointerDataInner>,
}

impl PointerData {
    pub fn new(seat: WlSeat) -> Self {
        Self { seat, inner: Default::default() }
    }

    /// The seat associated with this pointer.
    pub fn seat(&self) -> &WlSeat {
        &self.seat
    }

    /// Serial from the latest [`PointerEventKind::Enter`] event.
    pub fn latest_enter_serial(&self) -> Option<u32> {
        self.inner.lock().unwrap().latest_enter
    }

    /// Serial from the latest button [`PointerEventKind::Press`] and
    /// [`PointerEventKind::Release`] events.
    pub fn latest_button_serial(&self) -> Option<u32> {
        self.inner.lock().unwrap().latest_btn
    }
}

pub trait PointerDataExt: Send + Sync {
    fn pointer_data(&self) -> &PointerData;
}

impl PointerDataExt for PointerData {
    fn pointer_data(&self) -> &PointerData {
        self
    }
}

#[macro_export]
macro_rules! delegate_pointer {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::delegate_pointer!(@{ $(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty }; pointer: []);
        $crate::delegate_pointer!(@{ $(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty }; pointer-only: $crate::seat::pointer::PointerData);
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, pointer: [$($pointer_data:ty),* $(,)?]) => {
        $crate::delegate_pointer!(@{ $(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty }; pointer: [ $($pointer_data),* ]);
    };
    (@{$($ty:tt)*}; pointer: []) => {
        $crate::reexports::client::delegate_dispatch!($($ty)*:
            [
                $crate::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_manager_v1::WpCursorShapeManagerV1: $crate::globals::GlobalData
            ] => $crate::seat::pointer::cursor_shape::CursorShapeManager
        );
        $crate::reexports::client::delegate_dispatch!($($ty)*:
            [
                $crate::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1: $crate::globals::GlobalData
            ] => $crate::seat::pointer::cursor_shape::CursorShapeManager
        );
    };
    (@{$($ty:tt)*}; pointer-only: $pointer_data:ty) => {
        $crate::reexports::client::delegate_dispatch!($($ty)*:
            [
                $crate::reexports::client::protocol::wl_pointer::WlPointer: $pointer_data
            ] => $crate::seat::SeatState
        );
    };
    (@$ty:tt; pointer: [$($pointer:ty),*]) => {
        $crate::delegate_pointer!(@$ty; pointer: []);
        $( $crate::delegate_pointer!(@$ty; pointer-only: $pointer); )*
    }
}

#[derive(Debug, Default)]
pub(crate) struct PointerDataInner {
    /// Surface the pointer most recently entered
    pub(crate) surface: Option<WlSurface>,
    /// Position relative to the surface
    pub(crate) position: (f64, f64),

    /// List of pending events.  Only used for version >= 5.
    pub(crate) pending: SmallVec<[PointerEvent; 3]>,

    /// The serial of the latest enter event for the pointer
    pub(crate) latest_enter: Option<u32>,

    /// The serial of the latest button event for the pointer
    pub(crate) latest_btn: Option<u32>,
}

impl<D, U> Dispatch<WlPointer, U, D> for SeatState
where
    D: Dispatch<WlPointer, U> + PointerHandler,
    U: PointerDataExt,
{
    fn event(
        data: &mut D,
        pointer: &WlPointer,
        event: wl_pointer::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let udata = udata.pointer_data();
        let mut guard = udata.inner.lock().unwrap();
        let mut leave_surface = None;
        let kind = match event {
            wl_pointer::Event::Enter { surface, surface_x, surface_y, serial } => {
                guard.surface = Some(surface);
                guard.position = (surface_x, surface_y);
                guard.latest_enter.replace(serial);

                PointerEventKind::Enter { serial }
            }

            wl_pointer::Event::Leave { surface, serial } => {
                if guard.surface.as_ref() == Some(&surface) {
                    guard.surface = None;
                }
                leave_surface = Some(surface);

                PointerEventKind::Leave { serial }
            }

            wl_pointer::Event::Motion { time, surface_x, surface_y } => {
                guard.position = (surface_x, surface_y);

                PointerEventKind::Motion { time }
            }

            wl_pointer::Event::Button { time, button, state, serial } => {
                guard.latest_btn.replace(serial);
                match state {
                    WEnum::Value(wl_pointer::ButtonState::Pressed) => {
                        PointerEventKind::Press { time, button, serial }
                    }
                    WEnum::Value(wl_pointer::ButtonState::Released) => {
                        PointerEventKind::Release { time, button, serial }
                    }
                    WEnum::Unknown(unknown) => {
                        log::warn!(target: "sctk", "{}: invalid pointer button state: {:x}", pointer.id(), unknown);
                        return;
                    }
                    _ => unreachable!(),
                }
            }
            // Axis logical events.
            wl_pointer::Event::Axis { time, axis, value } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => {
                            vertical.absolute = value;
                        }
                        wl_pointer::Axis::HorizontalScroll => {
                            horizontal.absolute = value;
                        }
                        _ => unreachable!(),
                    };

                    PointerEventKind::Axis { time, horizontal, vertical, source: None }
                }
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                    return;
                }
            },

            wl_pointer::Event::AxisSource { axis_source } => match axis_source {
                WEnum::Value(source) => PointerEventKind::Axis {
                    horizontal: AxisScroll::default(),
                    vertical: AxisScroll::default(),
                    source: Some(source),
                    time: 0,
                },
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "unknown pointer axis source: {:x}", unknown);
                    return;
                }
            },

            wl_pointer::Event::AxisStop { time, axis } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => vertical.stop = true,
                        wl_pointer::Axis::HorizontalScroll => horizontal.stop = true,

                        _ => unreachable!(),
                    }

                    PointerEventKind::Axis { time, horizontal, vertical, source: None }
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                    return;
                }
            },

            wl_pointer::Event::AxisDiscrete { axis, discrete } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => {
                            vertical.discrete = discrete;
                        }

                        wl_pointer::Axis::HorizontalScroll => {
                            horizontal.discrete = discrete;
                        }

                        _ => unreachable!(),
                    };

                    PointerEventKind::Axis { time: 0, horizontal, vertical, source: None }
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                    return;
                }
            },

            wl_pointer::Event::AxisValue120 { axis, value120 } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => {
                            vertical.value120 = value120;
                        }

                        wl_pointer::Axis::HorizontalScroll => {
                            horizontal.value120 = value120;
                        }

                        _ => unreachable!(),
                    };

                    PointerEventKind::Axis { time: 0, horizontal, vertical, source: None }
                }
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                    return;
                }
            },

            wl_pointer::Event::AxisRelativeDirection { axis, direction } => {
                let direction = match direction {
                    WEnum::Value(dir) => Some(dir),
                    WEnum::Unknown(unknown) => {
                        log::warn!(target: "sctk", "{}: invalid axis direction: {:x}", pointer.id(), unknown);
                        return;
                    }
                };
                match axis {
                    WEnum::Value(axis) => {
                        let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                        match axis {
                            wl_pointer::Axis::VerticalScroll => {
                                vertical.relative_direction = direction;
                            }

                            wl_pointer::Axis::HorizontalScroll => {
                                horizontal.relative_direction = direction;
                            }

                            _ => unreachable!(),
                        };

                        PointerEventKind::Axis { time: 0, horizontal, vertical, source: None }
                    }

                    WEnum::Unknown(unknown) => {
                        log::warn!(target: "sctk", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                        return;
                    }
                }
            }

            wl_pointer::Event::Frame => {
                let pending = mem::take(&mut guard.pending);
                drop(guard);
                if !pending.is_empty() {
                    data.pointer_frame(conn, qh, pointer, &pending);
                }
                return;
            }

            _ => unreachable!(),
        };

        let surface = match (leave_surface, &guard.surface) {
            (Some(surface), _) => surface,
            (None, Some(surface)) => surface.clone(),
            (None, None) => {
                log::warn!(target: "sctk", "{}: got pointer event {:?} without an entered surface", pointer.id(), kind);
                return;
            }
        };

        let event = PointerEvent { surface, position: guard.position, kind };

        if pointer.version() < 5 {
            drop(guard);
            // No Frame events, send right away
            data.pointer_frame(conn, qh, pointer, &[event]);
        } else {
            // Merge a new Axis event with the previous event to create an event with more
            // information and potentially diagonal scrolling.
            if let (
                Some(PointerEvent {
                    kind:
                        PointerEventKind::Axis { time: ot, horizontal: oh, vertical: ov, source: os },
                    ..
                }),
                PointerEvent {
                    kind:
                        PointerEventKind::Axis { time: nt, horizontal: nh, vertical: nv, source: ns },
                    ..
                },
            ) = (guard.pending.last_mut(), &event)
            {
                // A time of 0 is "don't know", so avoid using it if possible.
                if *ot == 0 {
                    *ot = *nt;
                }
                let nh = oh.merge(nh);
                let nv = ov.merge(nv);
                // Merging doesn't make sense in some situations.
                if let (Some(nh), Some(nv)) = (nh, nv) {
                    *oh = nh;
                    *ov = nv;
                    *os = os.or(*ns);
                    return;
                }
            }

            guard.pending.push(event);
        }
    }
}

/// Pointer themeing
#[derive(Debug)]
pub struct ThemedPointer<U = PointerData, S = SurfaceData> {
    pub(super) themes: Arc<Mutex<Themes>>,
    /// The underlying wl_pointer.
    pub(super) pointer: WlPointer,
    pub(super) shm: WlShm,
    /// The surface owned by the cursor to present the icon.
    pub(super) surface: WlSurface,
    pub(super) shape_device: Option<WpCursorShapeDeviceV1>,
    pub(super) _marker: std::marker::PhantomData<U>,
    pub(super) _surface_data: std::marker::PhantomData<S>,
}

impl<U: PointerDataExt + 'static, S: SurfaceDataExt + 'static> ThemedPointer<U, S> {
    /// Set the cursor to the given [`CursorIcon`].
    ///
    /// The cursor icon should be reloaded on every [`PointerEventKind::Enter`] event.
    pub fn set_cursor(&self, conn: &Connection, icon: CursorIcon) -> Result<(), PointerThemeError> {
        let serial = match self
            .pointer
            .data::<U>()
            .and_then(|data| data.pointer_data().latest_enter_serial())
        {
            Some(serial) => serial,
            None => return Err(PointerThemeError::MissingEnterSerial),
        };

        if let Some(shape_device) = self.shape_device.as_ref() {
            shape_device.set_shape(serial, cursor_icon_to_shape(icon, shape_device.version()));
            Ok(())
        } else {
            self.set_cursor_legacy(conn, serial, icon)
        }
    }

    /// The legacy method of loading the cursor from the system cursor
    /// theme instead of relying on compositor to set the cursor.
    fn set_cursor_legacy(
        &self,
        conn: &Connection,
        serial: u32,
        icon: CursorIcon,
    ) -> Result<(), PointerThemeError> {
        let mut themes = self.themes.lock().unwrap();

        let scale = self.surface.data::<S>().unwrap().surface_data().scale_factor();
        for cursor_icon_name in iter::once(&icon.name()).chain(icon.alt_names().iter()) {
            if let Some(cursor) = themes
                .get_cursor(conn, cursor_icon_name, scale as u32, &self.shm)
                .map_err(PointerThemeError::InvalidId)?
            {
                let image = &cursor[0];
                let (w, h) = image.dimensions();
                let (hx, hy) = image.hotspot();

                self.surface.set_buffer_scale(scale);
                self.surface.attach(Some(image), 0, 0);

                if self.surface.version() >= 4 {
                    self.surface.damage_buffer(0, 0, w as i32, h as i32);
                } else {
                    // Fallback for the old old surface.
                    self.surface.damage(0, 0, w as i32 / scale, h as i32 / scale);
                }

                // Commit the surface to place the cursor image in the compositor's memory.
                self.surface.commit();

                // Set the pointer surface to change the pointer.
                self.pointer.set_cursor(
                    serial,
                    Some(&self.surface),
                    hx as i32 / scale,
                    hy as i32 / scale,
                );

                return Ok(());
            }
        }

        Err(PointerThemeError::CursorNotFound)
    }

    /// Hide the cursor by providing empty surface for it.
    ///
    /// The cursor should be hidden on every [`PointerEventKind::Enter`] event.
    pub fn hide_cursor(&self) -> Result<(), PointerThemeError> {
        let data = self.pointer.data::<U>();
        if let Some(serial) = data.and_then(|data| data.pointer_data().latest_enter_serial()) {
            self.pointer.set_cursor(serial, None, 0, 0);
            Ok(())
        } else {
            Err(PointerThemeError::MissingEnterSerial)
        }
    }

    /// The [`WlPointer`] associated with this [`ThemedPointer`].
    pub fn pointer(&self) -> &WlPointer {
        &self.pointer
    }

    /// The associated [`WlSurface`] with this [`ThemedPointer`].
    pub fn surface(&self) -> &WlSurface {
        &self.surface
    }
}

impl<U, S> Drop for ThemedPointer<U, S> {
    fn drop(&mut self) {
        if let Some(shape_device) = self.shape_device.take() {
            shape_device.destroy();
        }

        if self.pointer.version() >= 3 {
            self.pointer.release();
        }
        self.surface.destroy();
    }
}

/// Specifies which cursor theme should be used by the theme manager.
#[derive(Debug)]
pub enum ThemeSpec<'a> {
    /// Use this specific theme with the given base size.
    Named {
        /// Name of the cursor theme.
        name: &'a str,

        /// Base size of the cursor names.
        ///
        /// Note this size assumes a scale factor of 1. Cursor image sizes may be multiplied by the base size
        /// for HiDPI outputs.
        size: u32,
    },

    /// Use the system provided theme
    ///
    /// In this case SCTK will read the `XCURSOR_THEME` and
    /// `XCURSOR_SIZE` environment variables to figure out the
    /// theme to use.
    System,
}

impl Default for ThemeSpec<'_> {
    fn default() -> Self {
        Self::System
    }
}

/// An error indicating that the cursor was not found.
#[derive(Debug, thiserror::Error)]
pub enum PointerThemeError {
    /// An invalid ObjectId was used.
    #[error("Invalid ObjectId")]
    InvalidId(InvalidId),

    /// A global error occurred.
    #[error("A Global Error occured")]
    GlobalError(GlobalError),

    /// The requested cursor was not found.
    #[error("Cursor not found")]
    CursorNotFound,

    /// There has been no enter event yet for the pointer.
    #[error("Missing enter event serial")]
    MissingEnterSerial,
}

#[derive(Debug)]
pub(crate) struct Themes {
    name: String,
    size: u32,
    // Scale -> CursorTheme
    themes: HashMap<u32, CursorTheme>,
}

impl Default for Themes {
    fn default() -> Self {
        Themes::new(ThemeSpec::default())
    }
}

impl Themes {
    pub(crate) fn new(spec: ThemeSpec) -> Themes {
        let (name, size) = match spec {
            ThemeSpec::Named { name, size } => (name.into(), size),
            ThemeSpec::System => {
                let name = env::var("XCURSOR_THEME").ok().unwrap_or_else(|| "default".into());
                let size = env::var("XCURSOR_SIZE").ok().and_then(|s| s.parse().ok()).unwrap_or(24);
                (name, size)
            }
        };

        Themes { name, size, themes: HashMap::new() }
    }

    fn get_cursor(
        &mut self,
        conn: &Connection,
        name: &str,
        scale: u32,
        shm: &WlShm,
    ) -> Result<Option<&Cursor>, InvalidId> {
        // Check if the theme has been initialized at the specified scale.
        if let Entry::Vacant(e) = self.themes.entry(scale) {
            // Initialize the theme for the specified scale
            let theme = CursorTheme::load_from_name(
                conn,
                shm.clone(), // TODO: Does the cursor theme need to clone wl_shm?
                &self.name,
                self.size * scale,
            )?;

            e.insert(theme);
        }

        let theme = self.themes.get_mut(&scale).unwrap();

        Ok(theme.get_cursor(name))
    }
}
