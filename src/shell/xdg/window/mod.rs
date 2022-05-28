use std::sync::{Arc, Mutex, Weak};

use wayland_client::{
    protocol::{wl_output, wl_seat, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::{
    xdg::decoration::zv1::client::{
        zxdg_decoration_manager_v1,
        zxdg_toplevel_decoration_v1::{self, Mode},
    },
    xdg::shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
    },
};

use crate::error::GlobalError;
use crate::registry::GlobalProxy;

use self::inner::{WindowDataInner, WindowInner};

use super::{XdgShellHandler, XdgShellState};

pub(super) mod inner;

#[derive(Debug)]
pub struct XdgWindowState {
    // (name, global)
    xdg_decoration_manager: GlobalProxy<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    windows: Vec<Weak<WindowInner>>,
}

impl XdgWindowState {
    pub fn new() -> XdgWindowState {
        XdgWindowState { xdg_decoration_manager: GlobalProxy::NotReady, windows: vec![] }
    }

    pub fn window_by_wl(&self, surface: &wl_surface::WlSurface) -> Option<Window> {
        self.windows
            .iter()
            .filter_map(Weak::upgrade)
            .find(|window| window.xdg_surface.wl_surface() == surface)
            .map(Window)
    }

    pub fn window_by_xdg(&self, surface: &xdg_surface::XdgSurface) -> Option<Window> {
        self.windows
            .iter()
            .filter_map(Weak::upgrade)
            .find(|window| window.xdg_surface.xdg_surface() == surface)
            .map(Window)
    }

    pub fn window_by_toplevel(&self, toplevel: &xdg_toplevel::XdgToplevel) -> Option<Window> {
        self.windows
            .iter()
            .filter_map(Weak::upgrade)
            .find(|window| &window.xdg_toplevel == toplevel)
            .map(Window)
    }
}

pub trait WindowHandler: XdgShellHandler + Sized {
    fn xdg_window_state(&mut self) -> &mut XdgWindowState;

    /// Called when a window has been requested to close.
    ///
    /// This request does not destroy the window. You must drop the [`Window`] for the window to be destroyed.
    ///
    /// This may be sent at any time, whether it is the client side window decorations or the compositor.
    fn request_close(&mut self, conn: &Connection, qh: &QueueHandle<Self>, window: &Window);

    /// Called when the compositor has sent a configure event to an XdgSurface
    ///
    /// A configure atomically indicates that a sequence of events describing how a surface has changed have
    /// all been sent.
    fn configure(
        &mut self,
        conn: &Connection,
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
        qh: &QueueHandle<D>,
        shell_state: &XdgShellState,
        window_state: &mut XdgWindowState,
        surface: wl_surface::WlSurface,
    ) -> Result<Window, GlobalError>
    where
        D: Dispatch<xdg_surface::XdgSurface, WindowData>
            + Dispatch<xdg_toplevel::XdgToplevel, WindowData>
            + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, WindowData>
            + WindowHandler
            + 'static,
    {
        let decoration_manager = window_state.xdg_decoration_manager.get().ok();

        let data = Arc::new(WindowDataInner {
            pending_configure: Mutex::new(WindowConfigure {
                new_size: None,
                suggested_bounds: None,
                // Initial configure will indicate whether there are server side decorations.
                decoration_mode: DecorationMode::Client,
                states: Vec::new(),
            }),
        });
        let window_data = WindowData(data);

        let xdg_surface = shell_state.create_xdg_surface(qh, surface, window_data.clone())?;

        let xdg_toplevel = xdg_surface.xdg_surface().get_toplevel(qh, window_data.clone())?;

        // If server side decorations are available, create the toplevel decoration.
        let toplevel_decoration = if let Some(decoration_manager) = decoration_manager {
            match self.decorations {
                // Window does not want any server side decorations.
                WindowDecorations::ClientOnly | WindowDecorations::None => None,

                _ => {
                    // Create the toplevel decoration.
                    let toplevel_decoration = decoration_manager
                        .get_toplevel_decoration(&xdg_toplevel, qh, window_data)
                        .expect("failed to create toplevel decoration");

                    // Tell the compositor we would like a specific mode.
                    let mode = match self.decorations {
                        WindowDecorations::RequestServer => Some(Mode::ServerSide),
                        WindowDecorations::RequestClient => Some(Mode::ClientSide),
                        _ => None,
                    };

                    if let Some(mode) = mode {
                        toplevel_decoration.set_mode(mode);
                    }

                    Some(toplevel_decoration)
                }
            }
        } else {
            None
        };

        let window =
            Window(Arc::new(WindowInner { xdg_surface, xdg_toplevel, toplevel_decoration }));

        window_state.windows.push(Arc::downgrade(&window.0));

        // Apply state from builder
        if let Some(title) = self.title {
            window.set_title(title);
        }

        if let Some(app_id) = self.app_id {
            window.set_app_id(app_id);
        }

        if let Some(min_size) = self.min_size {
            window.set_min_size(Some(min_size));
        }

        if let Some(max_size) = self.max_size {
            window.set_max_size(Some(max_size));
        }

        if let Some(parent) = self.parent {
            window.xdg_toplevel().set_parent(Some(&parent));
        }

        if let Some(output) = self.fullscreen {
            window.set_fullscreen(Some(&output));
        }

        if self.maximized {
            window.set_maximized();
        }

        // Initial commit
        window.wl_surface().commit();

        Ok(window)
    }
}

#[derive(Debug, Clone)]
pub struct Window(Arc<WindowInner>);

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

    pub fn show_window_menu(&self, seat: &wl_seat::WlSeat, serial: u32, position: (u32, u32)) {
        self.0.show_window_menu(seat, serial, position.0, position.1)
    }

    pub fn set_title(&self, title: impl Into<String>) {
        self.0.set_title(title.into())
    }

    pub fn set_app_id(&self, app_id: impl Into<String>) {
        self.0.set_app_id(app_id.into())
    }

    pub fn set_parent(&self, parent: Option<&Window>) {
        self.0.set_parent(parent)
    }

    pub fn set_maximized(&self) {
        self.0.set_maximized()
    }

    pub fn unset_maximized(&self) {
        self.0.unset_maximized()
    }

    pub fn set_mimimized(&self) {
        self.0.set_minmized()
    }

    pub fn set_fullscreen(&self, output: Option<&wl_output::WlOutput>) {
        self.0.set_fullscreen(output)
    }

    pub fn unset_fullscreen(&self) {
        self.0.unset_fullscreen()
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
    pub fn request_decoration_mode(&self, mode: Option<DecorationMode>) {
        self.0.request_decoration_mode(mode)
    }

    // TODO: Move

    // TODO: Resize

    // Double buffered window state

    pub fn set_min_size(&self, min_size: Option<(u32, u32)>) {
        self.0.set_min_size(min_size)
    }

    /// # Protocol errors
    ///
    /// The maximum size of the window may not be smaller than the minimum size.
    pub fn set_max_size(&self, max_size: Option<(u32, u32)>) {
        self.0.set_max_size(max_size)
    }

    // TODO: Window geometry

    // TODO: WlSurface stuff

    // Other

    /// Returns the underlying surface wrapped by this window.
    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.0.xdg_surface.wl_surface()
    }

    /// Returns the underlying xdg surface wrapped by this window.
    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        self.0.xdg_surface.xdg_surface()
    }

    /// Returns the underlying xdg toplevel wrapped by this window.
    pub fn xdg_toplevel(&self) -> &xdg_toplevel::XdgToplevel {
        &self.0.xdg_toplevel
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Clone)]
pub struct WindowData(pub(crate) Arc<WindowDataInner>);

#[macro_export]
macro_rules! delegate_xdg_window {
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty: [
            $crate::reexports::protocols::xdg::shell::client::xdg_surface::XdgSurface: $crate::shell::xdg::window::WindowData,
            $crate::reexports::protocols::xdg::shell::client::xdg_toplevel::XdgToplevel: $crate::shell::xdg::window::WindowData,
            $crate::reexports::protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1: (),
            $crate::reexports::protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1: $crate::shell::xdg::window::WindowData,
        ] => $crate::shell::xdg::window::XdgWindowState);
    };
}
