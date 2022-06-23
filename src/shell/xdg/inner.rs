use wayland_client::{Connection, DelegateDispatch, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::xdg_wm_base;

use crate::registry::{ProvidesRegistryState, RegistryHandler};

use super::{XdgShellHandler, XdgShellState};

impl<D> RegistryHandler<D> for XdgShellState
where
    D: Dispatch<xdg_wm_base::XdgWmBase, ()> + XdgShellHandler + ProvidesRegistryState + 'static,
{
    fn ready(data: &mut D, _conn: &Connection, qh: &QueueHandle<D>) {
        data.xdg_shell_state().xdg_wm_base = data.registry().bind_one(qh, 1..=4, ()).into();
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
