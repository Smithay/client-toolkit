use crate::{compositor::Surface, error::GlobalError, globals::GlobalData, registry::GlobalProxy};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Weak,
};
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_output, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1, ext_session_lock_surface_v1, ext_session_lock_v1,
};

mod dispatch;

/// Handler trait for session lock protocol.
pub trait SessionLockHandler: Sized {
    /// The session lock is active, and the client may create lock surfaces.
    fn locked(&mut self, conn: &Connection, qh: &QueueHandle<Self>, session_lock: SessionLock);

    /// Session lock is not active and should be destroyed.
    ///
    /// This may be sent immediately if the compositor denys the requires to create a lock,
    /// or may be sent some time after `lock`.
    fn finished(&mut self, conn: &Connection, qh: &QueueHandle<Self>, session_lock: SessionLock);

    /// Compositor has requested size for surface.
    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        serial: u32,
    );
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SessionLockSurfaceConfigure {
    pub new_size: (u32, u32),
}

#[derive(Debug)]
struct SessionLockSurfaceInner {
    surface: Surface,
    session_lock_surface: ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
}

impl Drop for SessionLockSurfaceInner {
    fn drop(&mut self) {
        self.session_lock_surface.destroy();
    }
}

#[must_use]
#[derive(Debug, Clone)]
pub struct SessionLockSurface(Arc<SessionLockSurfaceInner>);

impl SessionLockSurface {
    pub fn from_ext_session_lock_surface(
        surface: &ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
    ) -> Option<Self> {
        surface.data::<SessionLockSurfaceData>().and_then(|data| data.inner.upgrade()).map(Self)
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.0.surface.wl_surface()
    }
}

#[derive(Debug)]
pub struct SessionLockSurfaceData {
    inner: Weak<SessionLockSurfaceInner>,
}

impl SessionLockSurfaceData {
    pub fn session_lock_surface(&self) -> Option<SessionLockSurface> {
        self.inner.upgrade().map(SessionLockSurface)
    }
}

#[derive(Debug)]
pub struct SessionLockInner {
    session_lock: ext_session_lock_v1::ExtSessionLockV1,
    locked: AtomicBool,
}

impl Drop for SessionLockInner {
    fn drop(&mut self) {
        // This does nothing if unlock() was called.  It may trigger a protocol error if unlock was
        // not called; this is an application bug, and choosing not to unlock here results in us
        // failing secure.
        self.session_lock.destroy();
    }
}

/// A session lock
///
/// Once a lock is created, you must wait for either a `locked` or `finished` event before
/// destroying this object.  If you get a `locked` event, you must explicitly call `unlock` prior
/// to dropping this object.
#[derive(Debug, Clone)]
pub struct SessionLock(Arc<SessionLockInner>);

impl SessionLock {
    pub fn from_ext_session_lock(surface: &ext_session_lock_v1::ExtSessionLockV1) -> Option<Self> {
        surface.data::<SessionLockData>().and_then(|data| data.inner.upgrade()).map(Self)
    }

    pub fn is_locked(&self) -> bool {
        self.0.locked.load(Ordering::SeqCst)
    }

    pub fn unlock(&self) {
        if self.0.locked.load(Ordering::SeqCst) {
            self.0.session_lock.unlock_and_destroy();
        }
    }
}

#[derive(Debug)]
pub struct SessionLockData {
    inner: Weak<SessionLockInner>,
}

impl SessionLock {
    pub fn create_lock_surface<D>(
        &self,
        surface: impl Into<Surface>,
        output: &wl_output::WlOutput,
        qh: &QueueHandle<D>,
    ) -> SessionLockSurface
    where
        D: Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, SessionLockSurfaceData>
            + 'static,
    {
        // Freeze the queue during the creation of the Arc to avoid a race between events on the
        // new objects being processed and the Weak in the SessionLockSurfaceData becoming usable.
        let freeze = qh.freeze();
        let surface = surface.into();

        let inner = Arc::new_cyclic(|weak| {
            let session_lock_surface = self.0.session_lock.get_lock_surface(
                surface.wl_surface(),
                output,
                qh,
                SessionLockSurfaceData { inner: weak.clone() },
            );

            SessionLockSurfaceInner { surface, session_lock_surface }
        });
        drop(freeze);

        SessionLockSurface(inner)
    }
}

/// A handler for [`ext_session_lock_manager_v1::ExtSessionLockManagerV1`]
#[derive(Debug)]
pub struct SessionLockState {
    session_lock_manager: GlobalProxy<ext_session_lock_manager_v1::ExtSessionLockManagerV1>,
}

impl SessionLockState {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ext_session_lock_manager_v1::ExtSessionLockManagerV1, GlobalData> + 'static,
    {
        let session_lock_manager = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { session_lock_manager }
    }

    pub fn lock<D>(&self, qh: &QueueHandle<D>) -> Result<SessionLock, GlobalError>
    where
        D: Dispatch<ext_session_lock_v1::ExtSessionLockV1, SessionLockData> + 'static,
    {
        let session_lock_manager = self.session_lock_manager.get()?;

        // Freeze the queue during the creation of the Arc to avoid a race between events on the
        // new objects being processed and the Weak in the SessionLockData becoming usable.
        let freeze = qh.freeze();

        let inner = Arc::new_cyclic(|weak| {
            let session_lock =
                session_lock_manager.lock(qh, SessionLockData { inner: weak.clone() });

            SessionLockInner { session_lock, locked: AtomicBool::new(false) }
        });
        drop(freeze);

        Ok(SessionLock(inner))
    }
}

#[macro_export]
macro_rules! delegate_session_lock {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::ext::session_lock::v1::client::ext_session_lock_manager_v1::ExtSessionLockManagerV1: $crate::globals::GlobalData
            ] => $crate::session_lock::SessionLockState
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::ext::session_lock::v1::client::ext_session_lock_v1::ExtSessionLockV1: $crate::session_lock::SessionLockData
            ] => $crate::session_lock::SessionLockState
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::ext::session_lock::v1::client::ext_session_lock_surface_v1::ExtSessionLockSurfaceV1: $crate::session_lock::SessionLockSurfaceData
            ] => $crate::session_lock::SessionLockState
        );
    };
}
