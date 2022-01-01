use std::{
    convert::TryFrom,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
};

use wayland_client::{
    protocol::wl_surface, ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch,
    Proxy, QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1,
        zxdg_toplevel_decoration_v1::{self, Mode},
    },
    xdg_shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
    },
};

use crate::shell::xdg::{
    inner::XdgSurfaceDataInner, XdgShellDispatch, XdgShellHandler, XdgShellState, XdgSurfaceData,
};

use super::DecorationMode;

#[derive(Debug)]
pub struct WindowInner {
    pub(crate) wl_surface: wl_surface::WlSurface,
    pub(crate) xdg_surface: xdg_surface::XdgSurface,
    pub(crate) xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub(crate) zxdg_decoration_manager:
        Mutex<Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>>,
    pub(crate) zxdg_toplevel_decoration:
        Mutex<Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>>,

    pub(crate) pending_configure: Mutex<PendingConfigure>,
    pub(crate) prefered_decoration_mode: Mutex<Option<DecorationMode>>,
    pub(crate) current_decoration_mode: Mutex<Mode>,
    pub(crate) first_configure: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct PendingConfigure {
    pub(crate) new_size: Option<(u32, u32)>,
    pub(crate) states: Vec<State>,
}

impl WindowInner {
    pub fn new(
        wl_surface: wl_surface::WlSurface,
        xdg_surface: xdg_surface::XdgSurface,
        xdg_toplevel: xdg_toplevel::XdgToplevel,
        zxdg_decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    ) -> Arc<WindowInner> {
        let inner = WindowInner {
            wl_surface,
            xdg_surface,
            xdg_toplevel,
            zxdg_decoration_manager: Mutex::new(zxdg_decoration_manager),
            zxdg_toplevel_decoration: Mutex::new(None),

            pending_configure: Mutex::new(PendingConfigure { new_size: None, states: vec![] }),
            prefered_decoration_mode: Mutex::new(None),
            current_decoration_mode: Mutex::new(Mode::ClientSide),
            first_configure: AtomicBool::new(true),
        };

        Arc::new(inner)
    }

    pub fn map<D>(&self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<
                zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
                UserData = XdgSurfaceData,
            > + 'static,
    {
        self.maybe_create_decoration(conn, qh);
        self.wl_surface.commit(conn);
    }

    pub fn set_title(&self, conn: &mut ConnectionHandle, title: String) {
        self.xdg_toplevel.set_title(conn, title);
        // TODO: Store name for client side frame.
    }

    pub fn set_app_id(&self, conn: &mut ConnectionHandle, app_id: String) {
        self.xdg_toplevel.set_app_id(conn, app_id);
    }

    pub fn set_min_size(&self, conn: &mut ConnectionHandle, min_size: Option<(u32, u32)>) {
        let min_size = min_size.unwrap_or((0, 0));
        self.xdg_toplevel.set_min_size(conn, min_size.0 as i32, min_size.1 as i32)
    }

    pub fn set_max_size(&self, conn: &mut ConnectionHandle, max_size: Option<(u32, u32)>) {
        let max_size = max_size.unwrap_or((0, 0));
        self.xdg_toplevel.set_max_size(conn, max_size.0 as i32, max_size.1 as i32)
    }

    fn maybe_create_decoration<D>(&self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<
                zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
                UserData = XdgSurfaceData,
            > + 'static,
    {
        let decoration_manager = self.zxdg_decoration_manager.lock().unwrap();

        if let Some(ref decoration_manager) = *decoration_manager {
            let guard = self.prefered_decoration_mode.lock().unwrap();
            // By default we assume the server should be preferred.
            let preferred = guard.unwrap_or(DecorationMode::PreferServer);
            drop(guard);

            match preferred {
                // Do not create the toplevel decoration.
                DecorationMode::ClientOnly | DecorationMode::None => (),

                _ => {
                    // Create the toplevel decoration.
                    let data = self.xdg_toplevel.data::<XdgSurfaceData>().unwrap().clone();

                    let zxdg_toplevel_decoration = decoration_manager
                        .get_toplevel_decoration(conn, &self.xdg_toplevel, qh, data)
                        .expect("failed to create toplevel decoration");

                    // Specifically request server side if requested.
                    if preferred == DecorationMode::ServerOnly {
                        zxdg_toplevel_decoration.set_mode(conn, Mode::ServerSide);
                    }

                    *self.zxdg_toplevel_decoration.lock().unwrap() = Some(zxdg_toplevel_decoration);
                }
            }
        }
    }
}

impl XdgShellState {
    pub(crate) fn init_decorations<D>(&mut self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<
                zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
                UserData = XdgSurfaceData,
            > + 'static,
    {
        for window in self.windows.iter().filter_map(Weak::upgrade) {
            // Only create decorations if the window is live.
            if !window.first_configure.load(Ordering::SeqCst) {
                window.maybe_create_decoration(conn, qh);
            }
        }
    }
}

impl<D, H> DelegateDispatchBase<xdg_toplevel::XdgToplevel> for XdgShellDispatch<'_, D, H>
where
    H: XdgShellHandler<D>,
{
    type UserData = XdgSurfaceData;
}

impl<D, H> DelegateDispatch<xdg_toplevel::XdgToplevel, D> for XdgShellDispatch<'_, D, H>
where
    D: Dispatch<xdg_toplevel::XdgToplevel, UserData = Self::UserData>,
    H: XdgShellHandler<D>,
{
    fn event(
        &mut self,
        toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        data: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, states } => {
                let states = states
                    .iter()
                    .cloned()
                    .map(|entry| entry as u32)
                    .map(State::try_from)
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>();

                let new_size = if width == 0 && height == 0 {
                    None
                } else {
                    Some((width as u32, height as u32))
                };

                if let XdgSurfaceDataInner::Window(window) = &*data.0.lock().unwrap() {
                    let mut pending_configure = window.pending_configure.lock().unwrap();
                    pending_configure.new_size = new_size;
                    pending_configure.states = states;
                }
            }

            xdg_toplevel::Event::Close => {
                if let Some(window) = self.0.window_by_toplevel(toplevel) {
                    self.1.request_close_window(conn, qh, self.0, &window);
                }
            }

            _ => unreachable!(),
        }
    }
}

// XDG decoration

impl<D, H> DelegateDispatchBase<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>
    for XdgShellDispatch<'_, D, H>
where
    H: XdgShellHandler<D>,
{
    type UserData = ();
}

impl<D, H> DelegateDispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, D>
    for XdgShellDispatch<'_, D, H>
where
    D: Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, UserData = Self::UserData>,
    H: XdgShellHandler<D>,
{
    fn event(
        &mut self,
        _: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _: zxdg_decoration_manager_v1::Event,
        _: &(),
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zxdg_decoration_manager_v1 has no events")
    }
}

impl<D, H> DelegateDispatchBase<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>
    for XdgShellDispatch<'_, D, H>
where
    H: XdgShellHandler<D>,
{
    type UserData = XdgSurfaceData;
}

impl<D, H> DelegateDispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, D>
    for XdgShellDispatch<'_, D, H>
where
    D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = Self::UserData>,
    H: XdgShellHandler<D>,
{
    fn event(
        &mut self,
        _: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        data: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        match event {
            zxdg_toplevel_decoration_v1::Event::Configure { mode } => {
                match mode {
                    wayland_client::WEnum::Value(mode) => {
                        let data = data.0.lock().unwrap();

                        if let XdgSurfaceDataInner::Window(window) = &*data {
                            *window.current_decoration_mode.lock().unwrap() = mode;
                            // TODO: Modify configure state?
                        } else {
                            unreachable!()
                        }
                    }

                    wayland_client::WEnum::Unknown(_) => unreachable!(),
                }
            }

            _ => unreachable!(),
        }
    }
}
