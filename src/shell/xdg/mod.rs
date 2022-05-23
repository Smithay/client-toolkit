//! ## Cross desktop group (XDG) shell
// TODO: Examples

use std::marker::PhantomData;

use wayland_client::{protocol::wl_surface, Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_wm_base};

use crate::error::GlobalError;

mod inner;
pub mod popup;
pub mod window;

#[derive(Debug)]
pub struct XdgShellState<D> {
    // (name, global)
    xdg_wm_base: Option<(u32, xdg_wm_base::XdgWmBase)>,
    _marker: PhantomData<D>,
}

impl<D> XdgShellState<D> {
    pub fn new() -> Self {
        Self { xdg_wm_base: None, _marker: PhantomData }
    }

    pub fn xdg_wm_base(&self) -> Option<&xdg_wm_base::XdgWmBase> {
        self.xdg_wm_base.as_ref().map(|(_, global)| global)
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
    pub fn create_xdg_surface(
        &self,
        qh: &QueueHandle<D>,
        surface: wl_surface::WlSurface,
        configure_handler: impl ConfigureHandler<D> + Send + Sync + 'static,
    ) -> Result<XdgShellSurface, GlobalError>
    where
        D: Dispatch<xdg_surface::XdgSurface, XdgSurfaceData<D>> + 'static,
    {
        let wm_base = self.xdg_wm_base().ok_or(GlobalError::MissingGlobals(&["xdg_wm_base"]))?;
        let xdg_surface = wm_base.get_xdg_surface(
            &surface,
            qh,
            XdgSurfaceData { configure_handler: Box::new(configure_handler) },
        )?;

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
    fn xdg_shell_state(&mut self) -> &mut XdgShellState<Self>;
}

/// Trait that should be implemented by data used to create [`XdgSurfaceData`].
///
/// This trait exists to allow specialized configure functions to be implemented in a specific handler trait
/// of any XDG shell managed surfaces such as [`WindowHandler::configure`](self::window::WindowHandler::configure).
pub trait ConfigureHandler<D> {
    /// The surface has received a configure.
    ///
    /// A configure atomically indicates that a sequence of events describing how a surface has changed have
    /// all been sent.
    ///
    /// Implementations of this function should invoke a `configure` function on the specific handler trait
    /// such as [`WindowHandler`](self::window::WindowHandler).
    fn configure(
        &self,
        data: &mut D,
        conn: &Connection,
        qh: &QueueHandle<D>,
        xdg_surface: &xdg_surface::XdgSurface,
        serial: u32,
    );
}

/// Data associated with an [`XdgSurface`](xdg_surface::XdgSurface) protocol object.
#[allow(missing_debug_implementations)]
pub struct XdgSurfaceData<D> {
    configure_handler: Box<(dyn ConfigureHandler<D> + Send + Sync + 'static)>,
}

#[macro_export]
macro_rules! delegate_xdg_shell {
    ($ty: ty) => {
        type __XdgWmBase = $crate::reexports::protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
        type __XdgSurface = $crate::reexports::protocols::xdg::shell::client::xdg_surface::XdgSurface;

        $crate::reexports::client::delegate_dispatch!($ty: [
            __XdgWmBase: (),
            __XdgSurface: $crate::shell::xdg::XdgSurfaceData<$ty>
        ] => $crate::shell::xdg::XdgShellState<$ty>);
    };
}

impl Drop for XdgShellSurface {
    fn drop(&mut self) {
        // Surface role must be destroyed before the wl_surface
        self.xdg_surface.destroy();
        self.surface.destroy();
    }
}
