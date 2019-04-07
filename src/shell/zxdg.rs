use std::cell::RefCell;
use std::rc::Rc;

use wayland_client::protocol::{wl_output, wl_seat, wl_surface};

use wayland_protocols::unstable::xdg_shell::v6::client::{
    zxdg_shell_v6, zxdg_surface_v6, zxdg_toplevel_v6,
};
use wayland_protocols::xdg_shell::client::xdg_toplevel;

use super::{Event, ShellSurface};

pub(crate) struct Zxdg {
    surface: zxdg_surface_v6::ZxdgSurfaceV6,
    toplevel: zxdg_toplevel_v6::ZxdgToplevelV6,
}

impl Zxdg {
    pub(crate) fn create<Impl>(
        surface: &wl_surface::WlSurface,
        shell: &zxdg_shell_v6::ZxdgShellV6,
        implementation: Impl,
    ) -> Zxdg
    where
        Impl: FnMut(Event) + Send + 'static,
    {
        let pending_configure = Rc::new(RefCell::new(None));
        let pending_configure_2 = pending_configure.clone();

        let implementation = Rc::new(RefCell::new(implementation));
        let implementation_2 = implementation.clone();
        let xdgs = shell
            .get_xdg_surface(surface, move |xdgs| {
                xdgs.implement_closure(
                    move |evt, xdgs| match evt {
                        zxdg_surface_v6::Event::Configure { serial } => {
                            xdgs.ack_configure(serial);
                            if let Some((new_size, states)) =
                                pending_configure_2.borrow_mut().take()
                            {
                                (&mut *implementation_2.borrow_mut())(Event::Configure {
                                    new_size,
                                    states,
                                });
                            }
                        }
                        _ => unreachable!(),
                    },
                    (),
                )
            })
            .unwrap();
        let toplevel = xdgs
            .get_toplevel(|toplevel| {
                toplevel.implement_closure(
                    move |evt, _| {
                        match evt {
                            zxdg_toplevel_v6::Event::Close => {
                                (&mut *implementation.borrow_mut())(Event::Close)
                            }
                            zxdg_toplevel_v6::Event::Configure {
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
                                    ::std::slice::from_raw_parts(
                                        states.as_ptr() as *const _,
                                        states.len() / 4,
                                    )
                                };
                                let states = view
                                    .iter()
                                    .cloned()
                                    // bit representation of xdg_toplevel_v6 and zxdg_toplevel_v6 matches
                                    .flat_map(xdg_toplevel::State::from_raw)
                                    .collect::<Vec<_>>();
                                *pending_configure.borrow_mut() = Some((new_size, states));
                            }
                            _ => unreachable!(),
                        }
                    },
                    (),
                )
            })
            .unwrap();
        surface.commit();
        Zxdg {
            surface: xdgs,
            toplevel,
        }
    }
}

impl ShellSurface for Zxdg {
    fn resize(&self, seat: &wl_seat::WlSeat, serial: u32, edges: xdg_toplevel::ResizeEdge) {
        self.toplevel.resize(seat, serial, edges as u32);
    }

    fn move_(&self, seat: &wl_seat::WlSeat, serial: u32) {
        self.toplevel._move(seat, serial);
    }

    fn set_title(&self, title: String) {
        self.toplevel.set_title(title);
    }

    fn set_app_id(&self, app_id: String) {
        self.toplevel.set_app_id(app_id);
    }

    fn set_fullscreen(&self, output: Option<&wl_output::WlOutput>) {
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

    fn get_xdg(&self) -> Option<&xdg_toplevel::XdgToplevel> {
        None
    }
}

impl Drop for Zxdg {
    fn drop(&mut self) {
        self.toplevel.destroy();
        self.surface.destroy();
    }
}
