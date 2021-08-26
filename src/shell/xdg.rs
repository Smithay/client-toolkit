use std::{cell::RefCell, convert::TryInto, rc::Rc};

use wayland_client::{
    protocol::{wl_output, wl_seat, wl_surface},
    DispatchData,
};

use wayland_protocols::xdg_shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

use super::{Event, ShellSurface};

#[derive(Debug)]
pub(crate) struct Xdg {
    surface: xdg_surface::XdgSurface,
    toplevel: xdg_toplevel::XdgToplevel,
}

impl Xdg {
    pub(crate) fn create<Impl>(
        surface: &wl_surface::WlSurface,
        shell: &xdg_wm_base::XdgWmBase,
        implementation: Impl,
    ) -> Xdg
    where
        Impl: FnMut(Event, DispatchData) + 'static,
    {
        let pending_configure = Rc::new(RefCell::new(None));
        let pending_configure_2 = pending_configure.clone();

        let implementation = Rc::new(RefCell::new(implementation));
        let implementation_2 = implementation.clone();
        let xdgs = shell.get_xdg_surface(surface);
        xdgs.quick_assign(move |xdgs, evt, ddata| match evt {
            xdg_surface::Event::Configure { serial } => {
                xdgs.ack_configure(serial);
                if let Some((new_size, states)) = pending_configure_2.borrow_mut().take() {
                    (&mut *implementation_2.borrow_mut())(
                        Event::Configure { new_size, states },
                        ddata,
                    );
                }
            }
            _ => unreachable!(),
        });
        let toplevel = xdgs.get_toplevel();
        toplevel.quick_assign(move |_, evt, ddata| {
            match evt {
                xdg_toplevel::Event::Close => {
                    (&mut *implementation.borrow_mut())(Event::Close, ddata)
                }
                xdg_toplevel::Event::Configure { width, height, states } => {
                    use std::cmp::max;
                    let new_size = if width == 0 || height == 0 {
                        // if either w or h is zero, then we get to choose our size
                        None
                    } else {
                        Some((max(width, 1) as u32, max(height, 1) as u32))
                    };
                    let translated_states = states
                        .chunks_exact(4)
                        .map(|c| u32::from_ne_bytes(c.try_into().unwrap()))
                        .flat_map(xdg_toplevel::State::from_raw)
                        .collect::<Vec<_>>();

                    *pending_configure.borrow_mut() = Some((new_size, translated_states));
                }
                _ => unreachable!(),
            }
        });
        surface.commit();
        Xdg { surface: xdgs.detach(), toplevel: toplevel.detach() }
    }
}

impl ShellSurface for Xdg {
    fn resize(&self, seat: &wl_seat::WlSeat, serial: u32, edges: xdg_toplevel::ResizeEdge) {
        self.toplevel.resize(seat, serial, edges);
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

    fn show_window_menu(&self, seat: &wl_seat::WlSeat, serial: u32, x: i32, y: i32) {
        self.toplevel.show_window_menu(seat, serial, x, y);
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
        Some(&self.toplevel)
    }
}

impl Drop for Xdg {
    fn drop(&mut self) {
        self.toplevel.destroy();
        self.surface.destroy();
    }
}
