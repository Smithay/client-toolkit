use std::sync::{Arc, Mutex, Weak};

use wayland_client::{
    protocol::wl_surface, ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch,
    QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
    },
    xdg_shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base},
};

use crate::registry::{ProvidesRegistryState, RegistryHandler};

use super::{
    window::{inner::WindowInner, Window, WindowHandler},
    XdgShellHandler, XdgShellState, XdgSurfaceData,
};

const MAX_XDG_WM_BASE: u32 = 3;
const MAX_ZXDG_DECORATION_MANAGER: u32 = 1;

impl XdgShellState {
    pub(crate) fn window_by_toplevel(
        &self,
        toplevel: &xdg_toplevel::XdgToplevel,
    ) -> Option<Window> {
        self.windows
            .iter()
            .filter_map(Weak::upgrade)
            .find(|inner| &inner.xdg_toplevel == toplevel)
            .map(Window)
    }
}

#[derive(Debug, Clone)]
pub enum XdgSurfaceDataInner {
    /// Uninitialized surface data.
    Uninit,

    Window(Arc<WindowInner>),
    // TODO: Popups
}

impl XdgSurfaceData {
    /// Creates uninitialized surface data for the surface.
    pub(crate) fn uninit() -> XdgSurfaceData {
        XdgSurfaceData(Arc::new(Mutex::new(XdgSurfaceDataInner::Uninit)))
    }

    /// Initializes the surface data as a window.
    pub(crate) fn init_window(
        &self,
        state: &mut XdgShellState,
        wl_surface: wl_surface::WlSurface,
        xdg_surface: xdg_surface::XdgSurface,
        xdg_toplevel: xdg_toplevel::XdgToplevel,
        zxdg_decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    ) -> Arc<WindowInner> {
        let mut inner = self.0.lock().unwrap();

        match *inner {
            XdgSurfaceDataInner::Uninit => {
                let window = WindowInner::new(
                    wl_surface,
                    xdg_surface,
                    xdg_toplevel,
                    zxdg_decoration_manager,
                );

                state.windows.push(Arc::downgrade(&window));
                *inner = XdgSurfaceDataInner::Window(window.clone());

                window
            }

            XdgSurfaceDataInner::Window { .. } => {
                unreachable!("XdgSurfaceData already initialized as window")
            }
        }
    }

    fn configure<D>(&self, handler: &mut D, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: WindowHandler,
    {
        let inner = self.0.lock().unwrap();

        match &*inner {
            XdgSurfaceDataInner::Window(window) => {
                let guard = window.pending_configure.lock().unwrap();
                let pending_configure = guard.clone();
                drop(guard);

                handler.configure_window(
                    conn,
                    qh,
                    pending_configure.new_size,
                    pending_configure.states,
                    &Window(window.clone()),
                );
            }

            _ => unreachable!("Configure on uninitialized surface"),
        }
    }
}

impl<D> RegistryHandler<D> for XdgShellState
where
    D: Dispatch<xdg_wm_base::XdgWmBase, UserData = ()>
        + Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, UserData = ()>
        // Decoration late-init
        + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = XdgSurfaceData>
        + XdgShellHandler
        + ProvidesRegistryState
        + 'static,
{
    fn new_global(
        state: &mut D,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        match interface {
            "xdg_wm_base" => {
                if state.xdg_shell_state().xdg_wm_base.is_some() {
                    log::warn!(target: "sctk", "compositor advertises xdg_wm_base but one is already bound");
                    return;
                }

                let xdg_wm_base = state
                    .registry()
                    .bind_once::<xdg_wm_base::XdgWmBase, _, _>(
                        conn,
                        qh,
                        name,
                        u32::min(version, MAX_XDG_WM_BASE),
                        (),
                    )
                    .expect("failed to bind global");

                state.xdg_shell_state().xdg_wm_base = Some((name, xdg_wm_base));
            }

            "zxdg_decoration_manager_v1" => {
                if state.xdg_shell_state().zxdg_decoration_manager_v1.is_some() {
                    log::warn!(target: "sctk", "compositor advertises zxdg_decoration_manager_v1 but one is already bound");
                    return;
                }

                let zxdg_decoration_manager_v1 = state
                    .registry()
                    .bind_once::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _>(
                        conn,
                        qh,
                        name,
                        MAX_ZXDG_DECORATION_MANAGER,
                        (),
                    )
                    .expect("failed to bind global");

                state.xdg_shell_state().zxdg_decoration_manager_v1 =
                    Some((name, zxdg_decoration_manager_v1));

                // Since the order in which globals are advertised is undefined, we need to ensure we enable
                // server side decorations if the decoration manager is advertised after any surfaces are
                // created.
                state.xdg_shell_state().init_decorations(conn, qh);
            }

            _ => (),
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

        if state
            .xdg_shell_state()
            .zxdg_decoration_manager_v1
            .as_ref()
            .filter(|(global_name, _)| global_name == &name)
            .is_some()
        {
            todo!("ZXDG decoration global destruction")
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
    type UserData = XdgSurfaceData;
}

impl<D> DelegateDispatch<xdg_surface::XdgSurface, D> for XdgShellState
where
    D: Dispatch<xdg_surface::XdgSurface, UserData = Self::UserData> + WindowHandler,
{
    fn event(
        state: &mut D,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        data: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                // Ack the configure
                surface.ack_configure(conn, serial);
                data.configure(state, conn, qh);
            }

            _ => unreachable!(),
        }
    }
}
