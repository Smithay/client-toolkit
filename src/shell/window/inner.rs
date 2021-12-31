use std::sync::Arc;

use wayland_client::{protocol::wl_surface, ConnectionHandle, Dispatch, QueueHandle};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
    },
    xdg_shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
    },
};

use crate::compositor::SurfaceData;

use super::{
    rust::RustWindow, DecorationMode, WindowData, WindowError, XdgShellState, XdgSurfaceData,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum WindowInner {
    // TODO: Libdecor
    Rust(RustWindow),
}

impl WindowInner {
    pub fn new<D>(
        shell: &mut XdgShellState,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        surface: wl_surface::WlSurface,
        decoration_mode: DecorationMode,
    ) -> Result<Arc<WindowInner>, WindowError>
    where
        D: Dispatch<wl_surface::WlSurface, UserData = SurfaceData>
            + Dispatch<xdg_surface::XdgSurface, UserData = XdgSurfaceData>
            + Dispatch<xdg_toplevel::XdgToplevel, UserData = WindowData>
            + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        let inner = Arc::new(WindowInner::Rust(RustWindow::new(
            shell,
            conn,
            qh,
            surface,
            decoration_mode,
        )?));

        shell.windows.push(Arc::downgrade(&inner));

        Ok(inner)
    }

    pub fn map(&self, conn: &mut ConnectionHandle) {
        match self {
            WindowInner::Rust(window) => window.wl_surface.commit(conn),
        }
    }

    pub fn set_min_size(&self, conn: &mut ConnectionHandle, min_size: (u32, u32)) {
        match self {
            WindowInner::Rust(window) => {
                window.xdg_toplevel.set_min_size(conn, min_size.0 as i32, min_size.1 as i32);

                let mut toplevel = window.toplevel.lock().unwrap();
                toplevel.min_size = min_size;
            }
        }
    }

    pub fn set_max_size(&self, conn: &mut ConnectionHandle, max_size: (u32, u32)) {
        match self {
            WindowInner::Rust(window) => {
                window.xdg_toplevel.set_max_size(conn, max_size.0 as i32, max_size.1 as i32);

                let mut toplevel = window.toplevel.lock().unwrap();
                toplevel.max_size = max_size;
            }
        }
    }

    pub fn set_title(&self, conn: &mut ConnectionHandle, title: String) {
        match self {
            WindowInner::Rust(window) => {
                window.xdg_toplevel.set_title(conn, title.clone());

                let mut toplevel = window.toplevel.lock().unwrap();
                toplevel.title = Some(title);
            }
        }
    }

    pub fn set_app_id(&self, conn: &mut ConnectionHandle, app_id: String) {
        match self {
            WindowInner::Rust(window) => {
                window.xdg_toplevel.set_app_id(conn, app_id.clone());

                let mut toplevel = window.toplevel.lock().unwrap();
                toplevel.app_id = Some(app_id);
            }
        }
    }

    pub fn xdg_toplevel(&self) -> &xdg_toplevel::XdgToplevel {
        match self {
            WindowInner::Rust(window) => &window.xdg_toplevel,
        }
    }

    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        match self {
            WindowInner::Rust(window) => &window.xdg_surface,
        }
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        match self {
            WindowInner::Rust(window) => &window.wl_surface,
        }
    }
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
