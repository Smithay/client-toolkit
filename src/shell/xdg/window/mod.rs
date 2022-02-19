use std::sync::{atomic::AtomicBool, Arc};

use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_output, wl_surface},
    ConnectionHandle, Dispatch, QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
    },
    xdg_shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
        xdg_wm_base,
    },
};

use crate::compositor::SurfaceData;

use self::inner::{WindowDataInner, WindowInner};

use super::XdgShellHandler;

pub(super) mod inner;

#[derive(Debug)]
pub struct XdgWindowState {
    // (name, global)
    xdg_wm_base: Option<(u32, xdg_wm_base::XdgWmBase)>,
    zxdg_decoration_manager_v1: Option<(u32, zxdg_decoration_manager_v1::ZxdgDecorationManagerV1)>,
    windows: Vec<Window>,
}

impl XdgWindowState {
    pub fn new() -> XdgWindowState {
        XdgWindowState { xdg_wm_base: None, zxdg_decoration_manager_v1: None, windows: vec![] }
    }

    pub fn window_by_wl(&self, surface: &wl_surface::WlSurface) -> Option<&Window> {
        self.windows.iter().find(|window| window.wl_surface() == surface)
    }

    pub fn window_by_xdg(&self, surface: &xdg_surface::XdgSurface) -> Option<&Window> {
        self.windows.iter().find(|window| window.xdg_surface() == surface)
    }

    pub fn window_by_toplevel(&self, toplevel: &xdg_toplevel::XdgToplevel) -> Option<&Window> {
        self.windows.iter().find(|window| window.xdg_toplevel() == toplevel)
    }

    /// Create a window.
    ///
    /// This function will create a window from a [`WlSurface`]. Note the window will consume the
    /// [`WlSurface`] when the window is dropped.
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
    /// # Protocol errors
    ///
    /// If the surface already has a role object, the compositor will raise a protocol error.
    ///
    /// A surface is considered to have a role object if some other type of surface was created using the
    /// surface. For example, creating a window, popup, layer or subsurface all assign a role object to a
    /// surface.
    ///
    /// The function here takes an owned reference to the surface to hint the surface will be owned by the
    /// returned window.
    ///
    /// [`WlSurface`]: wl_surface::WlSurface
    #[must_use = "dropping the window will consume the wl_surface and destroy the window"]
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
        let (_, xdg_wm_base) =
            self.xdg_wm_base.as_ref().ok_or(CreateWindowError::MissingRequiredGlobals)?;
        let zxdg_decoration_manager =
            self.zxdg_decoration_manager_v1.clone().map(|(_, global)| global);

        let xdg_surface = xdg_wm_base.get_xdg_surface(conn, &wl_surface, qh, ())?;
        let inner = WindowInner::new(conn, qh, &wl_surface, &xdg_surface, zxdg_decoration_manager)?;

        let window =
            Window { inner, primary: true, death_signal: Arc::new(AtomicBool::new(false)) };

        self.windows.push(window.impl_clone());

        Ok(window)
    }
}

pub trait WindowHandler: XdgShellHandler + Sized {
    fn xdg_window_state(&mut self) -> &mut XdgWindowState;

    /// Called when a window has been requested to close.
    ///
    /// This request does not destroy the window. You must drop the [`Window`] for the window to be destroyed.
    ///
    /// This may be sent at any time, whether it is the client side window decorations or the compositor.
    fn request_close_window(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        window: &Window,
    );
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct WindowConfigure {
    /// The compositor suggested new size of the window.
    ///
    /// If this value is [`None`], you may set the size of the window as you wish.
    pub new_size: Option<(u32, u32)>,

    /// States indicating how the window should be resized.
    ///
    /// Some states require the new size of the window to be obeyed. States may also be combined. For example,
    /// a window could be activated and maximized at the same time.
    ///
    /// Below is a table explains the constraints a window needs to obey depending on the set states:
    ///
    /// | State(s) | Any size | Notes |
    /// |-------|----------|-------|
    /// | No states | yes ||
    /// | [`Maximized`](State::Maximized) | no | the window geometry must be obeyed |
    /// | [`Fullscreen`](State::Fullscreen) | no | the window geometry is a maximum. Not obeying the size may result in letterboxes. |
    /// | [`Resizing`](State::Resizing) | no | the window geometry is a maximum. If you have cell sizing or a fixed aspect ratio, a smaller size may be used. |
    /// | [`Activated`](State::Activated) | yes? | if the client provides window decorations, the decorations should be drawn as if the window is active. |
    ///
    /// There are also states that indicate the sides of a window which are tiled. Tiling is a hint which
    /// specifies which sides of a window should probably not be resized and may be used to hide shadows.
    /// Tiling values include:
    /// - [`Left`](State::TiledLeft)
    /// - [`Right`](State::TiledRight)
    /// - [`Top`](State::TiledTop)
    /// - [`Bottom`](State::TiledBottom)
    pub states: Vec<State>,
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
    /// # Protocol errors
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

    /// # Protocol errors
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

#[derive(Debug, Clone)]
pub struct WindowData(pub(crate) Arc<WindowDataInner>);

#[macro_export]
macro_rules! delegate_xdg_window {
    ($ty: ty) => {
        // Toplevel
        type __XdgToplevel = $crate::reexports::protocols::xdg_shell::client::xdg_toplevel::XdgToplevel;
        type __ZxdgDecorationManagerV1 =
            $crate::reexports::protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
        type __ZxdgToplevelDecorationV1 =
            $crate::reexports::protocols::unstable::xdg_decoration::v1::client::zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1;

        $crate::reexports::client::delegate_dispatch!($ty: [
            __XdgToplevel,
            __ZxdgDecorationManagerV1,
            __ZxdgToplevelDecorationV1
        ] => $crate::shell::xdg::window::XdgWindowState);
    };
}
