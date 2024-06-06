use std::sync::{Arc, Weak};

use crate::{compositor::Surface, shell::WaylandSurface};
use wayland_client::{
    protocol::{
        wl_output::WlOutput,
        wl_seat::WlSeat,
        wl_shell_surface::{Resize, WlShellSurface},
        wl_surface::WlSurface,
    },
    Connection, QueueHandle,
};

pub trait WindowHandler: Sized {
    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        wl_surface: &WlSurface,
        configure: (Resize, u32, u32),
    );

    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, wl_surface: &WlSurface);
}

#[derive(Debug)]
pub struct WlShellWindowInner {
    pub surface: Surface,
    pub wl_shell_surface: WlShellSurface,
}

#[derive(Clone, Debug)]
pub struct Window(Arc<WlShellWindowInner>);

impl Window {
    pub fn new(surface: impl Into<Surface>, wl_shell_surface: WlShellSurface) -> Self {
        Self(Arc::new_cyclic(|_weak| WlShellWindowInner {
            surface: surface.into(),
            wl_shell_surface,
        }))
    }

    pub fn wl_shell_surface(&self) -> &WlShellSurface {
        &self.0.wl_shell_surface
    }

    pub fn set_maximized(&self) {
        self.0.wl_shell_surface.set_maximized(None)
    }
    pub fn set_top_level(&self) {
        self.0.wl_shell_surface.set_toplevel()
    }

    pub fn set_fullscreen(&self, output: Option<&WlOutput>) {
        self.0.wl_shell_surface.set_fullscreen(
            wayland_client::protocol::wl_shell_surface::FullscreenMethod::Fill,
            60000,
            output,
        );
    }

    pub fn resize(&self, seat: &WlSeat, serial: u32, edges: Resize) {
        self.0.wl_shell_surface.resize(seat, serial, edges)
    }

    pub fn move_(&self, seat: &WlSeat, serial: u32) {
        self.0.wl_shell_surface._move(seat, serial)
    }

    pub fn set_title(&self, title: impl Into<String>) {
        self.0.wl_shell_surface.set_title(title.into())
    }

    pub fn set_app_id(&self, app_id: impl Into<String>) {
        self.0.wl_shell_surface.set_class(app_id.into())
    }
}

impl WaylandSurface for Window {
    fn wl_surface(&self) -> &WlSurface {
        &self.0.surface.wl_surface()
    }
}

#[derive(Debug, Clone)]
pub struct WindowData(pub(crate) Weak<WlShellWindowInner>);
