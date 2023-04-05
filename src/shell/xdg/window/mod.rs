//! XDG shell windows.

use std::{
    num::NonZeroU32,
    sync::{Arc, Weak},
};

use crate::reexports::client::{
    protocol::{wl_output, wl_seat, wl_surface},
    Connection, Proxy, QueueHandle,
};
use crate::reexports::csd_frame::{WindowManagerCapabilities, WindowState};
use crate::reexports::protocols::{
    xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::{self, Mode},
    xdg::shell::client::{xdg_surface, xdg_toplevel},
};

use crate::shell::WaylandSurface;

use self::inner::WindowInner;

use super::XdgSurface;

pub(super) mod inner;

/// Handler for toplevel operations on a [`Window`].
pub trait WindowHandler: Sized {
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
    /// is complete by using this function.
    ///
    /// # Double buffering
    ///
    /// Configure events in Wayland are considered to be double buffered and the state of the window does not
    /// change until committed.
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
    pub new_size: (Option<NonZeroU32>, Option<NonZeroU32>),

    /// Compositor suggested maximum bounds for a window.
    ///
    /// This may be used to ensure a window is not created in a way where it will not fit.
    ///
    /// If xdg-shell is version 3 or lower, this will always be [`None`].
    pub suggested_bounds: Option<(u32, u32)>,

    /// The compositor set decoration mode of the window.
    ///
    /// This will always be [`DecorationMode::Client`] if server side decorations are not enabled or
    /// supported.
    pub decoration_mode: DecorationMode,

    /// The current state of the window.
    ///
    /// For more see [`WindowState`] documentation on the flag values.
    pub state: WindowState,

    /// The capabilities supported by the compositor.
    ///
    /// For more see [`WindowManagerCapabilities`] documentation on the flag values.
    pub capabilities: WindowManagerCapabilities,
}

impl WindowConfigure {
    /// Is [`WindowState::MAXIMIZED`] state is set.
    #[inline]
    pub fn is_maximized(&self) -> bool {
        self.state.contains(WindowState::MAXIMIZED)
    }

    /// Is [`WindowState::FULLSCREEN`] state is set.
    #[inline]
    pub fn is_fullscreen(&self) -> bool {
        self.state.contains(WindowState::FULLSCREEN)
    }

    /// Is [`WindowState::RESIZING`] state is set.
    #[inline]
    pub fn is_resizing(&self) -> bool {
        self.state.contains(WindowState::RESIZING)
    }

    /// Is [`WindowState::TILED`] state is set.
    #[inline]
    pub fn is_tiled(&self) -> bool {
        self.state.contains(WindowState::TILED)
    }

    /// Is [`WindowState::ACTIVATED`] state is set.
    #[inline]
    pub fn is_activated(&self) -> bool {
        self.state.contains(WindowState::ACTIVATED)
    }

    /// Is [`WindowState::TILED_LEFT`] state is set.
    #[inline]
    pub fn is_tiled_left(&self) -> bool {
        self.state.contains(WindowState::TILED_LEFT)
    }

    /// Is [`WindowState::TILED_RIGHT`] state is set.
    #[inline]
    pub fn is_tiled_right(&self) -> bool {
        self.state.contains(WindowState::TILED_RIGHT)
    }

    /// Is [`WindowState::TILED_TOP`] state is set.
    #[inline]
    pub fn is_tiled_top(&self) -> bool {
        self.state.contains(WindowState::TILED_TOP)
    }

    /// Is [`WindowState::TILED_BOTTOM`] state is set.
    #[inline]
    pub fn is_tiled_bottom(&self) -> bool {
        self.state.contains(WindowState::TILED_BOTTOM)
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

#[derive(Debug, Clone)]
pub struct Window(pub(super) Arc<WindowInner>);

impl Window {
    pub fn from_xdg_toplevel(toplevel: &xdg_toplevel::XdgToplevel) -> Option<Window> {
        toplevel.data::<WindowData>().and_then(|data| data.0.upgrade()).map(Window)
    }

    pub fn from_xdg_surface(surface: &xdg_surface::XdgSurface) -> Option<Window> {
        surface.data::<WindowData>().and_then(|data| data.0.upgrade()).map(Window)
    }

    pub fn from_toplevel_decoration(
        decoration: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
    ) -> Option<Window> {
        decoration.data::<WindowData>().and_then(|data| data.0.upgrade()).map(Window)
    }

    pub fn show_window_menu(&self, seat: &wl_seat::WlSeat, serial: u32, position: (i32, i32)) {
        self.xdg_toplevel().show_window_menu(seat, serial, position.0, position.1);
    }

    pub fn set_title(&self, title: impl Into<String>) {
        self.xdg_toplevel().set_title(title.into());
    }

    pub fn set_app_id(&self, app_id: impl Into<String>) {
        self.xdg_toplevel().set_app_id(app_id.into());
    }

    pub fn set_parent(&self, parent: Option<&Window>) {
        self.xdg_toplevel().set_parent(parent.map(Window::xdg_toplevel));
    }

    pub fn set_maximized(&self) {
        self.xdg_toplevel().set_maximized()
    }

    pub fn unset_maximized(&self) {
        self.xdg_toplevel().unset_maximized()
    }

    pub fn set_minimized(&self) {
        self.xdg_toplevel().set_minimized()
    }

    pub fn set_fullscreen(&self, output: Option<&wl_output::WlOutput>) {
        self.xdg_toplevel().set_fullscreen(output)
    }

    pub fn unset_fullscreen(&self) {
        self.xdg_toplevel().unset_fullscreen()
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
        if let Some(toplevel_decoration) = &self.0.toplevel_decoration {
            match mode {
                Some(DecorationMode::Client) => toplevel_decoration.set_mode(Mode::ClientSide),
                Some(DecorationMode::Server) => toplevel_decoration.set_mode(Mode::ServerSide),
                None => toplevel_decoration.unset_mode(),
            }
        }
    }

    pub fn move_(&self, seat: &wl_seat::WlSeat, serial: u32) {
        self.xdg_toplevel()._move(seat, serial)
    }

    pub fn resize(&self, seat: &wl_seat::WlSeat, serial: u32, edges: xdg_toplevel::ResizeEdge) {
        self.xdg_toplevel().resize(seat, serial, edges)
    }

    // Double buffered window state

    pub fn set_min_size(&self, min_size: Option<(u32, u32)>) {
        let min_size = min_size.unwrap_or_default();
        self.xdg_toplevel().set_min_size(min_size.0 as i32, min_size.1 as i32);
    }

    /// # Protocol errors
    ///
    /// The maximum size of the window may not be smaller than the minimum size.
    pub fn set_max_size(&self, max_size: Option<(u32, u32)>) {
        let max_size = max_size.unwrap_or_default();
        self.xdg_toplevel().set_max_size(max_size.0 as i32, max_size.1 as i32);
    }

    // Other

    /// Returns the underlying xdg toplevel wrapped by this window.
    pub fn xdg_toplevel(&self) -> &xdg_toplevel::XdgToplevel {
        &self.0.xdg_toplevel
    }
}

impl WaylandSurface for Window {
    fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.0.xdg_surface.wl_surface()
    }
}

impl XdgSurface for Window {
    fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        self.0.xdg_surface.xdg_surface()
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Clone)]
pub struct WindowData(pub(crate) Weak<WindowInner>);

#[macro_export]
macro_rules! delegate_xdg_window {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::xdg::shell::client::xdg_surface::XdgSurface: $crate::shell::xdg::window::WindowData
        ] => $crate::shell::xdg::XdgShell);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::xdg::shell::client::xdg_toplevel::XdgToplevel: $crate::shell::xdg::window::WindowData
        ] => $crate::shell::xdg::XdgShell);
    };
}
