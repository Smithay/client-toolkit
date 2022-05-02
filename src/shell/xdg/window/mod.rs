use std::sync::{atomic::AtomicBool, Arc, Mutex};

use wayland_client::{
    protocol::{wl_output, wl_seat, wl_surface},
    ConnectionHandle, Dispatch, QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1,
        zxdg_toplevel_decoration_v1::{self, Mode},
    },
    xdg_shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
    },
};

use crate::error::GlobalError;

use self::inner::{WindowDataInner, WindowInner};

use super::{ConfigureHandler, XdgShellHandler, XdgShellState, XdgSurfaceData};

pub(super) mod inner;

#[derive(Debug)]
pub struct XdgWindowState {
    // (name, global)
    xdg_decoration_manager: Option<(u32, zxdg_decoration_manager_v1::ZxdgDecorationManagerV1)>,
    windows: Vec<Window>,
}

impl XdgWindowState {
    pub fn new() -> XdgWindowState {
        XdgWindowState { xdg_decoration_manager: None, windows: vec![] }
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
}

pub trait WindowHandler: XdgShellHandler + Sized {
    fn xdg_window_state(&mut self) -> &mut XdgWindowState;

    /// Called when a window has been requested to close.
    ///
    /// This request does not destroy the window. You must drop the [`Window`] for the window to be destroyed.
    ///
    /// This may be sent at any time, whether it is the client side window decorations or the compositor.
    fn request_close(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        window: &Window,
    );

    /// Called when the compositor has sent a configure event to an XdgSurface
    ///
    /// A configure atomically indicates that a sequence of events describing how a surface has changed have
    /// all been sent.
    fn configure(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        serial: u32,
    );
}

/// Decoration mode of a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationMode {
    /// The window should draw client side decorations.
    Client,

    /// The server will draw window decorations.
    Server,
}

/// The configure state of a window
///
/// This type indicates compositor changes to the window, such as a new size and if the state of the window
/// has changed.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct WindowConfigure {
    /// The compositor suggested new size of the window.
    ///
    /// If this value is [`None`], you may set the size of the window as you wish.
    pub new_size: Option<(u32, u32)>,

    /// Compositor suggested maximum bounds for a window.
    ///
    /// This may be used to ensure a window is not created in a way where it will not fit.
    pub suggested_bounds: Option<(u32, u32)>,

    /// The compositor set decoration mode of the window.
    ///
    /// This will always be [`DecorationMode::Client`] if server side decorations are not enabled or
    /// supported.
    pub decoration_mode: DecorationMode,

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
    /// | [`Maximized`](State::Maximized) | no | the window geometry must be obeyed. Drop shadows should also been hidden. |
    /// | [`Fullscreen`](State::Fullscreen) | no | the window geometry is a maximum. A smaller size may be used but letterboxes will appear. |
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

/// Decorations a window is created with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDecorations {
    /// The window should use the decoration mode the server asks for.
    ///
    /// The server may ask the client to render with or without client side decorations. If server side
    /// decorations are not available, client side decorations are drawn instead.
    ServerDefault,

    /// The window should request server side decorations.
    ///
    /// The server may ignore this request and ask the client to render with client side decorations. If
    /// server side decorations are not available, client side decorations are drawn instead.
    RequestServer,

    /// The window should request client side decorations.
    ///
    /// The server may ignore this request and render server side decorations. If server side decorations are
    /// not available, client side decorations are drawn.
    RequestClient,

    /// The window should always draw it's own client side decorations.
    ClientOnly,

    /// The window should use server side decorations or draw any client side decorations.
    None,
}

#[derive(Debug)]
pub struct WindowBuilder {
    title: Option<String>,
    app_id: Option<String>,
    min_size: Option<(u32, u32)>,
    max_size: Option<(u32, u32)>,
    parent: Option<xdg_toplevel::XdgToplevel>,
    fullscreen: Option<wl_output::WlOutput>,
    maximized: bool,
    decorations: WindowDecorations,
}

impl WindowBuilder {
    /// Set the title of the window being built.
    pub fn title(self, title: impl Into<String>) -> Self {
        Self { title: Some(title.into()), ..self }
    }

    /// Set the app id of the window being built.
    ///
    /// This may be used as a compositor hint to influence how the window is initially configured.
    pub fn app_id(self, app_id: impl Into<String>) -> Self {
        Self { app_id: Some(app_id.into()), ..self }
    }

    /// Suggested the minimum size of the window being built.
    ///
    /// This may be used as a compositor hint to send an initial configure with the specified minimum size.
    pub fn min_size(self, min_size: (u32, u32)) -> Self {
        Self { min_size: Some(min_size), ..self }
    }

    /// Suggest the maximum size of the window being built.
    ///
    /// This may be used as a compositor hint to send an initial configure with the specified maximum size.
    pub fn max_size(self, max_size: (u32, u32)) -> Self {
        Self { max_size: Some(max_size), ..self }
    }

    /// Set the parent window of the window being built.
    pub fn parent(self, parent: &Window) -> Self {
        Self { parent: Some(parent.xdg_toplevel().clone()), ..self }
    }

    /// Suggest the window should be created full screened.
    ///
    /// This may be used as a compositor hint to send an initial configure with the window in a full screen
    /// state.
    pub fn fullscreen(self, output: &wl_output::WlOutput) -> Self {
        Self { fullscreen: Some(output.clone()), ..self }
    }

    /// Suggest the window should be created maximized.
    ///
    /// This may be used as a compositor hint to send an initial configure with the window maximized.
    pub fn maximized(self) -> Self {
        Self { maximized: true, ..self }
    }

    /// Sets the decoration mode the window should be created with.
    ///
    /// By default the decoration mode is set to [`DecorationMode::RequestServer`] to use server provided
    /// decorations where possible.
    pub fn decorations(self, decorations: WindowDecorations) -> Self {
        Self { decorations, ..self }
    }

    /// Build and map the window
    ///
    /// This function will create the window and send the initial commit.
    ///
    /// # Protocol errors
    ///
    /// If the surface already has a role object, the compositor will raise a protocol error.
    ///
    /// A surface is considered to have a role object if some other type of surface was created using the
    /// surface. For example, creating a window, popup, layer or subsurface all assign a role object to a
    /// surface.
    ///
    /// The function here takes an owned reference to the surface to hint the surface will be consumed by the
    /// window.
    ///
    /// [`WlSurface`]: wl_surface::WlSurface
    #[must_use = "The window is destroyed if dropped"]
    pub fn map<D>(
        self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        shell_state: &XdgShellState<D>,
        window_state: &mut XdgWindowState,
        surface: wl_surface::WlSurface,
    ) -> Result<Window, GlobalError>
    where
        D: Dispatch<xdg_surface::XdgSurface, UserData = XdgSurfaceData<D>>
            + Dispatch<xdg_toplevel::XdgToplevel, UserData = WindowData>
            + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + WindowHandler
            + 'static,
    {
        let decoration_manager =
            window_state.xdg_decoration_manager.as_ref().map(|(_, global)| global);

        let data = Arc::new(WindowDataInner {
            pending_configure: Mutex::new(WindowConfigure {
                new_size: None,
                suggested_bounds: None,
                // Initial configure will indicate whether there are server side decorations.
                decoration_mode: DecorationMode::Client,
                states: Vec::new(),
            }),
        });

        let xdg_surface = shell_state.create_xdg_surface(
            conn,
            qh,
            surface,
            WindowConfigureHandler { data: data.clone() },
        )?;

        let window_data = WindowData(data);
        let xdg_toplevel = xdg_surface.xdg_surface().get_toplevel(conn, qh, window_data.clone())?;

        // If server side decorations are available, create the toplevel decoration.
        let toplevel_decoration = if let Some(decoration_manager) = decoration_manager {
            match self.decorations {
                // Window does not want any server side decorations.
                WindowDecorations::ClientOnly | WindowDecorations::None => None,

                _ => {
                    // Create the toplevel decoration.
                    let toplevel_decoration = decoration_manager
                        .get_toplevel_decoration(conn, &xdg_toplevel, qh, window_data)
                        .expect("failed to create toplevel decoration");

                    // Tell the compositor we would like a specific mode.
                    let mode = match self.decorations {
                        WindowDecorations::RequestServer => Some(Mode::ServerSide),
                        WindowDecorations::RequestClient => Some(Mode::ClientSide),
                        _ => None,
                    };

                    if let Some(mode) = mode {
                        toplevel_decoration.set_mode(conn, mode);
                    }

                    Some(toplevel_decoration)
                }
            }
        } else {
            None
        };

        let inner = Arc::new(WindowInner { xdg_surface, xdg_toplevel, toplevel_decoration });

        let window =
            Window { inner, primary: true, death_signal: Arc::new(AtomicBool::new(false)) };

        window_state.windows.push(window.impl_clone());

        // Apply state from builder
        if let Some(title) = self.title {
            window.set_title(conn, title);
        }

        if let Some(app_id) = self.app_id {
            window.set_app_id(conn, app_id);
        }

        if let Some(min_size) = self.min_size {
            window.set_min_size(conn, Some(min_size));
        }

        if let Some(max_size) = self.max_size {
            window.set_max_size(conn, Some(max_size));
        }

        if let Some(parent) = self.parent {
            window.xdg_toplevel().set_parent(conn, Some(&parent));
        }

        if let Some(output) = self.fullscreen {
            window.set_fullscreen(conn, Some(&output));
        }

        if self.maximized {
            window.set_maximized(conn);
        }

        // Initial commit
        window.wl_surface().commit(conn);

        Ok(window)
    }
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
    pub fn builder() -> WindowBuilder {
        WindowBuilder {
            title: None,
            app_id: None,
            min_size: None,
            max_size: None,
            parent: None,
            fullscreen: None,
            maximized: false,
            decorations: WindowDecorations::RequestServer,
        }
    }

    pub fn show_window_menu(
        &self,
        conn: &mut ConnectionHandle,
        seat: &wl_seat::WlSeat,
        serial: u32,
        position: (u32, u32),
    ) {
        self.inner.show_window_menu(conn, seat, serial, position.0, position.1)
    }

    pub fn set_title(&self, conn: &mut ConnectionHandle, title: impl Into<String>) {
        self.inner.set_title(conn, title.into())
    }

    pub fn set_app_id(&self, conn: &mut ConnectionHandle, app_id: impl Into<String>) {
        self.inner.set_app_id(conn, app_id.into())
    }

    pub fn set_parent(&self, conn: &mut ConnectionHandle, parent: Option<&Window>) {
        self.inner.set_parent(conn, parent)
    }

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

    /// Requests the window should use the specified decoration mode.
    ///
    /// A mode of [`None`] indicates that the window does not care what type of decorations are used.
    ///
    /// The compositor will respond with a [`configure`](WindowHandler::configure). The configure will
    /// indicate whether the window's decoration mode has changed.
    ///
    /// # Configure loops
    ///
    /// You should avoid sending multiple decoration mode requests to ensure you do not enter a configure loop.
    pub fn request_decoration_mode(
        &self,
        conn: &mut ConnectionHandle,
        mode: Option<DecorationMode>,
    ) {
        self.inner.request_decoration_mode(conn, mode)
    }

    // TODO: Move

    // TODO: Resize

    // Double buffered window state

    pub fn set_min_size(&self, conn: &mut ConnectionHandle, min_size: Option<(u32, u32)>) {
        self.inner.set_min_size(conn, min_size)
    }

    /// # Protocol errors
    ///
    /// The maximum size of the window may not be smaller than the minimum size.
    pub fn set_max_size(&self, conn: &mut ConnectionHandle, max_size: Option<(u32, u32)>) {
        self.inner.set_max_size(conn, max_size)
    }

    // TODO: Window geometry

    // TODO: WlSurface stuff

    // Other

    /// Returns the underlying surface wrapped by this window.
    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.inner.xdg_surface.wl_surface()
    }

    /// Returns the underlying xdg surface wrapped by this window.
    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        self.inner.xdg_surface.xdg_surface()
    }

    /// Returns the underlying xdg toplevel wrapped by this window.
    pub fn xdg_toplevel(&self) -> &xdg_toplevel::XdgToplevel {
        &self.inner.xdg_toplevel
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
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

struct WindowConfigureHandler {
    data: Arc<WindowDataInner>,
}

impl<D> ConfigureHandler<D> for WindowConfigureHandler
where
    D: WindowHandler,
{
    fn configure(
        &self,
        data: &mut D,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        xdg_surface: &xdg_surface::XdgSurface,
        serial: u32,
    ) {
        if let Some(window) = data.xdg_window_state().window_by_xdg(xdg_surface) {
            let window = window.impl_clone();
            let configure = { self.data.pending_configure.lock().unwrap().clone() };

            WindowHandler::configure(data, conn, qh, &window, configure, serial);
        }
    }
}
