use std::sync::{Arc, Mutex};

use wayland_client::protocol::wl_surface;
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
    },
    xdg_shell::client::{
        xdg_popup, xdg_surface,
        xdg_toplevel::{self, State},
    },
};

use super::DecorationMode;

#[derive(Debug, Clone)]
pub(crate) struct WindowInner {
    pub wl_surface: wl_surface::WlSurface,
    pub xdg_surface: xdg_surface::XdgSurface,
    pub toplevel: Arc<Mutex<XdgToplevelInner>>,
    pub xdg_toplevel: xdg_toplevel::XdgToplevel,
}

impl PartialEq for WindowInner {
    fn eq(&self, other: &Self) -> bool {
        self.xdg_surface == other.xdg_surface && self.xdg_toplevel == other.xdg_toplevel
    }
}

#[derive(Debug)]
pub(crate) enum XdgSurfaceInner {
    Window(Arc<Mutex<XdgToplevelInner>>),

    Popup(Arc<Mutex<XdgPopupInner>>),

    Uninit,
}

#[derive(Debug)]
pub(crate) struct XdgToplevelInner {
    pub(crate) decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    pub(crate) decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    // Window info
    pub(crate) title: Option<String>,
    pub(crate) app_id: Option<String>,
    pub(crate) decoration_mode: DecorationMode,
    pub(crate) min_size: (u32, u32),
    pub(crate) max_size: (u32, u32),
    // Configure
    pub(crate) pending_configure: Option<PendingConfigure>,
}

#[derive(Debug)]
pub(crate) struct PendingConfigure {
    pub(crate) size: (u32, u32),
    pub(crate) states: Vec<State>,
}

#[derive(Debug)]
pub(crate) struct XdgPopupInner {
    pub(crate) popup: xdg_popup::XdgPopup,
}
