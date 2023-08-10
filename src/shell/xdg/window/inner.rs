use std::{
    convert::{TryFrom, TryInto},
    num::NonZeroU32,
    sync::Mutex,
};

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::{
    xdg::decoration::zv1::client::{
        zxdg_decoration_manager_v1,
        zxdg_toplevel_decoration_v1::{self, Mode},
    },
    xdg::shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State, WmCapabilities},
    },
};

use crate::{
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
    shell::xdg::{XdgShell, XdgShellSurface},
};

use super::{
    DecorationMode, Window, WindowConfigure, WindowData, WindowHandler, WindowManagerCapabilities,
    WindowState,
};

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
    pub xdg_surface: XdgShellSurface,
    pub xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub toplevel_decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    pub pending_configure: Mutex<WindowConfigure>,
}

impl ProvidesBoundGlobal<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, 1> for XdgShell {
    fn bound_global(
        &self,
    ) -> Result<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, GlobalError> {
        self.xdg_decoration_manager.get().cloned()
    }
}

impl<D> Dispatch<xdg_surface::XdgSurface, WindowData, D> for XdgShell
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

impl<D> Dispatch<xdg_toplevel::XdgToplevel, WindowData, D> for XdgShell
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
                    let new_state = states
                        .chunks_exact(4)
                        .flat_map(TryInto::<[u8; 4]>::try_into)
                        .map(u32::from_ne_bytes)
                        .flat_map(State::try_from)
                        .fold(WindowState::empty(), |mut acc, state| {
                            match state {
                                State::Maximized => acc.set(WindowState::MAXIMIZED, true),
                                State::Fullscreen => acc.set(WindowState::FULLSCREEN, true),
                                State::Resizing => acc.set(WindowState::RESIZING, true),
                                State::Activated => acc.set(WindowState::ACTIVATED, true),
                                State::TiledLeft => acc.set(WindowState::TILED_LEFT, true),
                                State::TiledRight => acc.set(WindowState::TILED_RIGHT, true),
                                State::TiledTop => acc.set(WindowState::TILED_TOP, true),
                                State::TiledBottom => acc.set(WindowState::TILED_BOTTOM, true),
                                State::Suspended => acc.set(WindowState::SUSPENDED, true),
                                _ => (),
                            }
                            acc
                        });

                    // XXX we do explicit convertion and sanity checking because compositor
                    // could pass negative values which we should ignore all together.
                    let width = u32::try_from(width).ok().and_then(NonZeroU32::new);
                    let height = u32::try_from(height).ok().and_then(NonZeroU32::new);

                    let pending_configure = &mut window.0.pending_configure.lock().unwrap();
                    pending_configure.new_size = (width, height);
                    pending_configure.state = new_state;
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
                xdg_toplevel::Event::WmCapabilities { capabilities } => {
                    let pending_configure = &mut window.0.pending_configure.lock().unwrap();
                    pending_configure.capabilities = capabilities
                        .chunks_exact(4)
                        .flat_map(TryInto::<[u8; 4]>::try_into)
                        .map(u32::from_ne_bytes)
                        .flat_map(WmCapabilities::try_from)
                        .fold(WindowManagerCapabilities::empty(), |mut acc, capability| {
                            match capability {
                                WmCapabilities::WindowMenu => {
                                    acc.set(WindowManagerCapabilities::WINDOW_MENU, true)
                                }
                                WmCapabilities::Maximize => {
                                    acc.set(WindowManagerCapabilities::MAXIMIZE, true)
                                }
                                WmCapabilities::Fullscreen => {
                                    acc.set(WindowManagerCapabilities::FULLSCREEN, true)
                                }
                                WmCapabilities::Minimize => {
                                    acc.set(WindowManagerCapabilities::MINIMIZE, true)
                                }
                                _ => (),
                            }
                            acc
                        });
                }
                _ => unreachable!(),
            }
        }
    }
}

// XDG decoration

impl<D> Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, GlobalData, D> for XdgShell
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

impl<D> Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, WindowData, D> for XdgShell
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
