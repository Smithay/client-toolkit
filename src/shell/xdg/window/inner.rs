use std::{
    convert::{TryFrom, TryInto},
    sync::Mutex,
};

use wayland_client::{
    protocol::{wl_output, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::{
    xdg::decoration::zv1::client::{
        zxdg_decoration_manager_v1,
        zxdg_toplevel_decoration_v1::{self, Mode},
    },
    xdg::shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
    },
};

use crate::{
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
    shell::xdg::XdgShellSurface,
};

use super::{DecorationMode, Window, WindowConfigure, WindowData, WindowHandler, XdgWindowState};

impl Drop for WindowInner {
    fn drop(&mut self) {
        // XDG decoration says we must destroy the decoration object before the toplevel
        if let Some(toplevel_decoration) = self.toplevel_decoration.as_ref() {
            toplevel_decoration.destroy();
        }

        // XDG Shell protocol dictates we must destroy the role object before the xdg surface.
        self.xdg_toplevel.destroy();
        // XdgShellSurface will do it's own drop
        // self.xdg_surface.destroy();
    }
}

#[derive(Debug)]
pub struct WindowInner {
    pub(super) xdg_surface: XdgShellSurface,
    pub(super) xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub(super) toplevel_decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    pub(super) pending_configure: Mutex<WindowConfigure>,
}

impl WindowInner {
    pub fn set_title(&self, title: String) {
        self.xdg_toplevel.set_title(title);
        // TODO: Store name for client side frame.
    }

    pub fn set_app_id(&self, app_id: String) {
        self.xdg_toplevel.set_app_id(app_id);
    }

    pub fn set_min_size(&self, min_size: Option<(u32, u32)>) {
        let min_size = min_size.unwrap_or((0, 0));
        self.xdg_toplevel.set_min_size(min_size.0 as i32, min_size.1 as i32)
    }

    pub fn set_max_size(&self, max_size: Option<(u32, u32)>) {
        let max_size = max_size.unwrap_or((0, 0));
        self.xdg_toplevel.set_max_size(max_size.0 as i32, max_size.1 as i32)
    }

    pub fn set_parent(&self, parent: Option<&Window>) {
        self.xdg_toplevel.set_parent(parent.map(Window::xdg_toplevel))
    }

    pub fn show_window_menu(&self, seat: &wl_seat::WlSeat, serial: u32, x: u32, y: u32) {
        self.xdg_toplevel.show_window_menu(seat, serial, x as i32, y as i32)
    }

    pub fn set_maximized(&self) {
        self.xdg_toplevel.set_maximized()
    }

    pub fn unset_maximized(&self) {
        self.xdg_toplevel.unset_maximized()
    }

    pub fn set_minmized(&self) {
        self.xdg_toplevel.set_minimized()
    }

    pub fn set_fullscreen(&self, output: Option<&wl_output::WlOutput>) {
        self.xdg_toplevel.set_fullscreen(output)
    }

    pub fn unset_fullscreen(&self) {
        self.xdg_toplevel.unset_fullscreen()
    }

    pub fn request_decoration_mode(&self, mode: Option<DecorationMode>) {
        if let Some(toplevel_decoration) = &self.toplevel_decoration {
            match mode {
                Some(DecorationMode::Client) => toplevel_decoration.set_mode(Mode::ClientSide),
                Some(DecorationMode::Server) => toplevel_decoration.set_mode(Mode::ServerSide),
                None => toplevel_decoration.unset_mode(),
            }
        }
    }
}

impl ProvidesBoundGlobal<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, 1>
    for XdgWindowState
{
    fn bound_global(
        &self,
    ) -> Result<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, GlobalError> {
        self.xdg_decoration_manager.get().cloned()
    }
}

impl<D> Dispatch<xdg_surface::XdgSurface, WindowData, D> for XdgWindowState
where
    D: Dispatch<xdg_surface::XdgSurface, WindowData> + WindowHandler,
{
    fn event(
        data: &mut D,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &WindowData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        if let Some(window) = Window::from_xdg_surface(xdg_surface) {
            match event {
                xdg_surface::Event::Configure { serial } => {
                    // Acknowledge the configure per protocol requirements.
                    xdg_surface.ack_configure(serial);

                    let configure = { window.0.pending_configure.lock().unwrap().clone() };
                    WindowHandler::configure(data, conn, qh, &window, configure, serial);
                }

                _ => unreachable!(),
            }
        }
    }
}

impl<D> Dispatch<xdg_toplevel::XdgToplevel, WindowData, D> for XdgWindowState
where
    D: Dispatch<xdg_toplevel::XdgToplevel, WindowData> + WindowHandler,
{
    fn event(
        data: &mut D,
        toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &WindowData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        if let Some(window) = Window::from_xdg_toplevel(toplevel) {
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

                    let pending_configure = &mut window.0.pending_configure.lock().unwrap();
                    pending_configure.new_size = new_size;
                    pending_configure.states = states;
                }

                xdg_toplevel::Event::Close => {
                    data.request_close(conn, qh, &window);
                }

                xdg_toplevel::Event::ConfigureBounds { width, height } => {
                    let pending_configure = &mut window.0.pending_configure.lock().unwrap();
                    if width == 0 && height == 0 {
                        pending_configure.suggested_bounds = None;
                    } else {
                        pending_configure.suggested_bounds = Some((width as u32, height as u32));
                    }
                }

                _ => unreachable!(),
            }
        }
    }
}

// XDG decoration

impl<D> Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, GlobalData, D>
    for XdgWindowState
where
    D: Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, GlobalData> + WindowHandler,
{
    fn event(
        _: &mut D,
        _: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _: zxdg_decoration_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zxdg_decoration_manager_v1 has no events")
    }
}

impl<D> Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, WindowData, D>
    for XdgWindowState
where
    D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, WindowData> + WindowHandler,
{
    fn event(
        _: &mut D,
        decoration: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        _: &WindowData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        if let Some(window) = Window::from_toplevel_decoration(decoration) {
            match event {
                zxdg_toplevel_decoration_v1::Event::Configure { mode } => match mode {
                    wayland_client::WEnum::Value(mode) => {
                        let mode = match mode {
                            Mode::ClientSide => DecorationMode::Client,
                            Mode::ServerSide => DecorationMode::Server,

                            _ => unreachable!(),
                        };

                        window.0.pending_configure.lock().unwrap().decoration_mode = mode;
                    }

                    wayland_client::WEnum::Unknown(unknown) => {
                        log::error!(target: "sctk", "unknown decoration mode 0x{:x}", unknown);
                    }
                },

                _ => unreachable!(),
            }
        }
    }
}
