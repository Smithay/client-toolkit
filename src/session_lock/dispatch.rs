use crate::globals::GlobalData;
use std::sync::atomic::Ordering;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1, ext_session_lock_surface_v1, ext_session_lock_v1,
};

use super::{
    SessionLock, SessionLockData, SessionLockHandler, SessionLockState, SessionLockSurface,
    SessionLockSurfaceConfigure, SessionLockSurfaceData,
};

impl<D> Dispatch<ext_session_lock_manager_v1::ExtSessionLockManagerV1, GlobalData, D>
    for SessionLockState
where
    D: Dispatch<ext_session_lock_manager_v1::ExtSessionLockManagerV1, GlobalData>,
{
    fn event(
        _state: &mut D,
        _proxy: &ext_session_lock_manager_v1::ExtSessionLockManagerV1,
        _event: ext_session_lock_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<ext_session_lock_v1::ExtSessionLockV1, SessionLockData, D> for SessionLockState
where
    D: Dispatch<ext_session_lock_v1::ExtSessionLockV1, SessionLockData> + SessionLockHandler,
{
    fn event(
        state: &mut D,
        proxy: &ext_session_lock_v1::ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        _: &SessionLockData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        if let Some(session_lock) = SessionLock::from_ext_session_lock(proxy) {
            match event {
                ext_session_lock_v1::Event::Locked => {
                    session_lock.0.locked.store(true, Ordering::SeqCst);
                    state.locked(conn, qh, session_lock);
                }
                ext_session_lock_v1::Event::Finished => {
                    state.finished(conn, qh, session_lock);
                }
                _ => unreachable!(),
            }
        }
    }
}

impl<D> Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, SessionLockSurfaceData, D>
    for SessionLockState
where
    D: Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, SessionLockSurfaceData>
        + SessionLockHandler,
{
    fn event(
        state: &mut D,
        proxy: &ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        _: &SessionLockSurfaceData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        if let Some(session_lock_surface) = SessionLockSurface::from_ext_session_lock_surface(proxy)
        {
            match event {
                ext_session_lock_surface_v1::Event::Configure { serial, width, height } => {
                    proxy.ack_configure(serial);
                    state.configure(
                        conn,
                        qh,
                        session_lock_surface,
                        SessionLockSurfaceConfigure { new_size: (width, height) },
                        serial,
                    );
                }
                _ => unreachable!(),
            }
        }
    }
}
