use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_output, wl_surface},
    ConnectionHandle, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_toplevel_decoration_v1,
    xdg_shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
    },
};

use crate::compositor::SurfaceData;

use self::inner::{WindowDataInner, WindowInner};

use super::{XdgShellHandler, XdgShellState};

pub(super) mod inner;

#[derive(Debug, Clone)]
pub struct WindowConfigure {
    pub new_size: Option<(u32, u32)>,
    pub states: Vec<State>,
}

pub trait WindowHandler: XdgShellHandler + Sized {
    fn request_close_window(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        window: &Window,
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationMode {
    PreferServer,

    ServerOnly,

    ClientOnly,

    None,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateWindowError {
    /// Surface already has a role object.
    #[error("surface already has a role object")]
    HasRole,

    /// The xdg_wm_base global is not available.
    #[error("the xdg_wm_base global is not available")]
    MissingRequiredGlobals,

    /// Protocol error.
    #[error(transparent)]
    Protocol(#[from] InvalidId),
}

#[derive(Debug)]
pub struct Window {
    pub(crate) inner: Arc<WindowInner>,

    /// Whether this is the primary handle to the window.
    ///
    /// This is only true for [`Window`] given the user from [`XdgShellState::create_window`]. Since we pass
    /// a reference to a [`Window`] in some traits the user implements, we need to make sure the window isn't
    /// actually destroyed while the user still holds the window. If this field is true, the drop implementation
    /// will mark the window as dead and will clean up when possible.
    pub(crate) primary: bool,

    /// Indicates whether the primary handle to the window has been destroyed.
    ///
    /// Since we can't destroy wayland objects without a connection handle, we need to mark the window for
    /// cleanup.
    pub(crate) death_signal: Arc<AtomicBool>,
}

impl Window {
    /// Map the window.
    ///
    /// This function will commit the initial window state and will result in the initial configure at some
    /// point.
    ///
    /// ## Protocol errors
    ///
    /// The [`WlSurface`] may not have any buffers attached. If a buffer is attached, a protocol error will
    /// occur.
    ///
    /// [`WlSurface`]: wl_surface::WlSurface
    pub fn map<D>(&self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        self.inner.map(conn, qh)
    }

    #[must_use]
    pub fn configure(&self) -> Option<WindowConfigure> {
        self.inner.configure()
    }

    pub fn set_title(&self, conn: &mut ConnectionHandle, title: impl Into<String>) {
        self.inner.set_title(conn, title.into())
    }

    pub fn set_app_id(&self, conn: &mut ConnectionHandle, app_id: impl Into<String>) {
        self.inner.set_app_id(conn, app_id.into())
    }

    pub fn set_min_size(&self, conn: &mut ConnectionHandle, min_size: Option<(u32, u32)>) {
        self.inner.set_min_size(conn, min_size)
    }

    /// ## Protocol errors
    ///
    /// The maximum size of the window may not be smaller than the minimum size.
    pub fn set_max_size(&self, conn: &mut ConnectionHandle, max_size: Option<(u32, u32)>) {
        self.inner.set_max_size(conn, max_size)
    }

    // TODO: Change decoration mode

    pub fn set_parent(&self, conn: &mut ConnectionHandle, parent: Option<&Window>) {
        self.inner.set_parent(conn, parent)
    }

    // TODO: Show window menu

    // TODO: Move

    // TODO: Resize

    pub fn set_maximized(&self, conn: &mut ConnectionHandle) {
        self.inner.set_maximized(conn)
    }

    pub fn unset_maximized(&self, conn: &mut ConnectionHandle) {
        self.inner.unset_maximized(conn)
    }

    pub fn set_mimimized(&self, conn: &mut ConnectionHandle) {
        self.inner.set_minmized(conn)
    }

    pub fn set_fullscreen(
        &self,
        conn: &mut ConnectionHandle,
        output: Option<&wl_output::WlOutput>,
    ) {
        self.inner.set_fullscreen(conn, output)
    }

    pub fn unset_fullscreen(&self, conn: &mut ConnectionHandle) {
        self.inner.unset_fullscreen(conn)
    }

    /// Returns the surface wrapped in this window.
    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.inner.wl_surface
    }

    /// Returns the xdg surface wrapped in this window.
    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        &self.inner.xdg_surface
    }

    /// Returns the xdg toplevel wrapped in this window.
    pub fn xdg_toplevel(&self) -> &xdg_toplevel::XdgToplevel {
        &self.inner.xdg_toplevel
    }
}

impl XdgShellState {
    /// Create a window.
    ///
    /// This function will create a window from a [`WlSurface`].
    ///
    /// ## Default settings
    ///
    /// ### Window decorations
    ///
    /// The window will use the decoration mode dictated by the compositor. If you do not want this, set the
    /// preferred decoration mode before [`map()`](Window::map)ing the window.
    ///
    /// ### Initial window size
    ///
    /// This will vary depending on the compositor. Some compositors may allow the you to make the window
    /// any desired size while others may give the you a desired size during the initial commit.
    ///
    /// You ultimately have control over what size buffer is committed, meaning you could ignore the
    /// compositor. However, not respecting the compositor will likely result in aggravated users and a subpar
    /// experience.
    ///
    /// Some compositors may take the minimum and maximum window size in consideration when determining how
    /// large of a window that will be requested during the initial commit.
    ///
    /// [`WlSurface`]: wl_surface::WlSurface
    #[must_use = "You must map() the window before presenting to it"]
    pub fn create_window<D>(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        wl_surface: wl_surface::WlSurface,
    ) -> Result<Window, CreateWindowError>
    where
        D: Dispatch<wl_surface::WlSurface, UserData = SurfaceData>
            + Dispatch<xdg_surface::XdgSurface, UserData = ()>
            + Dispatch<xdg_toplevel::XdgToplevel, UserData = WindowData>
            + 'static,
    {
        // We don't know if an un-managed surface has a role object.
        let surface_data = wl_surface.data::<SurfaceData>().ok_or(CreateWindowError::HasRole)?;

        // XDG Shell protocol forbids creating an window from a surface that already has a role object.
        if surface_data.has_role.load(Ordering::SeqCst) {
            return Err(CreateWindowError::HasRole);
        }

        let (_, xdg_wm_base) =
            self.xdg_wm_base.as_ref().ok_or(CreateWindowError::MissingRequiredGlobals)?;
        let zxdg_decoration_manager =
            self.zxdg_decoration_manager_v1.clone().map(|(_, global)| global);

        let xdg_surface = xdg_wm_base.get_xdg_surface(conn, &wl_surface, qh, ())?;
        let inner = WindowInner::new(conn, qh, &wl_surface, &xdg_surface, zxdg_decoration_manager)?;

        // Mark the surface has having a role object.
        let surface_data = wl_surface.data::<SurfaceData>().unwrap();
        surface_data.has_role.store(true, Ordering::SeqCst);

        let window =
            Window { inner, primary: true, death_signal: Arc::new(AtomicBool::new(false)) };

        self.windows.push(window.impl_clone());

        Ok(window)
    }
}

#[derive(Debug, Clone)]
pub struct WindowData(pub(crate) Arc<WindowDataInner>);
