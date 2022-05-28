//! ## Cross desktop group (XDG) shell
// TODO: Examples

use wayland_client::{protocol::wl_surface, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_wm_base};

use crate::error::GlobalError;
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

    /// Creates an [`XdgShellSurface`].
    ///
    /// This function is generally intended to be called in a higher level abstraction, such as
    /// [`XdgWindowState::create_window`](self::window::XdgWindowState::create_window).
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
    pub fn create_xdg_surface<U, D>(
        &self,
        qh: &QueueHandle<D>,
        surface: wl_surface::WlSurface,
        udata: U,
    ) -> Result<XdgShellSurface, GlobalError>
    where
        D: Dispatch<xdg_surface::XdgSurface, U> + 'static,
        U: Send + Sync + 'static,
    {
        let wm_base = self.xdg_wm_base()?;
        let xdg_surface = wm_base.get_xdg_surface(&surface, qh, udata)?;

        Ok(XdgShellSurface { xdg_surface, surface })
    }
}

#[derive(Debug)]
pub struct XdgShellSurface {
    xdg_surface: xdg_surface::XdgSurface,
    surface: wl_surface::WlSurface,
}

impl XdgShellSurface {
    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        &self.xdg_surface
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.surface
    }
}

pub trait XdgShellHandler: Sized {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState;
}

#[macro_export]
macro_rules! delegate_xdg_shell {
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty: [
            $crate::reexports::protocols::xdg::shell::client::xdg_wm_base::XdgWmBase: (),
        ] => $crate::shell::xdg::XdgShellState);
    };
}

impl Drop for XdgShellSurface {
    fn drop(&mut self) {
        // Surface role must be destroyed before the wl_surface
        self.xdg_surface.destroy();
        self.surface.destroy();
    }
}
