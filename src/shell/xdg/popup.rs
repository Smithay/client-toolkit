use crate::{
    compositor::{Surface, SurfaceData},
    error::GlobalError,
    globals::ProvidesBoundGlobal,
    shell::xdg::XdgShellSurface,
};
use std::sync::{
    atomic::{AtomicI32, AtomicU32, Ordering::Relaxed},
    Arc, Weak,
};
use wayland_client::{
    protocol::{wl_compositor::WlCompositor, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::shell::client::{xdg_popup, xdg_positioner, xdg_surface, xdg_wm_base};

#[derive(Debug, Clone)]
pub struct Popup {
    inner: Arc<PopupInner>,
}

impl Eq for Popup {}
impl PartialEq for Popup {
    fn eq(&self, other: &Popup) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

#[derive(Debug)]
pub struct PopupData {
    inner: Weak<PopupInner>,
}

#[derive(Debug)]
struct PopupInner {
    surface: XdgShellSurface,
    xdg_popup: xdg_popup::XdgPopup,
    pending_position: (AtomicI32, AtomicI32),
    pending_dimensions: (AtomicI32, AtomicI32),
    pending_token: AtomicU32,
    configure_state: AtomicU32,
}

impl Popup {
    /// Create a new popup.
    ///
    /// This creates the popup and sends the initial commit.  You must wait for
    /// [`PopupHandler::configure`] to commit contents to the surface.
    pub fn new<D>(
        parent: &xdg_surface::XdgSurface,
        position: &xdg_positioner::XdgPositioner,
        qh: &QueueHandle<D>,
        compositor: &impl ProvidesBoundGlobal<WlCompositor, 6>,
        wm_base: &impl ProvidesBoundGlobal<xdg_wm_base::XdgWmBase, 5>,
    ) -> Result<Popup, GlobalError>
    where
        D: Dispatch<wl_surface::WlSurface, SurfaceData>
            + Dispatch<xdg_surface::XdgSurface, PopupData>
            + Dispatch<xdg_popup::XdgPopup, PopupData>
            + PopupHandler
            + 'static,
    {
        let surface = Surface::new(compositor, qh)?;
        let popup = Self::from_surface(Some(parent), position, qh, surface, wm_base)?;
        popup.wl_surface().commit();
        Ok(popup)
    }

    /// Create a new popup from an existing surface.
    ///
    /// If you do not specify a parent surface, you must configure the parent using an alternate
    /// function such as [`LayerSurface::get_popup`] prior to committing the surface, or you will
    /// get an `invalid_popup_parent` protocol error.
    ///
    /// [`LayerSurface::get_popup`]: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1::get_popup
    pub fn from_surface<D>(
        parent: Option<&xdg_surface::XdgSurface>,
        position: &xdg_positioner::XdgPositioner,
        qh: &QueueHandle<D>,
        surface: impl Into<Surface>,
        wm_base: &impl ProvidesBoundGlobal<xdg_wm_base::XdgWmBase, 5>,
    ) -> Result<Popup, GlobalError>
    where
        D: Dispatch<xdg_surface::XdgSurface, PopupData>
            + Dispatch<xdg_popup::XdgPopup, PopupData>
            + 'static,
    {
        let surface = surface.into();
        let wm_base = wm_base.bound_global()?;
        // Freeze the queue during the creation of the Arc to avoid a race between events on the
        // new objects being processed and the Weak in the PopupData becoming usable.
        let freeze = qh.freeze();
        let inner = Arc::new_cyclic(|weak| {
            let xdg_surface = wm_base.get_xdg_surface(
                surface.wl_surface(),
                qh,
                PopupData { inner: weak.clone() },
            );
            let surface = XdgShellSurface { surface, xdg_surface };
            let xdg_popup = surface.xdg_surface().get_popup(
                parent,
                position,
                qh,
                PopupData { inner: weak.clone() },
            );

            PopupInner {
                surface,
                xdg_popup,
                pending_position: (AtomicI32::new(0), AtomicI32::new(0)),
                pending_dimensions: (AtomicI32::new(-1), AtomicI32::new(-1)),
                pending_token: AtomicU32::new(0),
                configure_state: AtomicU32::new(PopupConfigure::STATE_NEW),
            }
        });
        drop(freeze);
        Ok(Popup { inner })
    }

    pub fn xdg_popup(&self) -> &xdg_popup::XdgPopup {
        &self.inner.xdg_popup
    }

    pub fn xdg_shell_surface(&self) -> &XdgShellSurface {
        &self.inner.surface
    }

    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        self.inner.surface.xdg_surface()
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.inner.surface.wl_surface()
    }

    pub fn reposition(&self, position: &xdg_positioner::XdgPositioner, token: u32) {
        self.xdg_popup().reposition(position, token);
    }
}

impl PopupData {
    /// Get a new handle to the Popup
    ///
    /// This returns `None` if the popup has been destroyed.
    pub fn popup(&self) -> Option<Popup> {
        let inner = self.inner.upgrade()?;
        Some(Popup { inner })
    }
}

impl Drop for PopupInner {
    fn drop(&mut self) {
        self.xdg_popup.destroy();
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PopupConfigure {
    /// (x,y) relative to parent surface window geometry
    pub position: (i32, i32),
    pub width: i32,
    pub height: i32,
    pub serial: u32,
    pub kind: ConfigureKind,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ConfigureKind {
    /// Initial configure for this popup
    Initial,
    /// The configure is due to an xdg_positioner with set_reactive requested
    Reactive,
    /// The configure is due to a reposition request with this token
    Reposition { token: u32 },
}

impl PopupConfigure {
    const STATE_NEW: u32 = 0;
    const STATE_CONFIGURED: u32 = 1;
    const STATE_REPOSITION_ACK: u32 = 2;
}

pub trait PopupHandler: Sized {
    /// The popup has been configured.
    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        popup: &Popup,
        config: PopupConfigure,
    );

    /// The popup was dismissed by the compositor and should be destroyed.
    fn done(&mut self, conn: &Connection, qh: &QueueHandle<Self>, popup: &Popup);
}

impl<D> Dispatch<xdg_surface::XdgSurface, PopupData, D> for PopupData
where
    D: Dispatch<xdg_surface::XdgSurface, PopupData> + PopupHandler,
{
    fn event(
        data: &mut D,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        pdata: &PopupData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let popup = match pdata.popup() {
            Some(popup) => popup,
            None => return,
        };
        let inner = &popup.inner;
        match event {
            xdg_surface::Event::Configure { serial } => {
                xdg_surface.ack_configure(serial);
                let x = inner.pending_position.0.load(Relaxed);
                let y = inner.pending_position.1.load(Relaxed);
                let width = inner.pending_dimensions.0.load(Relaxed);
                let height = inner.pending_dimensions.1.load(Relaxed);
                let kind =
                    match inner.configure_state.swap(PopupConfigure::STATE_CONFIGURED, Relaxed) {
                        PopupConfigure::STATE_NEW => ConfigureKind::Initial,
                        PopupConfigure::STATE_CONFIGURED => ConfigureKind::Reactive,
                        PopupConfigure::STATE_REPOSITION_ACK => {
                            ConfigureKind::Reposition { token: inner.pending_token.load(Relaxed) }
                        }
                        _ => unreachable!(),
                    };

                let config = PopupConfigure { position: (x, y), width, height, serial, kind };

                data.configure(conn, qh, &popup, config);
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<xdg_popup::XdgPopup, PopupData, D> for PopupData
where
    D: Dispatch<xdg_popup::XdgPopup, PopupData> + PopupHandler,
{
    fn event(
        data: &mut D,
        _: &xdg_popup::XdgPopup,
        event: xdg_popup::Event,
        pdata: &PopupData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let popup = match pdata.popup() {
            Some(popup) => popup,
            None => return,
        };
        let inner = &popup.inner;
        match event {
            xdg_popup::Event::Configure { x, y, width, height } => {
                inner.pending_position.0.store(x, Relaxed);
                inner.pending_position.1.store(y, Relaxed);
                inner.pending_dimensions.0.store(width, Relaxed);
                inner.pending_dimensions.1.store(height, Relaxed);
            }
            xdg_popup::Event::PopupDone => {
                data.done(conn, qh, &popup);
            }
            xdg_popup::Event::Repositioned { token } => {
                inner.pending_token.store(token, Relaxed);
                inner.configure_state.store(PopupConfigure::STATE_REPOSITION_ACK, Relaxed);
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_xdg_popup {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::xdg::shell::client::xdg_popup::XdgPopup: $crate::shell::xdg::popup::PopupData
        ] => $crate::shell::xdg::popup::PopupData);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::xdg::shell::client::xdg_surface::XdgSurface: $crate::shell::xdg::popup::PopupData
        ] => $crate::shell::xdg::popup::PopupData);
    };
}
