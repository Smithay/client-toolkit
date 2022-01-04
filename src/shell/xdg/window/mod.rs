use std::sync::{atomic::Ordering, Arc};

use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_output, wl_surface},
    ConnectionHandle, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_toplevel_decoration_v1,
    xdg_shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
    },
};

use crate::compositor::SurfaceData;

use self::inner::WindowInner;

use super::{XdgShellHandler, XdgShellState, XdgSurfaceData};

pub(super) mod inner;

pub trait WindowHandler: XdgShellHandler + Sized {
    fn configure_window(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        new_size: Option<(u32, u32)>,
        states: Vec<State>,
        window: &Window,
    );

    fn request_close_window(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        window: &Window,
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationMode {
    PreferServer,

    ServerOnly,

    ClientOnly,

    None,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateWindowError {
    /// Surface already has a role object.
    #[error("surface already has a role object")]
    HasRole,

    /// The xdg_wm_base global is not available.
    #[error("the xdg_wm_base global is not available")]
    MissingRequiredGlobals,

    /// Protocol error.
    #[error(transparent)]
    Protocol(#[from] InvalidId),
}

#[derive(Debug)]
pub struct Window(pub(crate) Arc<WindowInner>);

impl Window {
    /// Map the window.
    ///
    /// This function will commit the initial window state and will result in the initial configure at some
    /// point.
    ///
    /// ## Protocol errors
    ///
    /// The [`WlSurface`] may not have any buffers attached. If a buffer is attached, a protocol error will
    /// occur.
    ///
    /// [`WlSurface`]: wl_surface::WlSurface
    pub fn map<D>(&self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>)
    where
        D: Dispatch<
                zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
                UserData = XdgSurfaceData,
            > + 'static,
    {
        self.0.map(conn, qh)
    }

    pub fn set_title(&self, conn: &mut ConnectionHandle, title: impl Into<String>) {
        self.0.set_title(conn, title.into())
    }

    pub fn set_app_id(&self, conn: &mut ConnectionHandle, app_id: impl Into<String>) {
        self.0.set_app_id(conn, app_id.into())
    }

    pub fn set_min_size(&self, conn: &mut ConnectionHandle, min_size: Option<(u32, u32)>) {
        self.0.set_min_size(conn, min_size)
    }

    /// ## Protocol errors
    ///
    /// The maximum size of the window may not be smaller than the minimum size.
    pub fn set_max_size(&self, conn: &mut ConnectionHandle, max_size: Option<(u32, u32)>) {
        self.0.set_max_size(conn, max_size)
    }

    // TODO: Change decoration mode

    pub fn set_parent(&self, conn: &mut ConnectionHandle, parent: Option<&Window>) {
        self.0.set_parent(conn, parent)
    }

    // TODO: Show window menu

    // TODO: Move

    // TODO: Resize

    pub fn set_maximized(&self, conn: &mut ConnectionHandle) {
        self.0.set_maximized(conn)
    }

    pub fn unset_maximized(&self, conn: &mut ConnectionHandle) {
        self.0.unset_maximized(conn)
    }

    pub fn set_mimimized(&self, conn: &mut ConnectionHandle) {
        self.0.set_minmized(conn)
    }

    pub fn set_fullscreen(
        &self,
        conn: &mut ConnectionHandle,
        output: Option<&wl_output::WlOutput>,
    ) {
        self.0.set_fullscreen(conn, output)
    }

    pub fn unset_fullscreen(&self, conn: &mut ConnectionHandle) {
        self.0.unset_fullscreen(conn)
    }

    /// Returns the surface wrapped in this window.
    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.0.wl_surface
    }

    /// Returns the xdg surface wrapped in this window.
    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        &self.0.xdg_surface
    }

    /// Returns the xdg toplevel wrapped in this window.
    pub fn xdg_toplevel(&self) -> &xdg_toplevel::XdgToplevel {
        &self.0.xdg_toplevel
    }
}

impl XdgShellState {
    /// Create a window.
    ///
    /// This function will create a window from a [`WlSurface`].
    ///
    /// ## Default settings
    ///
    /// ### Window decorations
    ///
    /// The window will use the decoration mode dictated by the compositor. If you do not want this, set the
    /// preferred decoration mode before [`map()`](Window::map)ing the window.
    ///
    /// ### Initial window size
    ///
    /// This will vary depending on the compositor. Some compositors may allow the you to make the window
    /// any desired size while others may give the you a desired size during the initial commit.
    ///
    /// You ultimately have control over what size buffer is committed, meaning you could ignore the
    /// compositor. However, not respecting the compositor will likely result in aggravated users and a subpar
    /// experience.
    ///
    /// Some compositors may take the minimum and maximum window size in consideration when determining how
    /// large of a window that will be requested during the initial commit.
    ///
    /// [`WlSurface`]: wl_surface::WlSurface
    #[must_use = "You must map() the window before presenting to it"]
    pub fn create_window<D>(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        wl_surface: wl_surface::WlSurface,
    ) -> Result<Window, CreateWindowError>
    where
        D: Dispatch<wl_surface::WlSurface, UserData = SurfaceData>
            + Dispatch<xdg_surface::XdgSurface, UserData = XdgSurfaceData>
            + Dispatch<xdg_toplevel::XdgToplevel, UserData = XdgSurfaceData>
            + 'static,
    {
        // We don't know if an un-managed surface has a role object.
        let surface_data = wl_surface.data::<SurfaceData>().ok_or(CreateWindowError::HasRole)?;

        // XDG Shell protocol forbids creating an window from a surface that already has a role object.
        if surface_data.has_role.load(Ordering::SeqCst) {
            return Err(CreateWindowError::HasRole);
        }

        let (_, xdg_wm_base) =
            self.xdg_wm_base.as_ref().ok_or(CreateWindowError::MissingRequiredGlobals)?;
        let zxdg_decoration_manager =
            self.zxdg_decoration_manager_v1.clone().map(|(_, global)| global);

        let xdg_surface_data = XdgSurfaceData::uninit();
        let xdg_surface =
            xdg_wm_base.get_xdg_surface(conn, &wl_surface, qh, xdg_surface_data.clone())?;
        let xdg_toplevel = xdg_surface.get_toplevel(conn, qh, xdg_surface_data.clone())?;

        // Mark the surface has having a role object.
        let surface_data = wl_surface.data::<SurfaceData>().unwrap();
        surface_data.has_role.store(true, Ordering::SeqCst);

        let inner = xdg_surface_data.init_window(
            self,
            wl_surface,
            xdg_surface,
            xdg_toplevel,
            zxdg_decoration_manager,
        );

        Ok(Window(inner))
    }
}
