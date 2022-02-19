//! ## Cross desktop group (XDG) shell
// TODO: Examples

use wayland_client::{ConnectionHandle, QueueHandle};
use wayland_protocols::xdg_shell::client::{xdg_surface, xdg_wm_base};

mod inner;
pub mod popup;
pub mod window;

#[derive(Debug)]
pub struct XdgShellState {
    // (name, global)
    xdg_wm_base: Option<(u32, xdg_wm_base::XdgWmBase)>,
}

impl XdgShellState {
    pub fn new() -> XdgShellState {
        XdgShellState { xdg_wm_base: None }
    }

    pub fn xdg_wm_base(&self) -> Option<&xdg_wm_base::XdgWmBase> {
        self.xdg_wm_base.as_ref().map(|(_, global)| global)
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
