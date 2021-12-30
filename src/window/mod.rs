//! Higher level abstractions over the XDG shell protocol used to create windows and pop ups.

use std::{
    convert::TryInto,
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use wayland_client::{
    backend::InvalidId, protocol::wl_surface, ConnectionHandle, DelegateDispatch,
    DelegateDispatchBase, Dispatch, QueueHandle, WEnum,
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
    compositor::SurfaceData,
    registry::{RegistryHandle, RegistryHandler},
    window::inner::XdgToplevelInner,
};

use self::inner::{PendingConfigure, WindowInner, XdgSurfaceInner};

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
    fn request_close(&mut self, cx: &mut ConnectionHandle, qh: &QueueHandle<D>, window: &Window);

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
pub enum WindowError {
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
    ) -> Result<Window, WindowError>
    where
        D: Dispatch<wl_surface::WlSurface, UserData = SurfaceData>
            + Dispatch<xdg_surface::XdgSurface, UserData = XdgSurfaceData>
            + Dispatch<xdg_toplevel::XdgToplevel, UserData = WindowData>
            + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, UserData = WindowData>
            + 'static,
    {
        let inner = WindowInner::new(self, cx, qh, surface, decoration_mode)?;
        Ok(Window(inner))
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
    /// Maps the window.
    ///
    /// This function will send the initial commit to the server and will later result in an initial configure
    /// being received.
    ///
    /// ## Protocol errors
    ///
    /// It is a protocol error to attach a buffer to the surface when sending the initial configure.
    pub fn map(&self, cx: &mut ConnectionHandle) {
        self.0.map(cx)
    }

    /// Sets the minimum size of the window.
    ///
    /// If the minimum size is `(0, 0)`, the minimum size is unset and the compositor may assume any window
    /// size is valid.
    ///
    /// This value may actually be ignored by the compositor, but compositors can try to respect this minimum
    /// size value when sending configures.
    ///
    /// Smithay's client toolkit will ensure the interactive resizes of the window will not go below the
    /// minimum size.
    ///
    /// ## Double buffering
    ///
    /// This value is double buffered and will not be applied until the next commit of the window.
    pub fn set_min_size(&self, cx: &mut ConnectionHandle, min_size: (u32, u32)) {
        self.0.set_min_size(cx, min_size)
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
        self.0.set_max_size(cx, max_size)
    }

    pub fn set_title(&self, cx: &mut ConnectionHandle, title: impl Into<String>) {
        self.0.set_title(cx, title.into())
    }

    pub fn set_app_id(&self, cx: &mut ConnectionHandle, app_id: impl Into<String>) {
        self.0.set_app_id(cx, app_id.into())
    }

    pub fn xdg_toplevel(&self) -> &xdg_toplevel::XdgToplevel {
        self.0.xdg_toplevel()
    }

    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        self.0.xdg_surface()
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.0.wl_surface()
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
                            if let Some(window) = self.0.window_inner_from_surface(surface) {
                                H::configure(
                                    self.1,
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
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
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
                if let Some(window) = self.0.window_inner_from_toplevel(toplevel) {
                    self.1.request_close(cx, qh, &Window(window));
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

    fn remove_global(&mut self, _cx: &mut ConnectionHandle, _qh: &QueueHandle<D>, _name: u32) {
        todo!("xdg shell destruction")
    }
}

impl XdgShellState {
    fn cleanup(&mut self) {
        // TODO: How do we want to deal with cleanup of dead windows and surfaces?
    }

    fn window_inner_from_surface(
        &self,
        xdg_surface: &xdg_surface::XdgSurface,
    ) -> Option<Arc<WindowInner>> {
        self.windows
            .iter()
            .filter_map(Weak::upgrade)
            .find(|window| window.xdg_surface() == xdg_surface)
    }

    fn window_inner_from_toplevel(
        &self,
        xdg_toplevel: &xdg_toplevel::XdgToplevel,
    ) -> Option<Arc<WindowInner>> {
        self.windows
            .iter()
            .filter_map(Weak::upgrade)
            .find(|window| window.xdg_toplevel() == xdg_toplevel)
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
