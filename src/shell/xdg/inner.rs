use wayland_client::{Connection, DelegateDispatch, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::xdg_wm_base;

use crate::registry::{ProvidesRegistryState, RegistryHandler};

use super::{XdgShellHandler, XdgShellState};

impl<D> RegistryHandler<D> for XdgShellState
where
    D: Dispatch<xdg_wm_base::XdgWmBase, ()> + XdgShellHandler + ProvidesRegistryState + 'static,
{
    fn new_global(
        data: &mut D,
        _conn: &Connection,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        if interface == "xdg_wm_base" {
            if data.xdg_shell_state().xdg_wm_base.is_some() {
                return;
            }

            let xdg_wm_base = data
                .registry()
                .bind_once::<xdg_wm_base::XdgWmBase, _, _>(qh, name, u32::min(version, 3), ())
                .expect("failed to bind global");

            data.xdg_shell_state().xdg_wm_base = Some((name, xdg_wm_base));
        }
    }

    fn remove_global(_: &mut D, _: &Connection, _: &QueueHandle<D>, _: u32) {
        // Unlikely to ever occur and the surfaces become inert if this happens.
    }
}

/* Delegate trait impls */

impl<D> DelegateDispatch<xdg_wm_base::XdgWmBase, (), D> for XdgShellState
where
    D: Dispatch<xdg_wm_base::XdgWmBase, ()> + XdgShellHandler,
{
    fn event(
        _: &mut D,
        xdg_wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            xdg_wm_base::Event::Ping { serial } => {
                xdg_wm_base.pong(serial);
            }

            _ => unreachable!(),
        }
    }
}
