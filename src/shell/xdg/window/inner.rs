use std::{
    convert::{TryFrom, TryInto},
    sync::{atomic::Ordering, Mutex},
};

use wayland_client::{
    protocol::{wl_output, wl_seat},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1,
        zxdg_toplevel_decoration_v1::{self, Mode},
    },
    xdg_shell::client::{
        xdg_toplevel::{self, State},
        xdg_wm_base,
    },
};

use crate::{
    registry::{ProvidesRegistryState, RegistryHandler},
    shell::xdg::XdgShellSurface,
};

use super::{DecorationMode, Window, WindowConfigure, WindowData, WindowHandler, XdgWindowState};

impl Window {
    /// Clone is an implementation detail of Window.
    ///
    /// This function creates another window handle that is not marked as a primary handle.
    pub(crate) fn impl_clone(&self) -> Window {
        Window {
            inner: self.inner.clone(),
            primary: false,
            death_signal: self.death_signal.clone(),
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // If we are the primary handle (an owned value given to the user), mark ourselves for cleanup.
        if self.primary {
            self.death_signal.store(true, Ordering::SeqCst);
        }
    }
}

#[derive(Debug)]
pub struct WindowInner {
    pub(crate) xdg_surface: XdgShellSurface,
    pub(crate) xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub(crate) toplevel_decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
}

impl WindowInner {
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

    pub fn set_parent(&self, conn: &mut ConnectionHandle, parent: Option<&Window>) {
        self.xdg_toplevel.set_parent(conn, parent.map(Window::xdg_toplevel))
    }

    pub fn show_window_menu(
        &self,
        conn: &mut ConnectionHandle,
        seat: &wl_seat::WlSeat,
        serial: u32,
        x: u32,
        y: u32,
    ) {
        self.xdg_toplevel.show_window_menu(conn, seat, serial, x as i32, y as i32)
    }

    pub fn set_maximized(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel.set_maximized(conn)
    }

    pub fn unset_maximized(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel.unset_maximized(conn)
    }

    pub fn set_minmized(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel.set_minimized(conn)
    }

    pub fn set_fullscreen(
        &self,
        conn: &mut ConnectionHandle,
        output: Option<&wl_output::WlOutput>,
    ) {
        self.xdg_toplevel.set_fullscreen(conn, output)
    }

    pub fn unset_fullscreen(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel.unset_fullscreen(conn)
    }

    pub fn request_decoration_mode(
        &self,
        conn: &mut ConnectionHandle,
        mode: Option<DecorationMode>,
    ) {
        if let Some(toplevel_decoration) = &self.toplevel_decoration {
            match mode {
                Some(DecorationMode::Client) => {
                    toplevel_decoration.set_mode(conn, Mode::ClientSide)
                }
                Some(DecorationMode::Server) => {
                    toplevel_decoration.set_mode(conn, Mode::ServerSide)
                }
                None => toplevel_decoration.unset_mode(conn),
            }
        }
    }
}

#[derive(Debug)]
pub struct WindowDataInner {
    pub(crate) pending_configure: Mutex<WindowConfigure>,
}

impl XdgWindowState {
    pub(crate) fn cleanup(&mut self, conn: &mut ConnectionHandle) {
        self.windows.retain(|window| {
            let alive = !window.death_signal.load(Ordering::SeqCst);

            if !alive {
                // XDG decoration says we must destroy the decoration object before the toplevel
                if let Some(toplevel_decoration) = window.inner.toplevel_decoration.as_ref() {
                    toplevel_decoration.destroy(conn);
                }

                // XDG Shell protocol dictates we must destroy the role object before the xdg surface.
                window.xdg_toplevel().destroy(conn);
                window.xdg_surface().destroy(conn);
                window.wl_surface().destroy(conn);
            }

            alive
        })
    }
}

const DECORATION_MANAGER_VERSION: u32 = 1;

impl<D> RegistryHandler<D> for XdgWindowState
where
    D: Dispatch<xdg_wm_base::XdgWmBase, UserData = ()>
        + Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, UserData = ()>
        // Lateinit for decorations
        + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
        + WindowHandler
        + ProvidesRegistryState
        + 'static,
{
    fn new_global(
        data: &mut D,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        _version: u32,
    ) {
        if "zxdg_decoration_manager_v1" == interface {
            if data.xdg_window_state().xdg_decoration_manager.is_some() {
                return;
            }

            let xdg_decoration_manager = data
                .registry()
                .bind_once::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _>(
                    conn,
                    qh,
                    name,
                    DECORATION_MANAGER_VERSION,
                    (),
                )
                .expect("failed to bind global");

            data.xdg_window_state().xdg_decoration_manager = Some((name, xdg_decoration_manager));
        }
    }

    fn remove_global(_: &mut D, _: &mut ConnectionHandle, _: &QueueHandle<D>, _: u32) {
        // Unlikely to ever occur.
    }
}

impl DelegateDispatchBase<xdg_toplevel::XdgToplevel> for XdgWindowState {
    type UserData = WindowData;
}

impl<D> DelegateDispatch<xdg_toplevel::XdgToplevel, D> for XdgWindowState
where
    D: Dispatch<xdg_toplevel::XdgToplevel, UserData = Self::UserData> + WindowHandler,
{
    fn event(
        data: &mut D,
        toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        udata: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, states } => {
                // The states are encoded as a bunch of u32 of native endian, but are encoded in an array of
                // bytes.
                let states = states
                    .chunks_exact(4)
                    .flat_map(TryInto::<[u8; 4]>::try_into)
                    .map(u32::from_ne_bytes)
                    .flat_map(State::try_from)
                    .collect::<Vec<_>>();

                let new_size = if width == 0 && height == 0 {
                    None
                } else {
                    Some((width as u32, height as u32))
                };

                let pending_configure = &mut *udata.0.pending_configure.lock().unwrap();
                pending_configure.new_size = new_size;
                pending_configure.states = states;
            }

            xdg_toplevel::Event::Close => {
                if let Some(window) = data.xdg_window_state().window_by_toplevel(toplevel) {
                    let window = window.impl_clone();

                    data.request_close(conn, qh, &window);
                } else {
                    log::warn!(target: "sctk", "closed event received for dead window: {}", toplevel.id());
                }
            }

            _ => unreachable!(),
        }

        // Perform cleanup
        data.xdg_window_state().cleanup(conn);
    }
}

// XDG decoration

impl DelegateDispatchBase<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1> for XdgWindowState {
    type UserData = ();
}

impl<D> DelegateDispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, D> for XdgWindowState
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

impl DelegateDispatchBase<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>
    for XdgWindowState
{
    type UserData = WindowData;
}

impl<D> DelegateDispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, D>
    for XdgWindowState
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
                    let mode = match mode {
                        Mode::ClientSide => DecorationMode::Client,
                        Mode::ServerSide => DecorationMode::Server,

                        _ => unreachable!(),
                    };

                    data.0.pending_configure.lock().unwrap().decoration_mode = mode;
                }

                wayland_client::WEnum::Unknown(unknown) => {
                    log::error!(target: "sctk", "unknown decoration mode 0x{:x}", unknown);
                }
            },

            _ => unreachable!(),
        }
    }
}
