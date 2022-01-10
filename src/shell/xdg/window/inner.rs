use std::{
    convert::TryFrom,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
};

use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_output, wl_surface},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy, QueueHandle,
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

use crate::shell::xdg::XdgShellState;

use super::{DecorationMode, Window, WindowConfigure, WindowData, WindowHandler};

#[derive(Debug)]
pub struct WindowInner {
    pub(crate) wl_surface: wl_surface::WlSurface,
    pub(crate) xdg_surface: xdg_surface::XdgSurface,
    // Lateinit: This is actually always some.
    pub(crate) xdg_toplevel: Mutex<Option<xdg_toplevel::XdgToplevel>>,
    pub(crate) zxdg_decoration_manager:
        Mutex<Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>>,
    pub(crate) zxdg_toplevel_decoration:
        Mutex<Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>>,

    pub(crate) pending_configure: Mutex<Option<WindowConfigure>>,
    pub(crate) prefered_decoration_mode: Mutex<Option<DecorationMode>>,
    pub(crate) current_decoration_mode: Mutex<Mode>,
    pub(crate) first_configure: AtomicBool,
}

impl WindowInner {
    pub fn new<D>(
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        wl_surface: &wl_surface::WlSurface,
        xdg_surface: &xdg_surface::XdgSurface,
        zxdg_decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    ) -> Result<Arc<WindowInner>, InvalidId>
    where
        D: Dispatch<xdg_toplevel::XdgToplevel, UserData = WindowData> + 'static,
    {
        let inner = WindowInner {
            wl_surface: wl_surface.clone(),
            xdg_surface: xdg_surface.clone(),
            xdg_toplevel: Mutex::new(None),
            zxdg_decoration_manager: Mutex::new(zxdg_decoration_manager),
            zxdg_toplevel_decoration: Mutex::new(None),

            pending_configure: Mutex::new(None),
            prefered_decoration_mode: Mutex::new(None),
            current_decoration_mode: Mutex::new(Mode::ClientSide),
            first_configure: AtomicBool::new(true),
        };

        let inner = Arc::new(inner);
        let window_data = WindowData(inner.clone());

        let xdg_toplevel = xdg_surface.get_toplevel(conn, qh, window_data)?;
        *inner.xdg_toplevel.lock().unwrap() = Some(xdg_toplevel);

        Ok(inner)
    }

    pub fn xdg_toplevel(&self) -> xdg_toplevel::XdgToplevel {
        self.xdg_toplevel.lock().unwrap().clone().unwrap()
    }

    pub fn map<D>(&self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        self.maybe_create_decoration(conn, qh);
        self.wl_surface.commit(conn);
    }

    #[must_use]
    pub fn configure(&self) -> Option<WindowConfigure> {
        self.pending_configure.lock().unwrap().take()
    }

    pub fn set_title(&self, conn: &mut ConnectionHandle, title: String) {
        self.xdg_toplevel().set_title(conn, title);
        // TODO: Store name for client side frame.
    }

    pub fn set_app_id(&self, conn: &mut ConnectionHandle, app_id: String) {
        self.xdg_toplevel().set_app_id(conn, app_id);
    }

    pub fn set_min_size(&self, conn: &mut ConnectionHandle, min_size: Option<(u32, u32)>) {
        let min_size = min_size.unwrap_or((0, 0));
        self.xdg_toplevel().set_min_size(conn, min_size.0 as i32, min_size.1 as i32)
    }

    pub fn set_max_size(&self, conn: &mut ConnectionHandle, max_size: Option<(u32, u32)>) {
        let max_size = max_size.unwrap_or((0, 0));
        self.xdg_toplevel().set_max_size(conn, max_size.0 as i32, max_size.1 as i32)
    }

    pub fn set_parent(&self, conn: &mut ConnectionHandle, parent: Option<&Window>) {
        self.xdg_toplevel()
            .set_parent(conn, parent.map(Window::xdg_toplevel).as_ref().map(AsRef::as_ref))
    }

    pub fn set_maximized(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel().set_maximized(conn)
    }

    pub fn unset_maximized(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel().unset_maximized(conn)
    }

    pub fn set_minmized(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel().set_minimized(conn)
    }

    pub fn set_fullscreen(
        &self,
        conn: &mut ConnectionHandle,
        output: Option<&wl_output::WlOutput>,
    ) {
        self.xdg_toplevel().set_fullscreen(conn, output)
    }

    pub fn unset_fullscreen(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel().unset_fullscreen(conn)
    }

    fn maybe_create_decoration<D>(&self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
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
                    let data = self.xdg_toplevel().data::<WindowData>().unwrap().clone();

                    let zxdg_toplevel_decoration = decoration_manager
                        .get_toplevel_decoration(conn, &self.xdg_toplevel(), qh, data)
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
        D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        for window in self.windows.iter().filter_map(Weak::upgrade) {
            // Only create decorations if the window is live.
            if !window.first_configure.load(Ordering::SeqCst) {
                window.maybe_create_decoration(conn, qh);
            }
        }
    }
}

impl DelegateDispatchBase<xdg_toplevel::XdgToplevel> for XdgShellState {
    type UserData = WindowData;
}

impl<D> DelegateDispatch<xdg_toplevel::XdgToplevel, D> for XdgShellState
where
    D: Dispatch<xdg_toplevel::XdgToplevel, UserData = Self::UserData> + WindowHandler,
{
    fn event(
        state: &mut D,
        _toplevel: &xdg_toplevel::XdgToplevel,
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

                let pending_configure = &mut *data.0.pending_configure.lock().unwrap();

                match pending_configure {
                    Some(pending_configure) => {
                        pending_configure.new_size = new_size;
                        pending_configure.states = states;
                    }

                    None => {
                        *pending_configure = Some(WindowConfigure { new_size, states });
                    }
                }
            }

            xdg_toplevel::Event::Close => {
                let window = Window(data.0.clone());
                state.request_close_window(conn, qh, &window);
            }

            _ => unreachable!(),
        }
    }
}

// XDG decoration

impl DelegateDispatchBase<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1> for XdgShellState {
    type UserData = ();
}

impl<D> DelegateDispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, D> for XdgShellState
where
    D: Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, UserData = Self::UserData>
        + WindowHandler,
{
    fn event(
        _: &mut D,
        _: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _: zxdg_decoration_manager_v1::Event,
        _: &(),
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zxdg_decoration_manager_v1 has no events")
    }
}

impl DelegateDispatchBase<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1> for XdgShellState {
    type UserData = WindowData;
}

impl<D> DelegateDispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, D> for XdgShellState
where
    D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = Self::UserData>
        + WindowHandler,
{
    fn event(
        _: &mut D,
        _: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        data: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        match event {
            zxdg_toplevel_decoration_v1::Event::Configure { mode } => match mode {
                wayland_client::WEnum::Value(mode) => {
                    *data.0.current_decoration_mode.lock().unwrap() = mode;
                }

                wayland_client::WEnum::Unknown(_) => unreachable!(),
            },

            _ => unreachable!(),
        }
    }
}
