use std::sync::{atomic::Ordering, Arc, Mutex};

use wayland_client::{protocol::wl_surface, ConnectionHandle, Dispatch, Proxy, QueueHandle};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
    },
    xdg_shell::client::{
        xdg_popup, xdg_surface,
        xdg_toplevel::{self, State},
    },
};

use crate::compositor::SurfaceData;

use super::{DecorationMode, WindowData, WindowError, XdgShellState, XdgSurfaceData};

#[derive(Debug, Clone)]
pub(crate) struct WindowInner {
    pub wl_surface: wl_surface::WlSurface,
    pub xdg_surface: xdg_surface::XdgSurface,
    pub toplevel: Arc<Mutex<XdgToplevelInner>>,
    pub xdg_toplevel: xdg_toplevel::XdgToplevel,
}

impl WindowInner {
    pub fn new<D>(
        shell: &mut XdgShellState,
        cx: &mut ConnectionHandle,
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
        let wl_surface = surface.clone();
        let surface_data = surface.data::<SurfaceData>().unwrap();
        let has_role = surface_data.has_role.load(Ordering::SeqCst);

        if !has_role {
            let (_, wm_base) = shell.wm_base.as_ref().ok_or(WindowError::MissingXdgShellGlobal)?;
            let decoration_manager =
                shell.zxdg_decoration_manager.clone().map(|(_, manager)| manager);

            let inner = Arc::new(Mutex::new(XdgSurfaceInner::Uninit));
            let xdg_surface_data = XdgSurfaceData { inner };
            let xdg_surface =
                wm_base.get_xdg_surface(cx, surface.clone(), qh, xdg_surface_data.clone())?;

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

                    if let Some(decoration) = decoration.clone() {
                        // Explicitly ask the server for server side decorations
                        if decoration_mode == DecorationMode::ServerSide {
                            decoration.set_mode(cx, zxdg_toplevel_decoration_v1::Mode::ServerSide);
                        }

                        window_data.inner.lock().unwrap().decoration = Some(decoration);
                    }
                }
            }

            // Perform an initial commit without any buffer attached per the xdg_surface requirements.
            wl_surface.commit(cx);

            let window_inner =
                WindowInner { wl_surface, xdg_surface, toplevel: window_data.inner, xdg_toplevel };

            // Mark the surface as having a role.
            surface_data.has_role.store(true, Ordering::SeqCst);
            drop(has_role);

            let inner = Arc::new(window_inner);
            shell.windows.push(Arc::downgrade(&inner));

            Ok(inner)
        } else {
            Err(WindowError::HasRole)
        }
    }
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
