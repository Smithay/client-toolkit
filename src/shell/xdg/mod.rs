//! ## Cross desktop group (XDG) shell
// TODO: Examples

use std::sync::{Arc, Mutex};
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::Connection;
use wayland_client::{protocol::wl_surface, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::Mode;
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};
use wayland_protocols::xdg::shell::client::{
    xdg_positioner, xdg_surface, xdg_toplevel, xdg_wm_base,
};

use crate::compositor::Surface;
use crate::error::GlobalError;
use crate::globals::{GlobalData, ProvidesBoundGlobal};
use crate::registry::GlobalProxy;

use self::window::inner::WindowInner;
use self::window::{
    DecorationMode, Window, WindowConfigure, WindowData, WindowDecorations, WindowHandler,
    WindowManagerCapabilities, WindowState,
};

use super::WaylandSurface;

pub mod frame;
pub mod popup;
pub mod window;

/// The xdg shell globals.
#[derive(Debug)]
pub struct XdgShell {
    xdg_wm_base: xdg_wm_base::XdgWmBase,
    xdg_decoration_manager: GlobalProxy<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
}

impl XdgShell {
    /// Binds the xdg shell global, `xdg_wm_base`.
    ///
    /// If available, the `zxdg_decoration_manager_v1` global will be bound to allow server side decorations
    /// for windows.
    ///
    /// # Errors
    ///
    /// This function will return [`Err`] if the `xdg_wm_base` global is not available.
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<xdg_wm_base::XdgWmBase, GlobalData, State>
            + Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, GlobalData, State>
            + 'static,
    {
        let xdg_wm_base = globals.bind(qh, 1..=5, GlobalData)?;
        let xdg_decoration_manager = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Ok(Self { xdg_wm_base, xdg_decoration_manager })
    }

    /// Creates a new, unmapped window.
    ///
    /// # Protocol errors
    ///
    /// If the surface already has a role object, the compositor will raise a protocol error.
    ///
    /// A surface is considered to have a role object if some other type of surface was created using the
    /// surface. For example, creating a window, popup, layer or subsurface all assign a role object to a
    /// surface.
    ///
    /// This function takes ownership of the surface.
    ///
    /// For more info related to creating windows, see [`the module documentation`](self).
    #[must_use = "Dropping all window handles will destroy the window"]
    pub fn create_window<State>(
        &self,
        surface: impl Into<Surface>,
        decorations: WindowDecorations,
        qh: &QueueHandle<State>,
    ) -> Window
    where
        State: Dispatch<xdg_surface::XdgSurface, WindowData>
            + Dispatch<xdg_toplevel::XdgToplevel, WindowData>
            + Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, WindowData>
            + WindowHandler
            + 'static,
    {
        let decoration_manager = self.xdg_decoration_manager.get().ok();
        let surface = surface.into();

        // Freeze the queue during the creation of the Arc to avoid a race between events on the
        // new objects being processed and the Weak in the PopupData becoming usable.
        let freeze = qh.freeze();

        let inner = Arc::new_cyclic(|weak| {
            let xdg_surface = self.xdg_wm_base.get_xdg_surface(
                surface.wl_surface(),
                qh,
                WindowData(weak.clone()),
            );
            let xdg_surface = XdgShellSurface { surface, xdg_surface };
            let xdg_toplevel = xdg_surface.xdg_surface().get_toplevel(qh, WindowData(weak.clone()));

            // If server side decorations are available, create the toplevel decoration.
            let toplevel_decoration = decoration_manager.and_then(|decoration_manager| {
                match decorations {
                    // Window does not want any server side decorations.
                    WindowDecorations::ClientOnly | WindowDecorations::None => None,

                    _ => {
                        // Create the toplevel decoration.
                        let toplevel_decoration = decoration_manager.get_toplevel_decoration(
                            &xdg_toplevel,
                            qh,
                            WindowData(weak.clone()),
                        );

                        // Tell the compositor we would like a specific mode.
                        let mode = match decorations {
                            WindowDecorations::RequestServer => Some(Mode::ServerSide),
                            WindowDecorations::RequestClient => Some(Mode::ClientSide),
                            _ => None,
                        };

                        if let Some(mode) = mode {
                            toplevel_decoration.set_mode(mode);
                        }

                        Some(toplevel_decoration)
                    }
                }
            });

            WindowInner {
                xdg_surface,
                xdg_toplevel,
                toplevel_decoration,
                pending_configure: Mutex::new(WindowConfigure {
                    new_size: (None, None),
                    suggested_bounds: None,
                    // Initial configure will indicate whether there are server side decorations.
                    decoration_mode: DecorationMode::Client,
                    state: WindowState::empty(),
                    // XXX by default we assume that everything is supported.
                    capabilities: WindowManagerCapabilities::all(),
                }),
            }
        });

        // Explicitly drop the queue freeze to allow the queue to resume work.
        drop(freeze);

        Window(inner)
    }

    pub fn xdg_wm_base(&self) -> &xdg_wm_base::XdgWmBase {
        &self.xdg_wm_base
    }
}

/// A trivial wrapper for an [`xdg_positioner::XdgPositioner`].
///
/// This wrapper calls [`destroy`][xdg_positioner::XdgPositioner::destroy] on the contained
/// positioner when it is dropped.
#[derive(Debug)]
pub struct XdgPositioner(xdg_positioner::XdgPositioner);

impl XdgPositioner {
    pub fn new(
        wm_base: &impl ProvidesBoundGlobal<xdg_wm_base::XdgWmBase, 4>,
    ) -> Result<Self, GlobalError> {
        wm_base
            .bound_global()
            .map(|wm_base| {
                wm_base
                    .send_constructor(
                        xdg_wm_base::Request::CreatePositioner {},
                        Arc::new(PositionerData),
                    )
                    .unwrap_or_else(|_| Proxy::inert(wm_base.backend().clone()))
            })
            .map(XdgPositioner)
    }
}

impl std::ops::Deref for XdgPositioner {
    type Target = xdg_positioner::XdgPositioner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for XdgPositioner {
    fn drop(&mut self) {
        self.0.destroy()
    }
}

struct PositionerData;

impl wayland_client::backend::ObjectData for PositionerData {
    fn event(
        self: Arc<Self>,
        _: &wayland_client::backend::Backend,
        _: wayland_client::backend::protocol::Message<
            wayland_client::backend::ObjectId,
            wayland_backend::io_lifetimes::OwnedFd,
        >,
    ) -> Option<Arc<(dyn wayland_client::backend::ObjectData + 'static)>> {
        unreachable!("xdg_positioner has no events");
    }
    fn destroyed(&self, _: wayland_client::backend::ObjectId) {}
}

/// A surface role for functionality common in desktop-like surfaces.
#[derive(Debug)]
pub struct XdgShellSurface {
    xdg_surface: xdg_surface::XdgSurface,
    surface: Surface,
}

impl XdgShellSurface {
    /// Creates an [`XdgShellSurface`].
    ///
    /// This function is generally intended to be called in a higher level abstraction, such as
    /// [`XdgShell::create_window`].
    ///
    /// The created [`XdgShellSurface`] will destroy the underlying [`XdgSurface`] or [`WlSurface`] when
    /// dropped. Higher level abstractions are responsible for ensuring the destruction order of protocol
    /// objects is correct. Since this function consumes the [`WlSurface`], it may be accessed using
    /// [`XdgShellSurface::wl_surface`].
    ///
    /// # Protocol errors
    ///
    /// If the surface already has a role object, the compositor will raise a protocol error.
    ///
    /// A surface is considered to have a role object if some other type of surface was created using the
    /// surface. For example, creating a window, popup, layer, subsurface or some other type of surface object
    /// all assign a role object to a surface.
    ///
    /// [`XdgSurface`]: xdg_surface::XdgSurface
    /// [`WlSurface`]: wl_surface::WlSurface
    pub fn new<U, D>(
        wm_base: &impl ProvidesBoundGlobal<xdg_wm_base::XdgWmBase, 4>,
        qh: &QueueHandle<D>,
        surface: impl Into<Surface>,
        udata: U,
    ) -> Result<XdgShellSurface, GlobalError>
    where
        D: Dispatch<xdg_surface::XdgSurface, U> + 'static,
        U: Send + Sync + 'static,
    {
        let surface = surface.into();
        let xdg_surface = wm_base.bound_global()?.get_xdg_surface(surface.wl_surface(), qh, udata);

        Ok(XdgShellSurface { xdg_surface, surface })
    }

    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        &self.xdg_surface
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.surface.wl_surface()
    }
}

pub trait XdgSurface: WaylandSurface + Sized {
    /// The underlying [`XdgSurface`](xdg_surface::XdgSurface).
    fn xdg_surface(&self) -> &xdg_surface::XdgSurface;

    fn set_window_geometry(&self, x: u32, y: u32, width: u32, height: u32) {
        self.xdg_surface().set_window_geometry(x as i32, y as i32, width as i32, height as i32);
    }
}

impl WaylandSurface for XdgShellSurface {
    fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.wl_surface()
    }
}

impl XdgSurface for XdgShellSurface {
    fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        &self.xdg_surface
    }
}

#[macro_export]
macro_rules! delegate_xdg_shell {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::xdg::shell::client::xdg_wm_base::XdgWmBase: $crate::globals::GlobalData
        ] => $crate::shell::xdg::XdgShell);
    };
}

impl Drop for XdgShellSurface {
    fn drop(&mut self) {
        // Surface role must be destroyed before the wl_surface
        self.xdg_surface.destroy();
    }
}

// Version 4 adds the configure_bounds event, which is a break
impl ProvidesBoundGlobal<xdg_wm_base::XdgWmBase, 4> for XdgShell {
    fn bound_global(&self) -> Result<xdg_wm_base::XdgWmBase, GlobalError> {
        Ok(self.xdg_wm_base.clone())
    }
}

impl<D> Dispatch<xdg_wm_base::XdgWmBase, GlobalData, D> for XdgShell
where
    D: Dispatch<xdg_wm_base::XdgWmBase, GlobalData>,
{
    fn event(
        _state: &mut D,
        xdg_wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        match event {
            xdg_wm_base::Event::Ping { serial } => {
                xdg_wm_base.pong(serial);
            }

            _ => unreachable!(),
        }
    }
}
