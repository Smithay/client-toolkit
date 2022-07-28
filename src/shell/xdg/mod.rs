//! ## Cross desktop group (XDG) shell
// TODO: Examples

use std::sync::Arc;
use wayland_client::{protocol::wl_surface, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_positioner, xdg_surface, xdg_wm_base};

use crate::compositor::Surface;
use crate::error::GlobalError;
use crate::globals::ProvidesBoundGlobal;
use crate::registry::GlobalProxy;

mod inner;
pub mod popup;
pub mod window;

#[derive(Debug)]
pub struct XdgShellState {
    xdg_wm_base: GlobalProxy<xdg_wm_base::XdgWmBase>,
}

impl XdgShellState {
    pub fn new() -> Self {
        Self { xdg_wm_base: GlobalProxy::NotReady }
    }

    pub fn xdg_wm_base(&self) -> Result<&xdg_wm_base::XdgWmBase, GlobalError> {
        self.xdg_wm_base.get()
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
            .bound_global()?
            .send_constructor(xdg_wm_base::Request::CreatePositioner {}, Arc::new(PositionerData))
            .map(XdgPositioner)
            .map_err(From::from)
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
        _: wayland_client::backend::protocol::Message<wayland_client::backend::ObjectId>,
    ) -> Option<Arc<(dyn wayland_client::backend::ObjectData + 'static)>> {
        unreachable!("xdg_positioner has no events");
    }
    fn destroyed(&self, _: wayland_client::backend::ObjectId) {}
}

#[derive(Debug)]
pub struct XdgShellSurface {
    xdg_surface: xdg_surface::XdgSurface,
    surface: Surface,
}

impl XdgShellSurface {
    /// Creates an [`XdgShellSurface`].
    ///
    /// This function is generally intended to be called in a higher level abstraction, such as
    /// [`Window::builder`](self::window::Window::builder).
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

pub trait XdgShellHandler: Sized {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState;
}

#[macro_export]
macro_rules! delegate_xdg_shell {
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty: [
            $crate::reexports::protocols::xdg::shell::client::xdg_wm_base::XdgWmBase: $crate::globals::GlobalData,
        ] => $crate::shell::xdg::XdgShellState);
    };
}

impl Drop for XdgShellSurface {
    fn drop(&mut self) {
        // Surface role must be destroyed before the wl_surface
        self.xdg_surface.destroy();
    }
}
