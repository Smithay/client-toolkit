//! ## Cross desktop group (XDG) shell
// TODO: Examples

use std::sync::{Arc, Mutex, Weak};

use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1,
    xdg_shell::client::xdg_wm_base,
};

use self::{inner::XdgSurfaceDataInner, window::inner::WindowInner};

mod inner;
pub mod popup;
pub mod window;

#[derive(Debug)]
pub struct XdgShellState {
    // (name, global)
    xdg_wm_base: Option<(u32, xdg_wm_base::XdgWmBase)>,
    zxdg_decoration_manager_v1: Option<(u32, zxdg_decoration_manager_v1::ZxdgDecorationManagerV1)>,

    windows: Vec<Weak<WindowInner>>,
}

impl XdgShellState {
    pub fn new() -> XdgShellState {
        XdgShellState { xdg_wm_base: None, zxdg_decoration_manager_v1: None, windows: vec![] }
    }
}

pub trait XdgShellHandler: Sized {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState;
}

/// Data associated with an XDG surface created by Smithay's client toolkit.
#[derive(Debug, Clone)]
pub struct XdgSurfaceData(Arc<Mutex<XdgSurfaceDataInner>>);

#[macro_export]
macro_rules! delegate_xdg_shell {
    ($ty: ty) => {
        type __XdgWmBase = $crate::reexports::protocols::xdg_shell::client::xdg_wm_base::XdgWmBase;
        type __XdgSurface = $crate::reexports::protocols::xdg_shell::client::xdg_surface::XdgSurface;

        // Toplevel
        type __XdgToplevel = $crate::reexports::protocols::xdg_shell::client::xdg_toplevel::XdgToplevel;
        type __ZxdgDecorationManagerV1 =
            $crate::reexports::protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
        type __ZxdgToplevelDecorationV1 =
            $crate::reexports::protocols::unstable::xdg_decoration::v1::client::zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1;

        // TODO: Popups

        $crate::reexports::client::delegate_dispatch!($ty: [
            __XdgWmBase,
            __XdgSurface,

            __XdgToplevel,
            __ZxdgDecorationManagerV1,
            __ZxdgToplevelDecorationV1
        ] => $crate::shell::xdg::XdgShellState);
    };
}
