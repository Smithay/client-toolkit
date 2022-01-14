//! ## Cross desktop group (XDG) shell
// TODO: Examples

use wayland_client::{ConnectionHandle, QueueHandle};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1,
    xdg_shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base},
};

use self::window::Window;

mod inner;
pub mod popup;
pub mod window;

#[derive(Debug)]
pub struct XdgShellState {
    // (name, global)
    xdg_wm_base: Option<(u32, xdg_wm_base::XdgWmBase)>,
    zxdg_decoration_manager_v1: Option<(u32, zxdg_decoration_manager_v1::ZxdgDecorationManagerV1)>,

    /// We hold strong references to the window
    windows: Vec<Window>,
}

impl XdgShellState {
    pub fn new() -> XdgShellState {
        XdgShellState { xdg_wm_base: None, zxdg_decoration_manager_v1: None, windows: vec![] }
    }

    pub fn window_by_surface(&self, surface: &xdg_surface::XdgSurface) -> Option<&Window> {
        self.windows.iter().find(|window| window.xdg_surface() == surface)
    }

    pub fn window_by_toplevel(&self, toplevel: &xdg_toplevel::XdgToplevel) -> Option<&Window> {
        self.windows.iter().find(|window| window.xdg_toplevel() == toplevel)
    }
}

pub trait XdgShellHandler: Sized {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState;

    /// Called when the compositor has sent a configure event to an XdgSurface
    ///
    /// A configure atomically indicates that a sequence of events describing how a surface has changed have
    /// all been sent.
    ///
    /// When this event is received, you can get information about the configure off the extending type of
    /// the XdgSurface. For example, the window's configure is available by calling [`Window::configure`].
    fn configure(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        surface: &xdg_surface::XdgSurface,
    );
}

#[macro_export]
macro_rules! delegate_xdg_shell {
    ($ty: ty) => {
        type __XdgWmBase = $crate::reexports::protocols::xdg_shell::client::xdg_wm_base::XdgWmBase;
        type __XdgSurface = $crate::reexports::protocols::xdg_shell::client::xdg_surface::XdgSurface;

        // TODO: Popups

        $crate::reexports::client::delegate_dispatch!($ty: [
            __XdgWmBase,
            __XdgSurface
        ] => $crate::shell::xdg::XdgShellState);
    };
}

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
        ] => $crate::shell::xdg::XdgShellState);
    };
}
