use wayland_client::commons::Implementation;
use wayland_client::protocol::{wl_output, wl_seat, wl_surface};
use wayland_client::Proxy;

use wayland_protocols::xdg_shell::client::xdg_toplevel;

use super::Event;
use Shell;

mod wl;
mod xdg;
mod zxdg;

pub(crate) fn create_shell_surface<Impl>(
    shell: &Shell,
    surface: &Proxy<wl_surface::WlSurface>,
    implem: Impl,
) -> Box<ShellSurface>
where
    Impl: Implementation<(), Event> + Send,
{
    match *shell {
        Shell::Wl(ref shell) => Box::new(wl::Wl::create(surface, shell, implem)) as Box<_>,
        Shell::Xdg(ref shell) => Box::new(xdg::Xdg::create(surface, shell, implem)) as Box<_>,
        Shell::Zxdg(ref shell) => Box::new(zxdg::Zxdg::create(surface, shell, implem)) as Box<_>,
    }
}

pub(crate) trait ShellSurface: Send + Sync {
    fn resize(&self, seat: &Proxy<wl_seat::WlSeat>, serial: u32, edges: xdg_toplevel::ResizeEdge);
    fn move_(&self, seat: &Proxy<wl_seat::WlSeat>, serial: u32);
    fn set_title(&self, title: String);
    fn set_app_id(&self, app_id: String);
    fn set_fullscreen(&self, output: Option<&Proxy<wl_output::WlOutput>>);
    fn unset_fullscreen(&self);
    fn set_maximized(&self);
    fn unset_maximized(&self);
    fn set_minimized(&self);
    fn set_geometry(&self, x: i32, y: i32, width: i32, height: i32);
    fn set_min_size(&self, size: Option<(i32, i32)>);
    fn set_max_size(&self, size: Option<(i32, i32)>);
    fn get_xdg(&self) -> Option<&Proxy<xdg_toplevel::XdgToplevel>>;
}
