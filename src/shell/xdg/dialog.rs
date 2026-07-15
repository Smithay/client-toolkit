use crate::reexports::client::{protocol::wl_compositor::WlCompositor, Proxy, QueueHandle};
use crate::reexports::client::{protocol::wl_surface, Connection, Dispatch};
use crate::reexports::protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
use crate::shell::xdg::window::inner::{
    determine_decoration_mode, determine_window_state, determine_wm_capabilities, WindowInner,
};
use crate::shell::xdg::window::ToplevelDecorationData;
use crate::shell::xdg::window::WindowConfigure;
use crate::shell::xdg::WindowDecorations;
use crate::shell::xdg::WindowHandler;
use crate::shell::WaylandSurface;
use crate::{
    compositor::{CompositorHandler, Surface, SurfaceData},
    globals::ProvidesBoundGlobal,
    output::OutputHandler,
};
use crate::{error::GlobalError, shell::xdg::XdgShellSurface};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex, Weak};
use wayland_protocols::xdg::{
    decoration::zv1::client::zxdg_toplevel_decoration_v1,
    dialog::v1::client::xdg_dialog_v1::XdgDialogV1, shell::client::xdg_wm_base,
};
use wayland_protocols::xdg::{dialog::v1::client::xdg_dialog_v1, shell::client::xdg_surface};
use wayland_protocols::xdg::{dialog::v1::client::xdg_wm_dialog_v1, shell::client::xdg_toplevel};

/// Handler for toplevel operations on a [`Dialog`]
pub trait DialogHandler: Sized {
    /// Request to close a dialog.
    ///
    /// This request does not destroy the dialog. You must drop all [`Dialog`] handles to destroy the dialog.
    /// This request may be sent either by the compositor or by some other mechanism (such as client side decorations).
    fn request_close(&mut self, conn: &Connection, qh: &QueueHandle<Self>, window: &Dialog);

    /// Apply a suggested surface change.
    ///
    /// When this function is called, the compositor is requesting the window's size or state to change.
    ///
    /// Internally this function is called when the underlying `xdg_surface` is configured. Any extension
    /// protocols that interface with xdg-shell are able to be notified that the surface's configure sequence
    /// is complete by using this function.
    ///
    /// # Double buffering
    ///
    /// Configure events in Wayland are considered to be double buffered and the state of the window does not
    /// change until committed.
    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &Dialog,
        configure: WindowConfigure,
        serial: u32,
    );
}

#[derive(Debug, Clone)]
pub struct Dialog {
    inner: Arc<DialogInner>,
}

#[derive(Debug)]
pub struct DialogData(pub(crate) Weak<DialogInner>);

#[derive(Debug)]
pub(crate) struct DialogInner {
    pub xdg_dialog: XdgDialogV1,
    pub window: WindowInner,
}

impl Dialog {
    pub fn new<D, GLOBAL>(
        parent: &xdg_toplevel::XdgToplevel,
        qh: &QueueHandle<D>,
        // TODO: is 6 correct?
        compositor: &impl ProvidesBoundGlobal<WlCompositor, 6>,
        wm: &GLOBAL,
        decoration_manager: Option<&ZxdgDecorationManagerV1>,
        decorations: WindowDecorations,
    ) -> Result<Self, GlobalError>
    where
        D: CompositorHandler + DialogHandler + OutputHandler + WindowHandler + 'static,
        GLOBAL: ProvidesBoundGlobal<xdg_wm_dialog_v1::XdgWmDialogV1, 1>
            + ProvidesBoundGlobal<xdg_wm_base::XdgWmBase, 5>,
    {
        let surface = Surface::new(compositor, qh)?;
        let dialog = Self::from_surface(surface, parent, qh, wm, decoration_manager, decorations)?;
        dialog.wl_surface().commit();
        Ok(dialog)
    }

    pub fn from_surface<D, GLOBAL>(
        surface: impl Into<Surface>,
        parent: &xdg_toplevel::XdgToplevel,
        qh: &QueueHandle<D>,
        wm_base: &GLOBAL,
        decoration_manager: Option<&ZxdgDecorationManagerV1>,
        decorations: WindowDecorations,
    ) -> Result<Self, GlobalError>
    where
        D: DialogHandler + WindowHandler + 'static,
        GLOBAL: ProvidesBoundGlobal<xdg_wm_dialog_v1::XdgWmDialogV1, 1>
            + ProvidesBoundGlobal<xdg_wm_base::XdgWmBase, 5>,
    {
        let surface = surface.into();
        let wm_dialog: xdg_wm_dialog_v1::XdgWmDialogV1 = wm_base.bound_global()?;
        let wm_base: xdg_wm_base::XdgWmBase = wm_base.bound_global()?;

        // Freeze the queue during the creation of the Arc to avoid a race between events on the
        // new objects being processed and the Weak in the DialogData becoming usable.
        let freeze = qh.freeze();

        let inner = Arc::new_cyclic(|weak| {
            let xdg_surface =
                wm_base.get_xdg_surface(surface.wl_surface(), qh, DialogData(weak.clone()));
            let surface = XdgShellSurface { surface, xdg_surface };
            let xdg_toplevel = surface.xdg_surface.get_toplevel(qh, DialogData(weak.clone()));
            xdg_toplevel.set_parent(Some(parent));
            let xdg_dialog = wm_dialog.get_xdg_dialog(&xdg_toplevel, qh, DialogData(weak.clone()));

            let toplevel_decoration = crate::shell::xdg::XdgShell::toplevel_decoration(
                decoration_manager,
                &xdg_toplevel,
                decorations,
                DialogData(weak.clone()),
                qh,
            );

            DialogInner {
                xdg_dialog,
                window: WindowInner {
                    xdg_surface: surface,
                    xdg_toplevel,
                    toplevel_decoration,
                    pending_configure: Mutex::new(Default::default()),
                },
            }
        });
        drop(freeze);
        let dialog = Dialog { inner };
        Ok(dialog)
    }

    pub fn from_xdg_toplevel(toplevel: &xdg_toplevel::XdgToplevel) -> Option<Dialog> {
        toplevel.data::<DialogData>().and_then(|data| data.dialog())
    }

    pub fn from_xdg_surface(surface: &xdg_surface::XdgSurface) -> Option<Dialog> {
        surface.data::<DialogData>().and_then(|data| data.dialog())
    }

    pub fn from_toplevel_decoration(
        decoration: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
    ) -> Option<Dialog> {
        decoration.data::<ToplevelDecorationData<DialogData>>().and_then(|data| data.0.dialog())
    }

    pub fn xdg_dialog(&self) -> &XdgDialogV1 {
        &self.inner.xdg_dialog
    }

    pub fn xdg_shell_surface(&self) -> &XdgShellSurface {
        &self.inner.window.xdg_surface
    }

    pub fn xdg_toplevel(&self) -> &xdg_toplevel::XdgToplevel {
        &self.inner.window.xdg_toplevel
    }

    pub fn xdg_surface(&self) -> &xdg_surface::XdgSurface {
        self.inner.window.xdg_surface.xdg_surface()
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.inner.window.xdg_surface.wl_surface()
    }

    pub fn set_modal(&self, modal: bool) {
        if modal {
            self.inner.xdg_dialog.set_modal();
        } else {
            self.inner.xdg_dialog.unset_modal();
        }
    }
}

impl WaylandSurface for Dialog {
    fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.wl_surface()
    }
}

impl PartialEq for Dialog {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl DialogData {
    /// Get a new handle to the Dialog
    ///
    /// This returns `None` if the dialog has been destroyed.
    pub fn dialog(&self) -> Option<Dialog> {
        let inner = self.0.upgrade()?;
        Some(Dialog { inner })
    }
}

impl Drop for DialogInner {
    fn drop(&mut self) {
        self.xdg_dialog.destroy();
    }
}

impl<D: DialogHandler> Dispatch<xdg_surface::XdgSurface, D> for DialogData {
    fn event(
        &self,
        data: &mut D,
        xdg_surface: &xdg_surface::XdgSurface,
        event: <xdg_surface::XdgSurface as wayland_client::Proxy>::Event,
        conn: &Connection,
        qhandle: &QueueHandle<D>,
    ) {
        if let Some(dialog) = Dialog::from_xdg_surface(xdg_surface) {
            match event {
                xdg_surface::Event::Configure { serial } => {
                    xdg_surface.ack_configure(serial);

                    let configure = dialog.inner.window.pending_configure.lock().unwrap().clone();
                    DialogHandler::configure(data, conn, qhandle, &dialog, configure, serial)
                }
                _ => unreachable!(),
            }
        }
    }
}

impl<D> Dispatch<XdgDialogV1, D> for DialogData {
    fn event(
        &self,
        _state: &mut D,
        _proxy: &XdgDialogV1,
        _event: <XdgDialogV1 as wayland_client::Proxy>::Event,
        _conn: &Connection,
        _qhandle: &QueueHandle<D>,
    ) {
    }
}

impl<D: DialogHandler> Dispatch<xdg_toplevel::XdgToplevel, D> for DialogData {
    fn event(
        &self,
        data: &mut D,
        toplevel: &xdg_toplevel::XdgToplevel,
        event: <xdg_toplevel::XdgToplevel as wayland_client::Proxy>::Event,
        conn: &Connection,
        qhandle: &QueueHandle<D>,
    ) {
        let Some(dialog) = Dialog::from_xdg_toplevel(toplevel) else {
            return;
        };

        match event {
            xdg_toplevel::Event::Configure { width, height, states } => {
                let new_state = determine_window_state(&states);

                // XXX we do explicit convertion and sanity checking because compositor
                // could pass negative values which we should ignore all together.
                let width = u32::try_from(width).ok().and_then(NonZeroU32::new);
                let height = u32::try_from(height).ok().and_then(NonZeroU32::new);

                let pending_configure = &mut dialog.inner.window.pending_configure.lock().unwrap();
                pending_configure.new_size = (width, height);
                pending_configure.state = new_state;
            }
            xdg_toplevel::Event::Close => {
                data.request_close(conn, qhandle, &dialog);
            }

            xdg_toplevel::Event::ConfigureBounds { width, height } => {
                let pending_configure = &mut dialog.inner.window.pending_configure.lock().unwrap();
                if width == 0 && height == 0 {
                    pending_configure.suggested_bounds = None;
                } else {
                    pending_configure.suggested_bounds = Some((width as u32, height as u32));
                }
            }
            xdg_toplevel::Event::WmCapabilities { capabilities } => {
                let pending_configure = &mut dialog.inner.window.pending_configure.lock().unwrap();
                pending_configure.capabilities = determine_wm_capabilities(&capabilities)
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, D> for DialogData
where
    D: DialogHandler,
{
    fn event(
        &self,
        _: &mut D,
        decoration: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        if let Some(dialog) = Dialog::from_toplevel_decoration(decoration) {
            match event {
                zxdg_toplevel_decoration_v1::Event::Configure { mode } => {
                    if mode.available_since().is_some_and(|v| v <= decoration.version()) {
                        let mode = determine_decoration_mode(mode);
                        dialog.inner.window.pending_configure.lock().unwrap().decoration_mode =
                            mode;
                    } else {
                        log::error!(target: "sctk", "unknown decoration mode 0x{:?}", mode);
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
