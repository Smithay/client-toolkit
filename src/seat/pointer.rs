use wayland_client::{
    protocol::wl_pointer, ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch,
    Proxy, QueueHandle, WEnum,
};

use super::{SeatData, SeatDispatch, SeatHandler};

pub(crate) struct InvalidFrame;

/// Accumulation of multiple pointer events ended by a wl_pointer::frame event.
#[derive(Debug)]
pub(crate) struct PointerFrame {
    /// Whether this pointer frame has had a single event logical group.
    ///
    /// wl_pointer::motion and wl_pointer::button are examples of single event logical groups.
    pub(crate) is_single_event_logical_group: bool,

    pub(crate) horizontal_axe: Option<AxisFrame>,

    pub(crate) vertical_axe: Option<AxisFrame>,

    /// The source of axis scrolling.
    ///
    /// This may only be set once during a frame. We ignore subsequent attempts to set the value.
    pub(crate) axis_source: Option<wl_pointer::AxisSource>,
}

impl PointerFrame {
    pub fn take(&mut self) -> PointerFrame {
        let is_single_event_logical_group = self.is_single_event_logical_group;

        PointerFrame {
            is_single_event_logical_group,
            horizontal_axe: self.horizontal_axe.take(),
            vertical_axe: self.vertical_axe.take(),
            axis_source: self.axis_source.take(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct AxisFrame {
    /// The time which an axis frame occurs at.
    ///
    /// This is an Option since some earlier frame events may not immediately provide the time but is must be
    /// [`Some`] when wl_pointer::frame is handled.
    time: Option<u32>,

    /// The axis scrolling was stopped.
    stop: bool,

    /// The number of pixels scrolled.
    ///
    /// Either this or discrete will be some.
    absolute: Option<f64>,

    /// The discrete scroll amount.
    ///
    /// This is generally defined in some unit, such as lines or columns depending on the application.
    ///
    /// Either this or discrete will be some.
    discrete: Option<i32>,
}

impl<D, H> DelegateDispatchBase<wl_pointer::WlPointer> for SeatDispatch<'_, D, H>
where
    H: SeatHandler<D>,
{
    type UserData = SeatData;
}

impl<D, H> DelegateDispatch<wl_pointer::WlPointer, D> for SeatDispatch<'_, D, H>
where
    D: Dispatch<wl_pointer::WlPointer, UserData = Self::UserData>,
    H: SeatHandler<D>,
{
    fn event(
        &mut self,
        pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        data: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wl_pointer::Event::Enter { surface, surface_x, surface_y, .. } => {
                self.1.pointer_focus(conn, qh, self.0, pointer, &surface, (surface_x, surface_y));
            }

            wl_pointer::Event::Leave { surface, .. } => {
                self.1.pointer_release_focus(conn, qh, self.0, pointer, &surface);
            }

            wl_pointer::Event::Motion { time, surface_x, surface_y } => {
                /*
                The protocol says the following regarding a frame:

                > A wl_pointer.frame event is sent for every logical event group, even if the group only
                > contains a single wl_pointer event.

                This means that wl_pointer::motion (this event) should be followed by a wl_pointer::frame event.
                However since this is the only event of the logical group for pointer motion, we can
                immediately invoke the handler trait to indicate pointer motion has occurred and simply
                swallow the incoming wl_pointer::frame event.
                */

                // Warn if we have an invalid frame
                let mut frame = data.pointer_frame.lock().unwrap();

                if frame.is_single_event_logical_group
                    || frame.horizontal_axe.is_some()
                    || frame.vertical_axe.is_some()
                    || frame.axis_source.is_some()
                {
                    log::warn!(target: "sctk", "wl_pointer::motion sent during a different frame. emitting anyways.");
                }

                frame.is_single_event_logical_group = true;
                // TODO: Emit event on pointer handler trait
            }

            wl_pointer::Event::Button { time, button, state, .. } => match state {
                /*
                The protocol says the following regarding a frame:

                > A wl_pointer.frame event is sent for every logical event group, even if the group only
                > contains a single wl_pointer event.

                This means that wl_pointer::button (this event) should be followed by a wl_pointer::frame event.
                However since this is the only event of the logical group for button press/release, we can
                immediately invoke the handler trait to indicate pointer motion has occurred and simply
                swallow the incoming wl_pointer::frame event.
                */
                WEnum::Value(state) => {
                    // Warn if we have an invalid frame
                    let mut frame = data.pointer_frame.lock().unwrap();

                    if frame.is_single_event_logical_group
                        || frame.horizontal_axe.is_some()
                        || frame.vertical_axe.is_some()
                        || frame.axis_source.is_some()
                    {
                        log::warn!(target: "sctk", "wl_pointer::button sent during a different frame. emitting anyways.");
                    }

                    frame.is_single_event_logical_group = true;

                    // TODO: Emit event on pointer handler trait
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: compositor sends invalid button state: {:x}", pointer.id(), unknown);
                }
            },

            /*
            Axis logical events.

            Since there are multiple events in the logical event group for axis events, we need to queue up
            all data regarding the events and emit all the data at once during the wl_pointer::frame event.
            */
            wl_pointer::Event::Axis { time, axis, value } => match axis {
                WEnum::Value(axis) => {
                    let mut frame = data.pointer_frame.lock().unwrap();

                    // Check if the compositor has sent an invalid frame.
                    if frame.is_single_event_logical_group {
                        log::warn!(target: "sctk", "wl_pointer::axis sent during a non-axis frame. emitting anyways.");
                    }

                    if let wl_pointer::Axis::HorizontalScroll = axis {
                        match frame.horizontal_axe {
                            Some(ref mut axis_frame) => {
                                // wl_pointer::axis_discrete may not provide a time, take it from here
                                axis_frame.time = Some(time);
                                axis_frame.absolute = Some(value);
                            }

                            None => {
                                frame.horizontal_axe = Some(AxisFrame {
                                    time: Some(time),
                                    stop: false,
                                    absolute: Some(value),
                                    discrete: None,
                                })
                            }
                        }
                    }

                    if let wl_pointer::Axis::VerticalScroll = axis {
                        match frame.vertical_axe {
                            Some(ref mut axis_frame) => {
                                // wl_pointer::axis_discrete may not provide a time, take it from here
                                axis_frame.time = Some(time);
                                axis_frame.absolute = Some(value);
                            }

                            None => {
                                frame.vertical_axe = Some(AxisFrame {
                                    time: Some(time),
                                    stop: false,
                                    absolute: Some(value),
                                    discrete: None,
                                })
                            }
                        }
                    }
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: compositor sends invalid axis: {:x}", pointer.id(), unknown);
                }
            },

            wl_pointer::Event::AxisSource { axis_source } => match axis_source {
                WEnum::Value(axis_source) => {
                    let mut frame = data.pointer_frame.lock().unwrap();

                    if frame.is_single_event_logical_group {
                        log::warn!(target: "sctk", "wl_pointer::axis_source sent during a non-axis frame. emitting anyways.");
                    }

                    frame.axis_source = Some(axis_source);
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "unknown axis source: {:x}", unknown);
                }
            },

            wl_pointer::Event::AxisStop { time, axis } => match axis {
                WEnum::Value(axis) => {
                    let mut frame = data.pointer_frame.lock().unwrap();

                    if let wl_pointer::Axis::HorizontalScroll = axis {
                        match frame.horizontal_axe {
                            Some(ref mut axis_frame) => {
                                // wl_pointer::axis_discrete may not provide a time, take it from here
                                axis_frame.time = Some(time);
                                axis_frame.stop = true;
                            }

                            None => {
                                frame.horizontal_axe = Some(AxisFrame {
                                    time: Some(time),
                                    stop: true,
                                    absolute: None,
                                    discrete: None,
                                })
                            }
                        }
                    }

                    if let wl_pointer::Axis::VerticalScroll = axis {
                        match frame.vertical_axe {
                            Some(ref mut axis_frame) => {
                                // wl_pointer::axis_discrete may not provide a time, take it from here
                                axis_frame.time = Some(time);
                                axis_frame.stop = true;
                            }

                            None => {
                                frame.vertical_axe = Some(AxisFrame {
                                    time: Some(time),
                                    stop: true,
                                    absolute: None,
                                    discrete: None,
                                })
                            }
                        }
                    }
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: compositor sends invalid axis: {:x}", pointer.id(), unknown);
                }
            },

            wl_pointer::Event::AxisDiscrete { axis, discrete } => {
                match axis {
                    WEnum::Value(axis) => {
                        // axis_discrete will always be the first event of some axe in the frame, so initializing the
                        // axis frame should never fail assuming a complaint server.
                        let mut frame = data.pointer_frame.lock().unwrap();
                        // We don't have the time, let a future event fill it in.

                        if let wl_pointer::Axis::HorizontalScroll = axis {
                            match frame.horizontal_axe {
                                Some(ref mut axis_frame) => {
                                    // wl_pointer::axis_discrete does not provide a time, but the protocol
                                    // says we will get the time later.
                                    axis_frame.discrete = Some(discrete);
                                }

                                None => {
                                    frame.horizontal_axe = Some(AxisFrame {
                                        time: None,
                                        stop: true,
                                        absolute: None,
                                        discrete: Some(discrete),
                                    })
                                }
                            }
                        }

                        if let wl_pointer::Axis::VerticalScroll = axis {
                            match frame.vertical_axe {
                                Some(ref mut axis_frame) => {
                                    // wl_pointer::axis_discrete does not provide a time, but the protocol
                                    // says we will get the time later.
                                    axis_frame.discrete = Some(discrete);
                                }

                                None => {
                                    frame.vertical_axe = Some(AxisFrame {
                                        time: None,
                                        stop: true,
                                        absolute: None,
                                        discrete: Some(discrete),
                                    })
                                }
                            }
                        }
                    }

                    WEnum::Unknown(unknown) => {
                        log::warn!(target: "sctk", "{}: compositor sends invalid axis: {:x}", pointer.id(), unknown);
                    }
                }
            }

            wl_pointer::Event::Frame => {
                let mut guard = data.pointer_frame.lock().unwrap();
                let frame = guard.take();
                drop(guard);

                if let Some(horizontal) = frame.horizontal_axe {
                    // TODO: Emit event on pointer handler trait
                }

                if let Some(vertical) = frame.vertical_axe {
                    // TODO: Emit event on pointer handler trait
                }
            }

            _ => unreachable!(),
        }
    }
}
