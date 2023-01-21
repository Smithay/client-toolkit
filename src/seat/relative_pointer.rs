use wayland_client::{
    globals::GlobalList, protocol::wl_pointer, Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::relative_pointer::zv1::client::{
    zwp_relative_pointer_manager_v1, zwp_relative_pointer_v1,
};

use crate::{error::GlobalError, globals::GlobalData, registry::GlobalProxy};

#[derive(Debug)]
pub struct RelativePointerState {
    relative_pointer_manager:
        GlobalProxy<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1>,
}

impl RelativePointerState {
    /// Bind `zwp_relative_pointer_manager_v1` global, if it exists
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1, GlobalData>
            + 'static,
    {
        let relative_pointer_manager = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { relative_pointer_manager }
    }

    pub fn get_relative_pointer<D>(
        &self,
        pointer: &wl_pointer::WlPointer,
        qh: &QueueHandle<D>,
    ) -> Result<zwp_relative_pointer_v1::ZwpRelativePointerV1, GlobalError>
    where
        D: Dispatch<zwp_relative_pointer_v1::ZwpRelativePointerV1, RelativePointerData> + 'static,
    {
        let udata = RelativePointerData { wl_pointer: pointer.clone() };
        Ok(self.relative_pointer_manager.get()?.get_relative_pointer(pointer, qh, udata))
    }
}

#[derive(Debug)]
pub struct RelativeMotionEvent {
    /// (x, y) motion vector
    pub delta: (f64, f64),
    /// Unaccelerated (x, y) motion vector
    pub delta_unaccel: (f64, f64),
    /// Timestamp in microseconds
    pub utime: u64,
}

pub trait RelativePointerHandler: Sized {
    fn relative_pointer_motion(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        relative_pointer: &zwp_relative_pointer_v1::ZwpRelativePointerV1,
        pointer: &wl_pointer::WlPointer,
        event: RelativeMotionEvent,
    );
}

#[doc(hidden)]
#[derive(Debug)]
pub struct RelativePointerData {
    wl_pointer: wl_pointer::WlPointer,
}

impl<D> Dispatch<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1, GlobalData, D>
    for RelativePointerState
where
    D: Dispatch<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1, GlobalData>
        + RelativePointerHandler,
{
    fn event(
        _data: &mut D,
        _manager: &zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1,
        _event: zwp_relative_pointer_manager_v1::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<zwp_relative_pointer_v1::ZwpRelativePointerV1, RelativePointerData, D>
    for RelativePointerState
where
    D: Dispatch<zwp_relative_pointer_v1::ZwpRelativePointerV1, RelativePointerData>
        + RelativePointerHandler,
{
    fn event(
        data: &mut D,
        relative_pointer: &zwp_relative_pointer_v1::ZwpRelativePointerV1,
        event: zwp_relative_pointer_v1::Event,
        udata: &RelativePointerData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zwp_relative_pointer_v1::Event::RelativeMotion {
                utime_hi,
                utime_lo,
                dx,
                dy,
                dx_unaccel,
                dy_unaccel,
            } => {
                data.relative_pointer_motion(
                    conn,
                    qh,
                    relative_pointer,
                    &udata.wl_pointer,
                    RelativeMotionEvent {
                        utime: ((utime_hi as u64) << 32) | (utime_lo as u64),
                        delta: (dx, dy),
                        delta_unaccel: (dx_unaccel, dy_unaccel),
                    },
                );
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_relative_pointer {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1: $crate::globals::GlobalData
        ] => $crate::seat::relative_pointer::RelativePointerState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_v1::ZwpRelativePointerV1: $crate::seat::relative_pointer::RelativePointerData
        ] => $crate::seat::relative_pointer::RelativePointerState);
    };
}
