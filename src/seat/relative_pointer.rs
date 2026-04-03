use wayland_client::{
    globals::GlobalList, protocol::wl_pointer, Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::relative_pointer::zv1::client::{
    zwp_relative_pointer_manager_v1, zwp_relative_pointer_v1,
};

use crate::{dispatch2::Dispatch2, error::GlobalError, globals::GlobalData, registry::GlobalProxy};

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

impl<D> Dispatch2<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1, D> for GlobalData
where
    D: RelativePointerHandler,
{
    fn event(
        &self,
        _data: &mut D,
        _manager: &zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1,
        _event: zwp_relative_pointer_manager_v1::Event,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch2<zwp_relative_pointer_v1::ZwpRelativePointerV1, D> for RelativePointerData
where
    D: RelativePointerHandler,
{
    fn event(
        &self,
        data: &mut D,
        relative_pointer: &zwp_relative_pointer_v1::ZwpRelativePointerV1,
        event: zwp_relative_pointer_v1::Event,
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
                    &self.wl_pointer,
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
