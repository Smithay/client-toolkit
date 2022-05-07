use std::sync::Mutex;

use wayland_client::{
    protocol::{wl_pointer, wl_surface},
    Connection, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy, QueueHandle, WEnum,
};

use super::{SeatHandler, SeatState};

/// Describes a kind of pointer axis event.
#[derive(Debug, Clone, Copy)]
pub enum AxisKind {
    /// The axis scrolling is in an absolute number of pixels.
    Absolute(f64),

    /// The axis scrolling is in discrete units of lines or columns.
    Discrete(i32),

    /// The axis scrolling was stopped.
    ///
    /// Generally this variant is encountered when hardware indicates the end of some continuous scrolling.
    Stop,
}

/// A type representing a scroll event.
///
/// A scroll event may consist of a vertical and horizontal component.
#[derive(Debug)]
pub struct PointerScroll {
    horizontal: Option<AxisKind>,
    vertical: Option<AxisKind>,
    source: Option<wl_pointer::AxisSource>,
}

impl PointerScroll {
    pub fn axis(&self, axis: wl_pointer::Axis) -> Option<AxisKind> {
        match axis {
            wl_pointer::Axis::VerticalScroll => self.vertical,
            wl_pointer::Axis::HorizontalScroll => self.horizontal,

            _ => unreachable!(),
        }
    }

    pub fn source(&self) -> Option<wl_pointer::AxisSource> {
        self.source
    }

    pub fn has_axis(&self, axis: wl_pointer::Axis) -> bool {
        match axis {
            wl_pointer::Axis::VerticalScroll => self.vertical.is_some(),
            wl_pointer::Axis::HorizontalScroll => self.horizontal.is_some(),

            _ => unreachable!(),
        }
    }
}

pub trait PointerHandler: SeatHandler + Sized {
    /// The pointer focus is set to a surface.
    ///
    /// The `entered` parameter are the surface local coordinates from the top left corner where the cursor
    /// has entered.
    ///
    /// The pos
    fn pointer_focus(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        surface: &wl_surface::WlSurface,
        entered: (f64, f64),
        serial: u32,
    );

    /// The pointer focus is released from the surface.
    fn pointer_release_focus(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        surface: &wl_surface::WlSurface,
        serial: u32,
    );

    /// The pointer has moved.
    ///
    /// The position is in surface relative coordinates.
    fn pointer_motion(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        time: u32,
        position: (f64, f64),
    );

    /// A pointer button is pressed.
    fn pointer_press_button(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        time: u32,
        button: u32,
        serial: u32,
    );

    /// A pointer button is released.
    fn pointer_release_button(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        time: u32,
        button: u32,
        serial: u32,
    );

    /// A pointer's axis has scrolled.
    fn pointer_axis(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        time: u32,
        scroll: PointerScroll,
    );
}

#[derive(Debug, Default)]
pub struct PointerData {
    inner: Mutex<PointerDataInner>,
}

#[macro_export]
macro_rules! delegate_pointer {
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty:
            [
                $crate::reexports::client::protocol::wl_pointer::WlPointer
            ] => $crate::seat::SeatState
        );
    };
}

#[derive(Debug, Default)]
pub(crate) struct PointerDataInner {
    /// Pending axis event.
    axis: Option<Axis>,
    /// Pending motion event.
    motion: Option<Motion>,
    /// Pending button event.
    button: Option<Button>,
}

#[derive(Debug)]
pub(crate) struct Axis {
    horizontal: Option<AxisKind>,
    vertical: Option<AxisKind>,
    source: Option<wl_pointer::AxisSource>,
    time: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct Motion {
    x: f64,
    y: f64,
    time: u32,
}

#[derive(Debug)]
pub(crate) struct Button {
    time: u32,
    state: wl_pointer::ButtonState,
    button: u32,
    serial: u32,
}

impl DelegateDispatchBase<wl_pointer::WlPointer> for SeatState {
    type UserData = PointerData;
}

impl<D> DelegateDispatch<wl_pointer::WlPointer, D> for SeatState
where
    D: Dispatch<wl_pointer::WlPointer, UserData = Self::UserData> + PointerHandler,
{
    fn event(
        data: &mut D,
        pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        udata: &Self::UserData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wl_pointer::Event::Enter { surface, surface_x, surface_y, serial } => {
                data.pointer_focus(conn, qh, pointer, &surface, (surface_x, surface_y), serial);
            }

            wl_pointer::Event::Leave { surface, serial } => {
                data.pointer_release_focus(conn, qh, pointer, &surface, serial);
            }

            /*
            Pointer events

            The wl_pointer protocol starting in version 5 requires the following of clients:
            > A client is expected to accumulate the data in all events within the frame before proceeding.

            If the protocol version is 5 or greater, each of these events will accumulate state until a frame
            event.
            */
            wl_pointer::Event::Motion { time, surface_x, surface_y } => {
                if pointer.version() < 5 {
                    data.pointer_motion(conn, qh, pointer, time, (surface_x, surface_y));
                } else {
                    let mut guard = udata.inner.lock().unwrap();
                    guard.motion = Some(Motion { x: surface_x, y: surface_y, time });
                }
            }

            wl_pointer::Event::Button { time, button, state, serial } => match state {
                WEnum::Value(state) => {
                    if pointer.version() < 5 {
                        match state {
                            wl_pointer::ButtonState::Released => {
                                data.pointer_release_button(conn, qh, pointer, time, button, serial)
                            }
                            wl_pointer::ButtonState::Pressed => {
                                data.pointer_press_button(conn, qh, pointer, time, button, serial)
                            }

                            _ => unreachable!(),
                        }
                    } else {
                        let mut guard = udata.inner.lock().unwrap();
                        guard.button = Some(Button { time, state, button, serial });
                    }
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: invalid pointer button state: {:x}", pointer.id(), unknown)
                }
            },

            // Axis logical events.
            wl_pointer::Event::Axis { time, axis, value } => {
                match axis {
                    WEnum::Value(axis) => {
                        let (horizontal, vertical) = match axis {
                            wl_pointer::Axis::VerticalScroll => {
                                (None, Some(AxisKind::Absolute(value)))
                            }
                            wl_pointer::Axis::HorizontalScroll => {
                                (Some(AxisKind::Absolute(value)), None)
                            }

                            _ => unreachable!(),
                        };

                        // Old seats must emit two events, one for each axis.
                        if pointer.version() < 5 {
                            let scroll = PointerScroll {
                                horizontal,
                                vertical,
                                // A source cannot exist below version 5.
                                source: None,
                            };

                            data.pointer_axis(conn, qh, pointer, time, scroll);
                        } else {
                            // Starting in version 5, we must wait for a `frame` event before invoking the
                            // handler trait functions.

                            let mut guard = udata.inner.lock().unwrap();

                            if let Some(pending_axis) = guard.axis.as_mut() {
                                // Set time if it is not set yet.
                                // The time is `None` beforehand if some other axis event started the frame.
                                pending_axis.time.get_or_insert(time);
                            }

                            // Add absolute axis events to the pending frame.
                            let axis = guard.axis.get_or_insert(Axis {
                                horizontal,
                                vertical,
                                source: None,
                                time: Some(time),
                            });

                            if let Some(horizontal) = horizontal {
                                axis.horizontal.get_or_insert(horizontal);
                            }

                            if let Some(vertical) = vertical {
                                axis.vertical.get_or_insert(vertical);
                            }
                        }
                    }

                    WEnum::Unknown(unknown) => {
                        log::warn!(target: "sctk", "{}: invalid pointer axis: {:x}", pointer.id(), unknown)
                    }
                }
            }

            // Introduced in version 5
            wl_pointer::Event::AxisSource { axis_source } => match axis_source {
                WEnum::Value(source) => {
                    let mut guard = udata.inner.lock().unwrap();

                    let axis = guard.axis.get_or_insert(Axis {
                        horizontal: None,
                        vertical: None,
                        source: Some(source),
                        time: None,
                    });

                    axis.source.get_or_insert(source);
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "unknown pointer axis source: {:x}", unknown);
                }
            },

            // Introduced in version 5
            wl_pointer::Event::AxisStop { time, axis } => match axis {
                WEnum::Value(axis) => {
                    let mut guard = udata.inner.lock().unwrap();

                    if let Some(pending_axis) = guard.axis.as_mut() {
                        // Set time if it is not set yet.
                        // The time is `None` beforehand if some other axis event started the frame.
                        pending_axis.time.get_or_insert(time);
                    }

                    let (horizontal, vertical) = match axis {
                        wl_pointer::Axis::VerticalScroll => (None, Some(AxisKind::Stop)),

                        wl_pointer::Axis::HorizontalScroll => (Some(AxisKind::Stop), None),

                        _ => unreachable!(),
                    };

                    let axis = guard.axis.get_or_insert(Axis {
                        horizontal,
                        vertical,
                        source: None,
                        time: Some(time),
                    });

                    if let Some(horizontal) = horizontal {
                        axis.horizontal.get_or_insert(horizontal);
                    }

                    if let Some(vertical) = vertical {
                        axis.vertical.get_or_insert(vertical);
                    }
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                }
            },

            // Introduced in version 5
            wl_pointer::Event::AxisDiscrete { axis, discrete } => match axis {
                WEnum::Value(axis) => {
                    let mut guard = udata.inner.lock().unwrap();

                    let (horizontal, vertical) = match axis {
                        wl_pointer::Axis::VerticalScroll => {
                            (None, Some(AxisKind::Discrete(discrete)))
                        }

                        wl_pointer::Axis::HorizontalScroll => {
                            (Some(AxisKind::Discrete(discrete)), None)
                        }

                        _ => unreachable!(),
                    };

                    let axis = guard.axis.get_or_insert(Axis {
                        horizontal,
                        vertical,
                        source: None,
                        time: None,
                    });

                    if let Some(horizontal) = horizontal {
                        axis.horizontal.get_or_insert(horizontal);
                    }

                    if let Some(vertical) = vertical {
                        axis.vertical.get_or_insert(vertical);
                    }
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                }
            },

            wl_pointer::Event::Frame => {
                // `frame` is essentially an atomic signal that all events have been received.
                let mut guard = udata.inner.lock().unwrap();
                // The protocol says only one of each "logical event group" will correspond to a frame.
                // However, compositor implementations are not all that consistent so let's be flexible.
                let axis = guard.axis.take();
                let motion = guard.motion.take();
                let button = guard.button.take();

                drop(guard);

                if let Some(axis) = axis {
                    let scroll = PointerScroll {
                        horizontal: axis.horizontal,
                        vertical: axis.vertical,
                        source: axis.source,
                    };

                    // If time isn't set for some reason, just pass 0.
                    data.pointer_axis(conn, qh, pointer, axis.time.unwrap_or(0), scroll);
                }

                if let Some(motion) = motion {
                    data.pointer_motion(conn, qh, pointer, motion.time, (motion.x, motion.y));
                }

                if let Some(button) = button {
                    match button.state {
                        wl_pointer::ButtonState::Released => {
                            data.pointer_release_button(
                                conn,
                                qh,
                                pointer,
                                button.time,
                                button.button,
                                button.serial,
                            );
                        }

                        wl_pointer::ButtonState::Pressed => {
                            data.pointer_press_button(
                                conn,
                                qh,
                                pointer,
                                button.time,
                                button.button,
                                button.serial,
                            );
                        }

                        _ => unreachable!(),
                    }
                }
            }

            _ => unreachable!(),
        }
    }
}
