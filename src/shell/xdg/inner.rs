use wayland_client::{
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, QueueHandle,
};
use wayland_protocols::xdg_shell::client::{xdg_surface, xdg_wm_base};

use crate::registry::{ProvidesRegistryState, RegistryHandler};

use super::{XdgShellHandler, XdgShellState};

pub(crate) const MAX_XDG_WM_BASE: u32 = 3;

impl<D> RegistryHandler<D> for XdgShellState
where
    D: Dispatch<xdg_wm_base::XdgWmBase, UserData = ()>
        + XdgShellHandler
        + ProvidesRegistryState
        + 'static,
{
    fn new_global(
        data: &mut D,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        if interface == "xdg_wm_base" {
            if data.xdg_shell_state().xdg_wm_base.is_some() {
                log::warn!(target: "sctk", "compositor advertises xdg_wm_base but one is already bound");
                return;
            }

            let xdg_wm_base = data
                .registry()
                .bind_cached::<xdg_wm_base::XdgWmBase, _, _, _>(conn, qh, name, || {
                    (u32::min(version, MAX_XDG_WM_BASE), ())
                })
                .expect("failed to bind global");

            data.xdg_shell_state().xdg_wm_base = Some((name, xdg_wm_base));
        }
    }

    fn remove_global(state: &mut D, _: &mut ConnectionHandle, _: &QueueHandle<D>, name: u32) {
        if state
            .xdg_shell_state()
            .xdg_wm_base
            .as_ref()
            .filter(|(global_name, _)| global_name == &name)
            .is_some()
        {
            todo!("XDG shell global destruction")
        }
    }
}

/* Delegate trait impls */

impl DelegateDispatchBase<xdg_wm_base::XdgWmBase> for XdgShellState {
    type UserData = ();
}

impl<D> DelegateDispatch<xdg_wm_base::XdgWmBase, D> for XdgShellState
where
    D: Dispatch<xdg_wm_base::XdgWmBase, UserData = Self::UserData> + XdgShellHandler,
{
    fn event(
        _: &mut D,
        xdg_wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        conn: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        match event {
            xdg_wm_base::Event::Ping { serial } => {
                xdg_wm_base.pong(conn, serial);
            }

            _ => unreachable!(),
        }
    }
}

impl DelegateDispatchBase<xdg_surface::XdgSurface> for XdgShellState {
    type UserData = ();
}

impl<D> DelegateDispatch<xdg_surface::XdgSurface, D> for XdgShellState
where
    D: Dispatch<xdg_surface::XdgSurface, UserData = Self::UserData> + XdgShellHandler,
{
    fn event(
        data: &mut D,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                // Ack the configure
                surface.ack_configure(conn, serial);
                data.configure(conn, qh, surface);
            }

            _ => unreachable!(),
        }
    }
}
