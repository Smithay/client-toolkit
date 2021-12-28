//! Higher level abstractions over the XDG shell protocol used to create windows and pop ups.

use std::{
    convert::TryInto,
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use wayland_client::{
    backend::InvalidId, protocol::wl_surface, ConnectionHandle, DelegateDispatch,
    DelegateDispatchBase, Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
    },
    xdg_shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
        xdg_wm_base,
    },
};

use crate::{
    compositor::{SurfaceData, SurfaceRole},
    registry::{RegistryHandle, RegistryHandler},
    window::inner::{WindowInner, XdgToplevelInner},
};

use self::inner::{PendingConfigure, XdgSurfaceInner};

pub(crate) mod inner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationMode {
    /// The server should decide which decoration mode to use.
    ///
    /// This is probably the best option.
    ServerDecides,

    /// Requests that the server should draw decorations if possible.
    ServerSide,

    /// The client will draw decorations.
    ClientSide,

    /// The client should not have any decorations.
    None,
}

pub trait ShellHandler<D> {
    /// A request to close the window has been received.
    ///
    /// This typically will be sent if
    fn request_close(&mut self, window: &Window);

    /// Called when the compositor asks to resize a window and or change the state of the window.
    ///
    /// The size of the window specifies how the window geometry should be changed.
    ///
    /// If the width and height are zero, you may set the window to your desired size.
    ///
    /// A large number of these events may be batched during an interactive resize. You only need to handle
    /// the last one of the batch
    fn configure(
        &mut self,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        size: (u32, u32),
        states: Vec<State>,
        window: &Window,
    );
}

#[derive(Debug, thiserror::Error)]
pub enum XdgShellError {
    /// Required globals are not present.
    #[error("xdg_wm_base global is not bound")]
    MissingXdgShellGlobal,

    /// The surface already has a role object
    #[error("the wl_surface already has a role object")]
    HasRole,

    /// Protocol error
    #[error(transparent)]
    Protocol(#[from] InvalidId),
}

#[derive(Debug)]
pub struct XdgShellState {
    pub(crate) wm_base: Option<(u32, xdg_wm_base::XdgWmBase)>, // (name, global)
    pub(crate) zxdg_decoration_manager:
        Option<(u32, zxdg_decoration_manager_v1::ZxdgDecorationManagerV1)>,
    pub(crate) windows: Vec<Weak<WindowInner>>,
}

impl XdgShellState {
    pub fn new() -> XdgShellState {
        XdgShellState { wm_base: None, zxdg_decoration_manager: None, windows: vec![] }
    }

    /// Creates a new window.
    ///
    /// The passed in [`wl_surface::WlSurface`] must not have any other role object associated with it.
    ///
    /// Per protocol requirements, you may not attach or commit any buffers until the initial configure of the
    /// window.
    pub fn create_window<D>(
        &mut self,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        surface: wl_surface::WlSurface,
        decoration_mode: DecorationMode,
    ) -> Result<Window, XdgShellError>
    where
        D: Dispatch<wl_surface::WlSurface, UserData = SurfaceData>
            + Dispatch<xdg_surface::XdgSurface, UserData = XdgSurfaceData>
            + Dispatch<xdg_toplevel::XdgToplevel, UserData = WindowData>
            + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        let wl_surface = surface.clone();
        let surface_data = surface.data::<SurfaceData>().unwrap();
        let mut role = surface_data.role.lock().unwrap();

        match *role {
            SurfaceRole::None => {
                let (_, wm_base) =
                    self.wm_base.as_ref().ok_or(XdgShellError::MissingXdgShellGlobal)?;
                let decoration_manager =
                    self.zxdg_decoration_manager.clone().map(|(_, manager)| manager);

                let inner = Arc::new(Mutex::new(XdgSurfaceInner::Uninit));
                let xdg_surface_data = XdgSurfaceData { inner };
                let xdg_surface =
                    wm_base.get_xdg_surface(cx, surface.clone(), qh, xdg_surface_data.clone())?;

                let inner = Arc::new(Mutex::new(XdgToplevelInner {
                    decoration_manager: decoration_manager.clone(),
                    decoration: None,

                    title: None,
                    app_id: None,
                    decoration_mode,
                    min_size: (0, 0),
                    max_size: (0, 0),
                    pending_configure: None,
                }));

                {
                    // Ugly but we need to give the xdg surface the toplevel's data.
                    let mut data =
                        xdg_surface.data::<XdgSurfaceData>().unwrap().inner.lock().unwrap();

                    *data = XdgSurfaceInner::Window(inner.clone());
                }

                let window_data = WindowData { inner };
                let xdg_toplevel = xdg_surface.get_toplevel(cx, qh, window_data.clone())?;

                match decoration_mode {
                    // Do not create the decoration manager.
                    DecorationMode::None | DecorationMode::ClientSide => (),

                    _ => {
                        let decoration = decoration_manager
                            .as_ref()
                            .map(|manager| {
                                manager.get_toplevel_decoration(
                                    cx,
                                    xdg_toplevel.clone(),
                                    qh,
                                    window_data.clone(),
                                )
                            })
                            .transpose()?;

                        if let Some(decoration) = decoration.clone() {
                            // Explicitly ask the server for server side decorations
                            if decoration_mode == DecorationMode::ServerSide {
                                decoration
                                    .set_mode(cx, zxdg_toplevel_decoration_v1::Mode::ServerSide);
                            }

                            window_data.inner.lock().unwrap().decoration = Some(decoration);
                        }
                    }
                }

                // Perform an initial commit without any buffer attached per the xdg_surface requirements.
                wl_surface.commit(cx);

                let window_inner = WindowInner {
                    wl_surface,
                    xdg_surface,
                    toplevel: window_data.inner,
                    xdg_toplevel,
                };

                // Now assign the role.
                *role = SurfaceRole::Toplevel;
                drop(role);

                let inner = Arc::new(window_inner);
                self.windows.push(Arc::downgrade(&inner));

                Ok(Window(inner))
            }

            _ => Err(XdgShellError::HasRole),
        }
    }
}

#[derive(Debug, Clone)]
pub struct XdgSurfaceData {
    inner: Arc<Mutex<XdgSurfaceInner>>,
}

#[derive(Debug, Clone)]
pub struct WindowData {
    inner: Arc<Mutex<XdgToplevelInner>>,
}

#[derive(Debug)]
pub struct Window(Arc<inner::WindowInner>);

impl Window {
    /// Sets the minimum size of the window.
    ///
    /// If the minimum size is `(0, 0)`, the minimum size is unset and the compositor may assume any window size is
    /// valid.
    ///
    /// This value may actually be ignored by the compositor, but most compositors will try to respect this minimum
    /// size value when sending configures.
    ///
    /// Smithay's client toolkit will ensure the interactive resizes of the window will not go below the minimum size.
    ///
    /// ## Double buffering
    ///
    /// This value is double buffered and will not be applied until the next commit of the window.
    pub fn set_min_size(&self, cx: &mut ConnectionHandle, min_size: (u32, u32)) {
        self.0.xdg_toplevel.set_min_size(cx, min_size.0 as i32, min_size.1 as i32);

        let mut toplevel = self.0.toplevel.lock().unwrap();
        toplevel.min_size = min_size;
    }

    /// Sets the maximum size of the window.
    ///
    /// If the maximum size is `(0, 0)`, the maximum size is unset and the compositor may assume any window size is
    /// valid.
    ///
    /// This value may actually be ignored by the compositor, but most compositors will try to respect this maximum
    /// size value when sending configures.
    ///
    /// Smithay's client toolkit will ensure the interactive resizes of the window will not go below the maximum size.
    ///
    /// ## Double buffering
    ///
    /// This value is double buffered and will not be applied until the next commit of the window.
    pub fn set_max_size(&self, cx: &mut ConnectionHandle, max_size: (u32, u32)) {
        self.0.xdg_toplevel.set_max_size(cx, max_size.0 as i32, max_size.1 as i32);

        let mut toplevel = self.0.toplevel.lock().unwrap();
        toplevel.max_size = max_size;
    }

    pub fn set_title(&self, cx: &mut ConnectionHandle, title: impl Into<String>) {
        let title = title.into();
        self.0.xdg_toplevel.set_title(cx, title.clone());

        let mut toplevel = self.0.toplevel.lock().unwrap();
        toplevel.title = Some(title);
    }

    pub fn set_app_id(&self, cx: &mut ConnectionHandle, app_id: impl Into<String>) {
        let app_id = app_id.into();
        self.0.xdg_toplevel.set_app_id(cx, app_id.clone());

        let mut toplevel = self.0.toplevel.lock().unwrap();
        toplevel.app_id = Some(app_id);
    }

    pub fn xdg_toplevel(&self) -> &xdg_toplevel::XdgToplevel {
        &self.0.xdg_toplevel
    }

    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        &self.0.xdg_surface
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.0.wl_surface
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Debug)]
pub struct XdgShellDispatch<'s, D, H: ShellHandler<D>>(
    pub &'s mut XdgShellState,
    pub &'s mut H,
    pub PhantomData<D>,
);

impl<D, H: ShellHandler<D>> DelegateDispatchBase<xdg_wm_base::XdgWmBase>
    for XdgShellDispatch<'_, D, H>
{
    type UserData = ();
}

impl<D, H> DelegateDispatch<xdg_wm_base::XdgWmBase, D> for XdgShellDispatch<'_, D, H>
where
    D: Dispatch<xdg_wm_base::XdgWmBase, UserData = Self::UserData>,
    H: ShellHandler<D>,
{
    fn event(
        &mut self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        cx: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        match event {
            xdg_wm_base::Event::Ping { serial } => {
                wm_base.pong(cx, serial);
            }

            _ => unreachable!(),
        }
    }
}

impl<D, H: ShellHandler<D>>
    DelegateDispatchBase<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>
    for XdgShellDispatch<'_, D, H>
{
    type UserData = ();
}

impl<D, H> DelegateDispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, D>
    for XdgShellDispatch<'_, D, H>
where
    D: Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, UserData = Self::UserData>,
    H: ShellHandler<D>,
{
    fn event(
        &mut self,
        _: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _: zxdg_decoration_manager_v1::Event,
        _: &(),
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zxdg_decoration_manager_v1 has no events")
    }
}

impl<D, H: ShellHandler<D>> DelegateDispatchBase<xdg_surface::XdgSurface>
    for XdgShellDispatch<'_, D, H>
{
    type UserData = XdgSurfaceData;
}

impl<D, H> DelegateDispatch<xdg_surface::XdgSurface, D> for XdgShellDispatch<'_, D, H>
where
    D: Dispatch<xdg_surface::XdgSurface, UserData = Self::UserData>,
    H: ShellHandler<D>,
{
    fn event(
        &mut self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        data: &Self::UserData,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                surface.ack_configure(cx, serial);

                let data = data.inner.lock().unwrap();

                match &*data {
                    XdgSurfaceInner::Window(inner) => {
                        let mut inner = inner.lock().unwrap();

                        if let Some(pending_configure) = inner.pending_configure.take() {
                            if let Some(window) = self
                                .0
                                .windows
                                .iter()
                                .filter_map(Weak::upgrade)
                                .find(|window| &window.xdg_surface == surface)
                            {
                                H::configure(
                                    &mut self.1,
                                    cx,
                                    qh,
                                    pending_configure.size,
                                    pending_configure.states,
                                    &Window(window),
                                );
                            }
                        }
                    }

                    XdgSurfaceInner::Popup(_) => todo!("don't know how to configure popup yet"),

                    XdgSurfaceInner::Uninit => unreachable!(),
                }
            }

            _ => unreachable!(),
        }
    }
}

impl<D, H: ShellHandler<D>> DelegateDispatchBase<xdg_toplevel::XdgToplevel>
    for XdgShellDispatch<'_, D, H>
{
    type UserData = WindowData;
}

impl<D, H> DelegateDispatch<xdg_toplevel::XdgToplevel, D> for XdgShellDispatch<'_, D, H>
where
    D: Dispatch<xdg_toplevel::XdgToplevel, UserData = Self::UserData>,
    H: ShellHandler<D>,
{
    fn event(
        &mut self,
        toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        data: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        match event {
            // TODO: Configures
            xdg_toplevel::Event::Configure { width, height, states } => {
                let states = states
                    .iter()
                    .copied()
                    // No impl of TryFrom<u8>
                    .map(|v| v as u32)
                    .map(TryInto::<State>::try_into)
                    // Discard any states we don't know how to handle.
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>();

                let mut data = data.inner.lock().unwrap();

                data.pending_configure =
                    Some(PendingConfigure { size: (width as u32, height as u32), states });
            }

            xdg_toplevel::Event::Close => {
                if let Some(window) = self
                    .0
                    .windows
                    .iter()
                    .filter_map(Weak::upgrade)
                    .find(|window| &window.xdg_toplevel == toplevel)
                {
                    self.1.request_close(&Window(window));
                }

                self.0.cleanup();
            }

            _ => unreachable!(),
        }
    }
}

impl<D, H: ShellHandler<D>>
    DelegateDispatchBase<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>
    for XdgShellDispatch<'_, D, H>
{
    type UserData = WindowData;
}

impl<D, H> DelegateDispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, D>
    for XdgShellDispatch<'_, D, H>
where
    D: Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = Self::UserData>,
    H: ShellHandler<D>,
{
    fn event(
        &mut self,
        _: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        data: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        match event {
            zxdg_toplevel_decoration_v1::Event::Configure { mode } => {
                match mode {
                    WEnum::Value(mode) => {
                        let mut _data = data.inner.lock().unwrap();

                        log::debug!(target: "sctk", "request to switch to decoration mode {:?}", mode);
                        // TODO: Propagate this information.
                    }

                    WEnum::Unknown(unknown) => {
                        log::error!(target: "sctk", "received unknown decoration mode {:x}", unknown);
                    }
                }
            }

            _ => unreachable!(),
        }
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

                self.zxdg_decoration_manager = Some((name, zxdg_decoration_manager));

                // TODO: Tell all existing toplevel surfaces about the manager now existing.
            }

            _ => (),
        }
    }

    fn remove_global(&mut self, _cx: &mut ConnectionHandle, _name: u32) {
        todo!("xdg shell destruction")
    }
}

impl XdgShellState {
    fn cleanup(&mut self) {
        // TODO: How do we want to deal with cleanup of dead windows and surfaces?
    }
}

impl From<zxdg_toplevel_decoration_v1::Mode> for DecorationMode {
    fn from(mode: zxdg_toplevel_decoration_v1::Mode) -> Self {
        match mode {
            zxdg_toplevel_decoration_v1::Mode::ClientSide => DecorationMode::ClientSide,
            zxdg_toplevel_decoration_v1::Mode::ServerSide => DecorationMode::ServerSide,

            _ => unreachable!(),
        }
    }
}
