//! ## Cross desktop group (XDG) shell
// TODO: Examples

use std::sync::Arc;
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::Connection;
use wayland_client::{protocol::wl_surface, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_positioner, xdg_surface, xdg_wm_base};

use crate::compositor::Surface;
use crate::error::GlobalError;
use crate::globals::{GlobalData, ProvidesBoundGlobal};

pub mod popup;
pub mod window;

#[derive(Debug)]
pub struct XdgShellState {
    xdg_wm_base: xdg_wm_base::XdgWmBase,
}

impl XdgShellState {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<xdg_wm_base::XdgWmBase, GlobalData, State> + 'static,
    {
        let xdg_wm_base = globals.bind(qh, 1..=4, GlobalData)?;
        Ok(Self { xdg_wm_base })
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

#[macro_export]
macro_rules! delegate_xdg_shell {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::xdg::shell::client::xdg_wm_base::XdgWmBase: $crate::globals::GlobalData
        ] => $crate::shell::xdg::XdgShellState);
    };
}

impl Drop for XdgShellSurface {
    fn drop(&mut self) {
        // Surface role must be destroyed before the wl_surface
        self.xdg_surface.destroy();
    }
}

// Version 4 adds the configure_bounds event, which is a break
impl ProvidesBoundGlobal<xdg_wm_base::XdgWmBase, 4> for XdgShellState {
    fn bound_global(&self) -> Result<xdg_wm_base::XdgWmBase, GlobalError> {
        Ok(self.xdg_wm_base.clone())
    }
}

impl<D> Dispatch<xdg_wm_base::XdgWmBase, GlobalData, D> for XdgShellState
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
