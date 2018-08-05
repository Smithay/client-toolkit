use wayland_client::commons::Implementation;
use wayland_client::protocol::{wl_output, wl_seat, wl_surface};
use wayland_client::Proxy;

use wayland_protocols::xdg_shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

use wayland_client::protocol::wl_surface::RequestsTrait as WlSurfaceRequests;
use wayland_protocols::xdg_shell::client::xdg_surface::RequestsTrait as SurfaceRequests;
use wayland_protocols::xdg_shell::client::xdg_toplevel::RequestsTrait as ToplevelRequests;
use wayland_protocols::xdg_shell::client::xdg_wm_base::RequestsTrait as ShellRequests;

use super::{Event, ShellSurface};

pub(crate) struct Xdg {
    surface: Proxy<xdg_surface::XdgSurface>,
    toplevel: Proxy<xdg_toplevel::XdgToplevel>,
}

impl Xdg {
    pub(crate) fn create<Impl>(
        surface: &Proxy<wl_surface::WlSurface>,
        shell: &Proxy<xdg_wm_base::XdgWmBase>,
        mut implementation: Impl,
    ) -> Xdg
    where
        Impl: Implementation<(), Event> + Send,
    {
        let xdgs = shell.get_xdg_surface(surface).unwrap().implement(
            |evt, xdgs: Proxy<_>| match evt {
                xdg_surface::Event::Configure { serial } => {
                    xdgs.ack_configure(serial);
                }
            },
        );
        let toplevel = xdgs.get_toplevel().unwrap().implement(move |evt, _| {
            match evt {
                xdg_toplevel::Event::Close => implementation.receive(Event::Close, ()),
                xdg_toplevel::Event::Configure {
                    width,
                    height,
                    states,
                } => {
                    use std::cmp::max;
                    let new_size = if width == 0 || height == 0 {
                        // if either w or h is zero, then we get to choose our size
                        None
                    } else {
                        Some((max(width, 1) as u32, max(height, 1) as u32))
                    };
                    let view: &[u32] = unsafe {
                        ::std::slice::from_raw_parts(states.as_ptr() as *const _, states.len() / 4)
                    };
                    let states = view
                        .iter()
                        .cloned()
                        .flat_map(xdg_toplevel::State::from_raw)
                        .collect::<Vec<_>>();
                    implementation.receive(Event::Configure { new_size, states }, ());
                }
            }
        });
        surface.commit();
        Xdg {
            surface: xdgs,
            toplevel,
        }
    }
}

impl ShellSurface for Xdg {
    fn resize(&self, seat: &Proxy<wl_seat::WlSeat>, serial: u32, edges: xdg_toplevel::ResizeEdge) {
        self.toplevel.resize(seat, serial, edges as u32);
    }

    fn move_(&self, seat: &Proxy<wl_seat::WlSeat>, serial: u32) {
        self.toplevel._move(seat, serial);
    }

    fn set_title(&self, title: String) {
        self.toplevel.set_title(title);
    }

    fn set_app_id(&self, app_id: String) {
        self.toplevel.set_app_id(app_id);
    }

    fn set_fullscreen(&self, output: Option<&Proxy<wl_output::WlOutput>>) {
        self.toplevel.set_fullscreen(output)
    }

    fn unset_fullscreen(&self) {
        self.toplevel.unset_fullscreen();
    }

    fn set_maximized(&self) {
        self.toplevel.set_maximized();
    }

    fn unset_maximized(&self) {
        self.toplevel.unset_maximized();
    }

    fn set_minimized(&self) {
        self.toplevel.set_minimized();
    }

    fn set_geometry(&self, x: i32, y: i32, width: i32, height: i32) {
        self.surface.set_window_geometry(x, y, width, height);
    }

    fn set_min_size(&self, size: Option<(i32, i32)>) {
        if let Some((w, h)) = size {
            self.toplevel.set_min_size(w, h);
        } else {
            self.toplevel.set_min_size(0, 0);
        }
    }

    fn set_max_size(&self, size: Option<(i32, i32)>) {
        if let Some((w, h)) = size {
            self.toplevel.set_max_size(w, h);
        } else {
            self.toplevel.set_max_size(0, 0);
        }
    }

    fn get_xdg(&self) -> Option<&Proxy<xdg_toplevel::XdgToplevel>> {
        Some(&self.toplevel)
    }
}

impl Drop for Xdg {
    fn drop(&mut self) {
        self.toplevel.destroy();
        self.surface.destroy();
    }
}
