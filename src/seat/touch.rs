use std::sync::Mutex;

use wayland_client::protocol::wl_seat::WlSeat;

use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::protocol::wl_touch::{Event as TouchEvent, WlTouch};
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::seat::SeatState;

#[derive(Debug)]
pub struct TouchData {
    seat: WlSeat,

    inner: Mutex<TouchDataInner>,
}

impl TouchData {
    /// Create the new touch data associated with the given seat.
    pub fn new(seat: WlSeat) -> Self {
        Self { seat, inner: Default::default() }
    }

    /// Get the associated seat from the data.
    pub fn seat(&self) -> &WlSeat {
        &self.seat
    }
}

#[derive(Debug, Default)]
pub(crate) struct TouchDataInner {
    events: Vec<TouchEvent>,
}

#[macro_export]
macro_rules! delegate_touch {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_touch::WlTouch: $crate::seat::touch::TouchData
            ] => $crate::seat::SeatState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, touch: [$($td:ty),* $(,)?]) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $(
                    $crate::reexports::client::protocol::wl_touch::WlTouch: $td,
                )*
            ] => $crate::seat::SeatState
        );
    };
}

pub trait TouchDataExt: Send + Sync {
    fn touch_data(&self) -> &TouchData;
}

impl TouchDataExt for TouchData {
    fn touch_data(&self) -> &TouchData {
        self
    }
}

pub trait TouchHandler: Sized {
    /// New touch point.
    ///
    /// Indicates a new touch point has appeared on the surface, starting a touch sequence. The ID
    /// associated with this event identifies this touch point for devices with multi-touch and
    /// will be referenced in future events.
    ///
    /// The associated touch ID ceases to be valid after the touch up event with the associated ID
    /// and may be reused for other touch points after that.
    ///
    /// Coordinates are surface-local.
    #[allow(clippy::too_many_arguments)]
    fn down(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &WlTouch,
        serial: u32,
        time: u32,
        surface: WlSurface,
        id: i32,
        position: (f64, f64),
    );

    /// End of touch sequence.
    fn up(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &WlTouch,
        serial: u32,
        time: u32,
        id: i32,
    );

    /// Touch point motion.
    ///
    /// Coordinates are surface-local.
    fn motion(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &WlTouch,
        time: u32,
        id: i32,
        position: (f64, f64),
    );

    /// Touch point shape change.
    ///
    /// The shape of a touch point is approximated by an ellipse through the major and minor axis
    /// length. Major always represents the larger of the two axis and is orthogonal to minor.
    ///
    /// The dimensions are specified in surface-local coordinates and the locations reported by
    /// other events always report the center of the ellipse.
    fn shape(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &WlTouch,
        id: i32,
        major: f64,
        minor: f64,
    );

    /// Touch point shape orientation.
    ///
    /// The orientation describes the clockwise angle of a touch point's major axis to the positive
    /// surface y-axis and is normalized to the -180° to +180° range.
    fn orientation(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        touch: &WlTouch,
        id: i32,
        orientation: f64,
    );

    /// Cancel active touch sequence.
    ///
    /// This indicates that the compositor has cancelled the active touch sequence, for example due
    /// to detection of a touch gesture.
    fn cancel(&mut self, conn: &Connection, qh: &QueueHandle<Self>, touch: &WlTouch);
}

impl<D, U> Dispatch<WlTouch, U, D> for SeatState
where
    D: Dispatch<WlTouch, U> + TouchHandler,
    U: TouchDataExt,
{
    fn event(
        data: &mut D,
        touch: &WlTouch,
        event: TouchEvent,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let udata = udata.touch_data();
        match event {
            // Buffer events until frame is received.
            TouchEvent::Down { .. }
            | TouchEvent::Up { .. }
            | TouchEvent::Motion { .. }
            | TouchEvent::Shape { .. }
            | TouchEvent::Orientation { .. } => {
                let mut guard = udata.inner.lock().unwrap();
                guard.events.push(event);
            }
            // Process all buffered events.
            TouchEvent::Frame => {
                let mut guard = udata.inner.lock().unwrap();
                for event in guard.events.drain(..) {
                    process_framed_event(data, touch, conn, qh, event);
                }
            }
            TouchEvent::Cancel => {
                let mut guard = udata.inner.lock().unwrap();
                guard.events.clear();

                data.cancel(conn, qh, touch);
            }
            _ => unreachable!(),
        }
    }
}

/// Process a single frame-buffered touch event.
fn process_framed_event<D>(
    data: &mut D,
    touch: &WlTouch,
    conn: &Connection,
    qh: &QueueHandle<D>,
    event: TouchEvent,
) where
    D: TouchHandler,
{
    match event {
        TouchEvent::Down { serial, time, surface, id, x, y } => {
            data.down(conn, qh, touch, serial, time, surface, id, (x, y));
        }
        TouchEvent::Up { serial, time, id } => {
            data.up(conn, qh, touch, serial, time, id);
        }
        TouchEvent::Motion { time, id, x, y } => {
            data.motion(conn, qh, touch, time, id, (x, y));
        }
        TouchEvent::Shape { id, major, minor } => {
            data.shape(conn, qh, touch, id, major, minor);
        }
        TouchEvent::Orientation { id, orientation } => {
            data.orientation(conn, qh, touch, id, orientation);
        }
        // No other events should be frame-buffered.
        _ => unreachable!(),
    }
}
