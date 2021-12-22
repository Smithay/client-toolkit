use wayland_client::{
    ConnectionHandle, DataInit, DelegateDispatch, DelegateDispatchBase, Dispatch, QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
    },
    xdg_shell::client::{xdg_popup, xdg_surface, xdg_toplevel, xdg_wm_base},
};

use crate::registry::{RegistryHandle, RegistryHandler};

#[derive(Debug)]
pub struct XdgShellState {
    pub(crate) wm_base: Option<(u32, xdg_wm_base::XdgWmBase)>, // (name, global)
    pub(crate) zxdg_decoration_manager:
        Option<(u32, zxdg_decoration_manager_v1::ZxdgDecorationManagerV1)>,
    pub(crate) _surfaces: Vec<XdgSurfaceInner>,
}

impl XdgShellState {
    pub fn new() -> XdgShellState {
        XdgShellState { wm_base: None, zxdg_decoration_manager: None, _surfaces: vec![] }
    }
}

#[derive(Debug)]
pub struct XdgShellDispatch<'s, H>(pub &'s mut XdgShellState, pub &'s H);

impl<H> DelegateDispatchBase<xdg_wm_base::XdgWmBase> for XdgShellDispatch<'_, H> {
    type UserData = ();
}

impl<D, H> DelegateDispatch<xdg_wm_base::XdgWmBase, D> for XdgShellDispatch<'_, H>
where
    D: Dispatch<xdg_wm_base::XdgWmBase, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        cx: &mut ConnectionHandle,
        _: &QueueHandle<D>,
        _: &mut DataInit<'_>,
    ) {
        match event {
            xdg_wm_base::Event::Ping { serial } => {
                wm_base.pong(cx, serial);
            }

            _ => unreachable!(),
        }
    }
}

impl<H> DelegateDispatchBase<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>
    for XdgShellDispatch<'_, H>
{
    type UserData = ();
}

impl<D, H> DelegateDispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, D>
    for XdgShellDispatch<'_, H>
where
    D: Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        _: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _: zxdg_decoration_manager_v1::Event,
        _: &(),
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
        _: &mut DataInit<'_>,
    ) {
        unreachable!("zxdg_decoration_manager_v1 has no events")
    }
}

impl<D> RegistryHandler<D> for XdgShellState
where
    D: Dispatch<xdg_wm_base::XdgWmBase, UserData = ()>
        + Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, UserData = ()>
        + 'static,
{
    fn new_global(
        &mut self,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
        handle: &mut RegistryHandle,
    ) {
        match interface {
            "xdg_wm_base" => {
                let wm_base = handle
                    .bind_once::<xdg_wm_base::XdgWmBase, _, _>(
                        cx,
                        qh,
                        name,
                        u32::min(version, 3),
                        (),
                    )
                    .expect("Failed to bind global");

                log::debug!(target: "sctk", "xdg_wm_base v{} bound", u32::min(version, 3));

                self.wm_base = Some((name, wm_base));
            }

            "zxdg_decoration_manager_v1" => {
                let zxdg_decoration_manager = handle
                    .bind_once::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _>(
                        cx,
                        qh,
                        name,
                        1,
                        (),
                    )
                    .expect("Failed to bind global");

                log::debug!(target: "sctk", "zxdg_decoration_manager_v1 v1 bound");

                self.zxdg_decoration_manager = Some((name, zxdg_decoration_manager));
            }

            _ => (),
        }
    }

    fn remove_global(&mut self, _cx: &mut ConnectionHandle, _name: u32) {
        todo!("xdg shell destruction")
    }
}

#[derive(Debug)]
pub(crate) struct XdgToplevelInner {
    pub(crate) surface: xdg_surface::XdgSurface,
    pub(crate) toplevel: xdg_toplevel::XdgToplevel,
    pub(crate) decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
}

#[derive(Debug)]
pub(crate) struct XdgPopupInner {
    pub(crate) surface: xdg_surface::XdgSurface,
    pub(crate) popup: xdg_popup::XdgPopup,
}

#[derive(Debug)]
pub(crate) enum XdgSurfaceInner {
    Toplevel(XdgToplevelInner),

    Popup(XdgToplevelInner),
}
