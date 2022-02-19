use std::{
    convert::TryFrom,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_output, wl_surface},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1,
        zxdg_toplevel_decoration_v1::{self, Mode},
    },
    xdg_shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
        xdg_wm_base,
    },
};

use crate::{
    registry::{ProvidesRegistryState, RegistryHandler},
    shell::xdg::inner::MAX_XDG_WM_BASE,
};

use super::{DecorationMode, Window, WindowConfigure, WindowData, WindowHandler, XdgWindowState};

impl Window {
    /// Clone is an implementation detail of Window.
    ///
    /// This function creates another window handle that is not marked as a primary handle.
    pub(crate) fn impl_clone(&self) -> Window {
        Window {
            inner: self.inner.clone(),
            primary: false,
            death_signal: self.death_signal.clone(),
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // If we are the primary handle (an owned value given to the user), mark ourselves for cleanup.
        if self.primary {
            self.death_signal.store(true, Ordering::SeqCst);
        }
    }
}

#[derive(Debug)]
pub struct WindowInner {
    pub(crate) wl_surface: wl_surface::WlSurface,
    pub(crate) xdg_surface: xdg_surface::XdgSurface,
    pub(crate) xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub(crate) zxdg_decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    pub(crate) zxdg_toplevel_decoration:
        Mutex<Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>>,

    pub(crate) data: Arc<WindowDataInner>,
}

impl WindowInner {
    pub fn new<D>(
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        wl_surface: &wl_surface::WlSurface,
        xdg_surface: &xdg_surface::XdgSurface,
        zxdg_decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    ) -> Result<Arc<WindowInner>, InvalidId>
    where
        D: Dispatch<xdg_toplevel::XdgToplevel, UserData = WindowData> + 'static,
    {
        let inner = WindowDataInner {
            pending_configure: Mutex::new(None),
            prefered_decoration_mode: Mutex::new(None),
            current_decoration_mode: Mutex::new(Mode::ClientSide),
            first_configure: AtomicBool::new(true),
        };

        let data = Arc::new(inner);
        let window_data = WindowData(data.clone());

        let xdg_toplevel = xdg_surface.get_toplevel(conn, qh, window_data)?;

        let inner = Arc::new(WindowInner {
            wl_surface: wl_surface.clone(),
            xdg_surface: xdg_surface.clone(),
            xdg_toplevel,
            zxdg_decoration_manager,
            zxdg_toplevel_decoration: Mutex::new(None),
            data,
        });

        Ok(inner)
    }

    pub fn map<D>(&self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        self.maybe_create_decoration(conn, qh);
        self.wl_surface.commit(conn);
    }

    #[must_use]
    pub fn configure(&self) -> Option<WindowConfigure> {
        self.data.pending_configure.lock().unwrap().take()
    }

    pub fn set_title(&self, conn: &mut ConnectionHandle, title: String) {
        self.xdg_toplevel.set_title(conn, title);
        // TODO: Store name for client side frame.
    }

    pub fn set_app_id(&self, conn: &mut ConnectionHandle, app_id: String) {
        self.xdg_toplevel.set_app_id(conn, app_id);
    }

    pub fn set_min_size(&self, conn: &mut ConnectionHandle, min_size: Option<(u32, u32)>) {
        let min_size = min_size.unwrap_or((0, 0));
        self.xdg_toplevel.set_min_size(conn, min_size.0 as i32, min_size.1 as i32)
    }

    pub fn set_max_size(&self, conn: &mut ConnectionHandle, max_size: Option<(u32, u32)>) {
        let max_size = max_size.unwrap_or((0, 0));
        self.xdg_toplevel.set_max_size(conn, max_size.0 as i32, max_size.1 as i32)
    }

    pub fn set_parent(&self, conn: &mut ConnectionHandle, parent: Option<&Window>) {
        self.xdg_toplevel.set_parent(conn, parent.map(Window::xdg_toplevel))
    }

    pub fn set_maximized(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel.set_maximized(conn)
    }

    pub fn unset_maximized(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel.unset_maximized(conn)
    }

    pub fn set_minmized(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel.set_minimized(conn)
    }

    pub fn set_fullscreen(
        &self,
        conn: &mut ConnectionHandle,
        output: Option<&wl_output::WlOutput>,
    ) {
        self.xdg_toplevel.set_fullscreen(conn, output)
    }

    pub fn unset_fullscreen(&self, conn: &mut ConnectionHandle) {
        self.xdg_toplevel.unset_fullscreen(conn)
    }

    fn maybe_create_decoration<D>(&self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        if let Some(decoration_manager) = self.zxdg_decoration_manager.as_ref() {
            let guard = self.data.prefered_decoration_mode.lock().unwrap();
            // By default we assume the server should be preferred.
            let preferred = guard.unwrap_or(DecorationMode::PreferServer);
            drop(guard);

            match preferred {
                // Do not create the toplevel decoration.
                DecorationMode::ClientOnly | DecorationMode::None => (),

                _ => {
                    // Create the toplevel decoration.
                    let data = self.xdg_toplevel.data::<WindowData>().unwrap().clone();

                    let zxdg_toplevel_decoration = decoration_manager
                        .get_toplevel_decoration(conn, &self.xdg_toplevel, qh, data)
                        .expect("failed to create toplevel decoration");

                    // Specifically request server side if requested.
                    if preferred == DecorationMode::ServerOnly {
                        zxdg_toplevel_decoration.set_mode(conn, Mode::ServerSide);
                    }

                    *self.zxdg_toplevel_decoration.lock().unwrap() = Some(zxdg_toplevel_decoration);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct WindowDataInner {
    pub(crate) pending_configure: Mutex<Option<WindowConfigure>>,
    pub(crate) prefered_decoration_mode: Mutex<Option<DecorationMode>>,
    pub(crate) current_decoration_mode: Mutex<Mode>,
    pub(crate) first_configure: AtomicBool,
}

impl WindowDataInner {}

impl XdgWindowState {
    pub(crate) fn init_decorations<D>(&mut self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        for window in self.windows.iter() {
            // Only create decorations if the window is live.
            if !window.inner.data.first_configure.load(Ordering::SeqCst) {
                window.inner.maybe_create_decoration(conn, qh);
            }
        }
    }

    pub(crate) fn cleanup(&mut self, conn: &mut ConnectionHandle) {
        self.windows.retain(|window| {
            let alive = !window.death_signal.load(Ordering::SeqCst);

            if !alive {
                // XDG decoration says we must destroy the decoration object before the toplevel
                if let Some(decoration) = &*window.inner.zxdg_toplevel_decoration.lock().unwrap() {
                    decoration.destroy(conn);
                }

                // XDG Shell protocol dictates we must destroy the role object before the xdg surface.
                window.xdg_toplevel().destroy(conn);
                window.xdg_surface().destroy(conn);
                window.wl_surface().destroy(conn);
            }

            alive
        })
    }
}

const MAX_ZXDG_DECORATION_MANAGER: u32 = 1;

impl<D> RegistryHandler<D> for XdgWindowState
where
    D: Dispatch<xdg_wm_base::XdgWmBase, UserData = ()>
        + Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, UserData = ()>
        // Lateinit for decorations
        + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
        + WindowHandler
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
        match interface {
            "xdg_wm_base" => {
                if data.xdg_window_state().xdg_wm_base.is_some() {
                    log::warn!(target: "sctk", "compositor advertises xdg_wm_base but one is already bound");
                    return;
                }

                let xdg_wm_base = data
                    .registry()
                    .bind_cached::<xdg_wm_base::XdgWmBase, _, _, _>(conn, qh, name, || {
                        (u32::min(version, MAX_XDG_WM_BASE), ())
                    })
                    .expect("failed to bind global");

                data.xdg_window_state().xdg_wm_base = Some((name, xdg_wm_base));
            }

            "zxdg_decoration_manager_v1" => {
                if data.xdg_window_state().zxdg_decoration_manager_v1.is_some() {
                    log::warn!(target: "sctk", "compositor advertises zxdg_decoration_manager_v1 but one is already bound");
                    return;
                }

                let zxdg_decoration_manager_v1 = data
                    .registry()
                    .bind_once::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _>(
                        conn,
                        qh,
                        name,
                        MAX_ZXDG_DECORATION_MANAGER,
                        (),
                    )
                    .expect("failed to bind global");

                data.xdg_window_state().zxdg_decoration_manager_v1 =
                    Some((name, zxdg_decoration_manager_v1));

                // Since the order in which globals are advertised is undefined, we need to ensure we enable
                // server side decorations if the decoration manager is advertised after any surfaces are
                // created.
                data.xdg_window_state().init_decorations(conn, qh);
            }

            _ => (),
        }
    }

    fn remove_global(data: &mut D, _: &mut ConnectionHandle, _: &QueueHandle<D>, name: u32) {
        if data
            .xdg_window_state()
            .xdg_wm_base
            .as_ref()
            .filter(|(global_name, _)| global_name == &name)
            .is_some()
        {
            todo!("XDG shell global destruction")
        }

        if data
            .xdg_window_state()
            .zxdg_decoration_manager_v1
            .as_ref()
            .filter(|(global_name, _)| global_name == &name)
            .is_some()
        {
            todo!("ZXDG decoration global destruction")
        }
    }
}

impl DelegateDispatchBase<xdg_toplevel::XdgToplevel> for XdgWindowState {
    type UserData = WindowData;
}

impl<D> DelegateDispatch<xdg_toplevel::XdgToplevel, D> for XdgWindowState
where
    D: Dispatch<xdg_toplevel::XdgToplevel, UserData = Self::UserData> + WindowHandler,
{
    fn event(
        data: &mut D,
        toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        udata: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, states } => {
                let states = states
                    .iter()
                    .cloned()
                    .map(|entry| entry as u32)
                    .map(State::try_from)
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>();

                let new_size = if width == 0 && height == 0 {
                    None
                } else {
                    Some((width as u32, height as u32))
                };

                let pending_configure = &mut *udata.0.pending_configure.lock().unwrap();

                match pending_configure {
                    Some(pending_configure) => {
                        pending_configure.new_size = new_size;
                        pending_configure.states = states;
                    }

                    None => {
                        *pending_configure = Some(WindowConfigure { new_size, states });
                    }
                }
            }

            xdg_toplevel::Event::Close => {
                if let Some(window) = data.xdg_window_state().window_by_toplevel(toplevel) {
                    let window = window.impl_clone();

                    data.request_close_window(conn, qh, &window);
                } else {
                    log::warn!(target: "sctk", "closed event received for dead window: {}", toplevel.id());
                }
            }

            _ => unreachable!(),
        }

        // Perform cleanup as necessary
        data.xdg_window_state().cleanup(conn);
    }
}

// XDG decoration

impl DelegateDispatchBase<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1> for XdgWindowState {
    type UserData = ();
}

impl<D> DelegateDispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, D> for XdgWindowState
where
    D: Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, UserData = Self::UserData>
        + WindowHandler,
{
    fn event(
        _: &mut D,
        _: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _: zxdg_decoration_manager_v1::Event,
        _: &(),
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zxdg_decoration_manager_v1 has no events")
    }
}

impl DelegateDispatchBase<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>
    for XdgWindowState
{
    type UserData = WindowData;
}

impl<D> DelegateDispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, D>
    for XdgWindowState
where
    D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = Self::UserData>
        + WindowHandler,
{
    fn event(
        _: &mut D,
        _: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        data: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        match event {
            zxdg_toplevel_decoration_v1::Event::Configure { mode } => match mode {
                wayland_client::WEnum::Value(mode) => {
                    *data.0.current_decoration_mode.lock().unwrap() = mode;
                }

                wayland_client::WEnum::Unknown(_) => unreachable!(),
            },

            _ => unreachable!(),
        }
    }
}
