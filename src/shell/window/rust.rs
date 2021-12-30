use std::sync::{atomic::Ordering, Arc, Mutex};

use wayland_client::{protocol::wl_surface, ConnectionHandle, Dispatch, Proxy, QueueHandle};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_toplevel_decoration_v1,
    xdg_shell::client::{xdg_surface, xdg_toplevel},
};

use crate::{compositor::SurfaceData, shell::xdg_inner::XdgSurfaceInner};

use super::{
    inner::XdgToplevelInner, DecorationMode, WindowData, WindowError, XdgShellState, XdgSurfaceData,
};

#[derive(Debug, Clone)]
pub(crate) struct RustWindow {
    pub wl_surface: wl_surface::WlSurface,
    pub xdg_surface: xdg_surface::XdgSurface,
    pub toplevel: Arc<Mutex<XdgToplevelInner>>,
    pub xdg_toplevel: xdg_toplevel::XdgToplevel,
}

impl RustWindow {
    pub fn new<D>(
        shell: &mut XdgShellState,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        surface: wl_surface::WlSurface,
        decoration_mode: DecorationMode,
    ) -> Result<RustWindow, WindowError>
    where
        D: Dispatch<wl_surface::WlSurface, UserData = SurfaceData>
            + Dispatch<xdg_surface::XdgSurface, UserData = XdgSurfaceData>
            + Dispatch<xdg_toplevel::XdgToplevel, UserData = WindowData>
            + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        let wl_surface = surface.clone();
        let surface_data = surface.data::<SurfaceData>().unwrap();
        let has_role = surface_data.has_role.load(Ordering::SeqCst);

        if !has_role {
            let (_, wm_base) = shell.wm_base.as_ref().ok_or(WindowError::MissingXdgShellGlobal)?;
            let decoration_manager =
                shell.zxdg_decoration_manager.clone().map(|(_, manager)| manager);

            let inner = Arc::new(Mutex::new(XdgSurfaceInner::Uninit));
            let xdg_surface_data = XdgSurfaceData { inner };
            let xdg_surface = wm_base.get_xdg_surface(cx, surface.clone(), qh, xdg_surface_data)?;

            let inner = Arc::new(Mutex::new(XdgToplevelInner {
                decoration_manager: decoration_manager.clone(),
                decoration: None,

                title: None,
                app_id: None,
                decoration_mode,
                min_size: (0, 0),
                max_size: (0, 0),
                pending_configure: None,
            }));

            {
                // Ugly but we need to give the xdg surface the toplevel's data.
                let mut data = xdg_surface.data::<XdgSurfaceData>().unwrap().inner.lock().unwrap();

                *data = XdgSurfaceInner::Window(inner.clone());
            }

            let window_data = WindowData { inner };
            let xdg_toplevel = xdg_surface.get_toplevel(cx, qh, window_data.clone())?;

            match decoration_mode {
                // Do not create the decoration manager.
                DecorationMode::None | DecorationMode::ClientSide => (),

                _ => {
                    let decoration = decoration_manager
                        .as_ref()
                        .map(|manager| {
                            manager.get_toplevel_decoration(
                                cx,
                                xdg_toplevel.clone(),
                                qh,
                                window_data.clone(),
                            )
                        })
                        .transpose()?;

                    if let Some(decoration) = decoration.as_ref() {
                        // Explicitly ask the server for server side decorations
                        if decoration_mode == DecorationMode::ServerSide {
                            decoration.set_mode(cx, zxdg_toplevel_decoration_v1::Mode::ServerSide);
                        }

                        window_data.inner.lock().unwrap().decoration = Some(decoration.clone());
                    }
                }
            }

            let window_inner =
                RustWindow { wl_surface, xdg_surface, toplevel: window_data.inner, xdg_toplevel };

            // Mark the surface as having a role.
            surface_data.has_role.store(true, Ordering::SeqCst);

            Ok(window_inner)
        } else {
            Err(WindowError::HasRole)
        }
    }
}

impl PartialEq for RustWindow {
    fn eq(&self, other: &Self) -> bool {
        self.xdg_surface == other.xdg_surface && self.xdg_toplevel == other.xdg_toplevel
    }
}
