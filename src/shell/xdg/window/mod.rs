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
        xdg_wm_base,
    },
};

use crate::compositor::Surface;
use crate::error::GlobalError;
use crate::globals::ProvidesBoundGlobal;
use crate::registry::GlobalProxy;

use self::inner::{WindowDataInner, WindowInner};

use super::{XdgShellHandler, XdgShellSurface};

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

/// Handler for toplevel operations on a [`Window`].
pub trait WindowHandler: XdgShellHandler + Sized {
    fn xdg_window_state(&mut self) -> &mut XdgWindowState;

    /// Request to close a window.
    ///
    /// This request does not destroy the window. You must drop all [`Window`] handles to destroy the window.
    /// This request may be sent either by the compositor or by some other mechanism (such as client side decorations).
    fn request_close(&mut self, conn: &Connection, qh: &QueueHandle<Self>, window: &Window);

    /// Apply a suggested surface change.
    ///
    /// When this function is called, the compositor is requesting the window's size or state to change.
    ///
    /// Internally this function is called when the underlying `xdg_surface` is configured. Any extension
    /// protocols that interface with xdg-shell are able to be notified that the surface's configure sequence
    /// is complete.
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

/// A window configure.
///
/// A configure describes a compositor request to resize the window or change it's state.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct WindowConfigure {
    /// The compositor suggested new size of the window in window geometry coordinates.
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

    /// Window states.
    ///
    /// Depending on which states are set, the allowed size of the window may change. States may also be
    /// combined. For example, a window could be activated and maximized at the same time.
    /// Along side this [`Vec`] of states, there are also helper functions that are part of [`WindowConfigure`]
    /// to test if some particular state is set.
    ///
    /// Below is a table explains the constraints a window needs to obey depending on the set states:
    ///
    /// | State(s) | Any size | Notes |
    /// |-------|----------|-------|
    /// | No states | yes ||
    /// | [`Maximized`](State::Maximized) | no | The window geometry must be obeyed. Drop shadows should also been hidden. |
    /// | [`Fullscreen`](State::Fullscreen) | no[^fullscreen] | The window geometry is the maximum allowed size. |
    /// | [`Resizing`](State::Resizing) | no[^resizing] | The window geometry is the maximum allowed size. |
    /// | [`Activated`](State::Activated) | yes | If the client provides window decorations, the decorations should be drawn as if the window is active. |
    ///
    /// There are also states that indicate the sides of a window which are tiled. Tiling is a hint which
    /// indicates what sides of a window should probably not be resized and may be used to hide shadows on tiled
    /// edges.
    ///
    /// Tiling values include:
    /// - [`Left`](State::TiledLeft)
    /// - [`Right`](State::TiledRight)
    /// - [`Top`](State::TiledTop)
    /// - [`Bottom`](State::TiledBottom)
    ///
    /// [^fullscreen]: A smaller size buffer may be used, but letterboxing or cropping could occur.
    ///
    /// [^resizing]: If you have cell sizing or a fixed aspect ratio, a smaller size buffer may be used.
    pub states: Vec<State>,
    // TODO: wm capabilities added in version 5.
}

impl WindowConfigure {
    /// Is [`State::Maximized`] the state is set.
    pub fn is_maximized(&self) -> bool {
        self.states.iter().any(|&state| state == State::Maximized)
    }

    /// Is [`State::Fullscreen`] the state is set.
    pub fn is_fullscreen(&self) -> bool {
        self.states.iter().any(|&state| state == State::Fullscreen)
    }

    /// Is [`State::Resizing`] the state is set.
    pub fn is_resizing(&self) -> bool {
        self.states.iter().any(|&state| state == State::Resizing)
    }

    /// Is [`State::Activated`] the state is set.
    pub fn is_activated(&self) -> bool {
        self.states.iter().any(|&state| state == State::Activated)
    }

    /// Is [`State::TiledLeft`] the state is set.
    pub fn is_tiled_left(&self) -> bool {
        self.states.iter().any(|&state| state == State::TiledLeft)
    }

    /// Is [`State::TiledRight`] the state is set.
    pub fn is_tiled_right(&self) -> bool {
        self.states.iter().any(|&state| state == State::TiledRight)
    }

    /// Is [`State::TiledTop`] the state is set.
    pub fn is_tiled_top(&self) -> bool {
        self.states.iter().any(|&state| state == State::TiledTop)
    }

    /// Is [`State::TiledBottom`] the state is set.
    pub fn is_tiled_bottom(&self) -> bool {
        self.states.iter().any(|&state| state == State::TiledBottom)
    }
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
        wm_base: &impl ProvidesBoundGlobal<xdg_wm_base::XdgWmBase, 4>,
        window_state: &mut XdgWindowState,
        surface: impl Into<Surface>,
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

        let xdg_surface = XdgShellSurface::new(wm_base, qh, surface, window_data.clone())?;

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
            $crate::reexports::protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1: $crate::globals::GlobalData,
            $crate::reexports::protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1: $crate::shell::xdg::window::WindowData,
        ] => $crate::shell::xdg::window::XdgWindowState);
    };
}
