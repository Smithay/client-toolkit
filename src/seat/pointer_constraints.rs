use wayland_client::{
    globals::GlobalList,
    protocol::{wl_pointer, wl_region, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::pointer_constraints::zv1::client::{
    zwp_confined_pointer_v1, zwp_locked_pointer_v1, zwp_pointer_constraints_v1,
};

use crate::{
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
    registry::GlobalProxy,
};

#[derive(Debug)]
pub struct PointerConstraintsState {
    pointer_constraints: GlobalProxy<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1>,
}

impl PointerConstraintsState {
    /// Bind `zwp_pointer_constraints_v1` global, if it exists
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1, GlobalData> + 'static,
    {
        let pointer_constraints = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { pointer_constraints }
    }

    /// Request that the compositor confine the pointer to a region
    ///
    /// It is a protocol error to call when a constraint already exists for a pointer on the seat.
    pub fn confine_pointer<D>(
        &self,
        surface: &wl_surface::WlSurface,
        pointer: &wl_pointer::WlPointer,
        region: Option<&wl_region::WlRegion>,
        lifetime: zwp_pointer_constraints_v1::Lifetime,
        qh: &QueueHandle<D>,
    ) -> Result<zwp_confined_pointer_v1::ZwpConfinedPointerV1, GlobalError>
    where
        D: Dispatch<zwp_confined_pointer_v1::ZwpConfinedPointerV1, PointerConstraintData> + 'static,
    {
        let udata = PointerConstraintData { surface: surface.clone(), pointer: pointer.clone() };
        Ok(self
            .pointer_constraints
            .get()?
            .confine_pointer(surface, pointer, region, lifetime, qh, udata))
    }

    /// Request that the compositor lock the pointer in place
    ///
    /// It is a protocol error to call when a constraint already exists for a pointer on the seat.
    pub fn lock_pointer<D>(
        &self,
        surface: &wl_surface::WlSurface,
        pointer: &wl_pointer::WlPointer,
        region: Option<&wl_region::WlRegion>,
        lifetime: zwp_pointer_constraints_v1::Lifetime,
        qh: &QueueHandle<D>,
    ) -> Result<zwp_locked_pointer_v1::ZwpLockedPointerV1, GlobalError>
    where
        D: Dispatch<zwp_locked_pointer_v1::ZwpLockedPointerV1, PointerConstraintData> + 'static,
    {
        let udata = PointerConstraintData { surface: surface.clone(), pointer: pointer.clone() };
        Ok(self
            .pointer_constraints
            .get()?
            .lock_pointer(surface, pointer, region, lifetime, qh, udata))
    }
}

impl ProvidesBoundGlobal<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1, 1>
    for PointerConstraintsState
{
    fn bound_global(
        &self,
    ) -> Result<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1, GlobalError> {
        self.pointer_constraints.get().cloned()
    }
}

pub trait PointerConstraintsHandler: Sized {
    /// Pointer confinement activated by compositor
    fn confined(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        confined_pointer: &zwp_confined_pointer_v1::ZwpConfinedPointerV1,
        surface: &wl_surface::WlSurface,
        pointer: &wl_pointer::WlPointer,
    );

    /// Pointer confinement deactivated by compositor
    ///
    /// For `Oneshot` constraints, it will not be reactivated.
    fn unconfined(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        confined_pointer: &zwp_confined_pointer_v1::ZwpConfinedPointerV1,
        surface: &wl_surface::WlSurface,
        pointer: &wl_pointer::WlPointer,
    );

    /// Pointer lock activated by compositor
    fn locked(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        locked_pointer: &zwp_locked_pointer_v1::ZwpLockedPointerV1,
        surface: &wl_surface::WlSurface,
        pointer: &wl_pointer::WlPointer,
    );

    /// Pointer lock deactivated by compositor
    ///
    /// For `Oneshot` constraints, it will not be reactivated.
    fn unlocked(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        locked_pointer: &zwp_locked_pointer_v1::ZwpLockedPointerV1,
        surface: &wl_surface::WlSurface,
        pointer: &wl_pointer::WlPointer,
    );
}

#[doc(hidden)]
#[derive(Debug)]
pub struct PointerConstraintData {
    surface: wl_surface::WlSurface,
    pointer: wl_pointer::WlPointer,
}

impl<D> Dispatch<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1, GlobalData, D>
    for PointerConstraintsState
where
    D: Dispatch<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1, GlobalData>
        + PointerConstraintsHandler,
{
    fn event(
        _data: &mut D,
        _constraints: &zwp_pointer_constraints_v1::ZwpPointerConstraintsV1,
        _event: zwp_pointer_constraints_v1::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<zwp_confined_pointer_v1::ZwpConfinedPointerV1, PointerConstraintData, D>
    for PointerConstraintsState
where
    D: Dispatch<zwp_confined_pointer_v1::ZwpConfinedPointerV1, PointerConstraintData>
        + PointerConstraintsHandler,
{
    fn event(
        data: &mut D,
        confined_pointer: &zwp_confined_pointer_v1::ZwpConfinedPointerV1,
        event: zwp_confined_pointer_v1::Event,
        udata: &PointerConstraintData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zwp_confined_pointer_v1::Event::Confined => {
                data.confined(conn, qh, confined_pointer, &udata.surface, &udata.pointer)
            }
            zwp_confined_pointer_v1::Event::Unconfined => {
                data.unconfined(conn, qh, confined_pointer, &udata.surface, &udata.pointer)
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<zwp_locked_pointer_v1::ZwpLockedPointerV1, PointerConstraintData, D>
    for PointerConstraintsState
where
    D: Dispatch<zwp_locked_pointer_v1::ZwpLockedPointerV1, PointerConstraintData>
        + PointerConstraintsHandler,
{
    fn event(
        data: &mut D,
        locked_pointer: &zwp_locked_pointer_v1::ZwpLockedPointerV1,
        event: zwp_locked_pointer_v1::Event,
        udata: &PointerConstraintData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zwp_locked_pointer_v1::Event::Locked => {
                data.locked(conn, qh, locked_pointer, &udata.surface, &udata.pointer)
            }
            zwp_locked_pointer_v1::Event::Unlocked => {
                data.unlocked(conn, qh, locked_pointer, &udata.surface, &udata.pointer)
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_pointer_constraints {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1::ZwpPointerConstraintsV1: $crate::globals::GlobalData
        ] => $crate::seat::pointer_constraints::PointerConstraintsState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::pointer_constraints::zv1::client::zwp_confined_pointer_v1::ZwpConfinedPointerV1: $crate::seat::pointer_constraints::PointerConstraintData
        ] => $crate::seat::pointer_constraints::PointerConstraintsState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::pointer_constraints::zv1::client::zwp_locked_pointer_v1::ZwpLockedPointerV1: $crate::seat::pointer_constraints::PointerConstraintData
        ] => $crate::seat::pointer_constraints::PointerConstraintsState);
    };
}
