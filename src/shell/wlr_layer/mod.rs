mod dispatch;

use std::{
    convert::TryFrom,
    sync::{Arc, Weak},
};

use bitflags::bitflags;
use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::{wl_output, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::xdg::shell::client::xdg_popup::XdgPopup;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::{compositor::Surface, globals::GlobalData};

use super::WaylandSurface;

#[derive(Debug)]
pub struct LayerShell {
    wlr_layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
}

impl LayerShell {
    /// Binds the wlr layer shell global, `zwlr_layer_shell_v1`.
    ///
    /// # Errors
    ///
    /// This function will return [`Err`] if the `zwlr_layer_shell_v1` global is not available.
    pub fn bind<State>(
        globals: &GlobalList,
        qh: &QueueHandle<State>,
    ) -> Result<LayerShell, BindError>
    where
        State: Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalData, State>
            + LayerShellHandler
            + 'static,
    {
        let wlr_layer_shell = globals.bind(qh, 1..=4, GlobalData)?;
        Ok(LayerShell { wlr_layer_shell })
    }

    #[must_use]
    pub fn create_layer_surface<State>(
        &self,
        qh: &QueueHandle<State>,
        surface: impl Into<Surface>,
        layer: Layer,
        namespace: Option<impl Into<String>>,
        output: Option<&wl_output::WlOutput>,
    ) -> LayerSurface
    where
        State: Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, LayerSurfaceData> + 'static,
    {
        // Freeze the queue during the creation of the Arc to avoid a race between events on the
        // new objects being processed and the Weak in the LayerSurfaceData becoming usable.
        let freeze = qh.freeze();
        let surface = surface.into();

        let inner = Arc::new_cyclic(|weak| {
            let layer_surface = self.wlr_layer_shell.get_layer_surface(
                surface.wl_surface(),
                output,
                layer.into(),
                namespace.map(Into::into).unwrap_or_default(),
                qh,
                LayerSurfaceData { inner: weak.clone() },
            );

            LayerSurfaceInner { wl_surface: surface, kind: SurfaceKind::Wlr(layer_surface) }
        });
        drop(freeze);

        LayerSurface(inner)
    }
}

/// Handler for operations on a [`LayerSurface`]
pub trait LayerShellHandler: Sized {
    /// The layer surface has been closed.
    ///
    /// When this requested is called, the layer surface is no longer shown and all handles of the [`LayerSurface`]
    /// should be dropped.
    fn closed(&mut self, conn: &Connection, qh: &QueueHandle<Self>, layer: &LayerSurface);

    /// Apply a suggested surface change.
    ///
    /// When this function is called, the compositor is requesting the layer surfaces's size or state to change.
    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        serial: u32,
    );
}

#[derive(Debug, Clone)]
pub struct LayerSurface(Arc<LayerSurfaceInner>);

impl PartialEq for LayerSurface {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl LayerSurface {
    pub fn from_wlr_surface(
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    ) -> Option<LayerSurface> {
        surface.data::<LayerSurfaceData>().and_then(|data| data.inner.upgrade()).map(LayerSurface)
    }

    pub fn get_popup(&self, popup: &XdgPopup) {
        match self.0.kind {
            SurfaceKind::Wlr(ref s) => s.get_popup(popup),
        }
    }

    // Double buffered state

    pub fn set_size(&self, width: u32, height: u32) {
        match self.0.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_size(width, height),
        }
    }

    pub fn set_anchor(&self, anchor: Anchor) {
        match self.0.kind {
            // We currently rely on the bitsets being the same
            SurfaceKind::Wlr(ref wlr) => {
                wlr.set_anchor(zwlr_layer_surface_v1::Anchor::from_bits_truncate(anchor.bits()))
            }
        }
    }

    pub fn set_exclusive_zone(&self, zone: i32) {
        match self.0.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_exclusive_zone(zone),
        }
    }

    pub fn set_margin(&self, top: i32, right: i32, bottom: i32, left: i32) {
        match self.0.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_margin(top, right, bottom, left),
        }
    }

    pub fn set_keyboard_interactivity(&self, value: KeyboardInteractivity) {
        match self.0.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_keyboard_interactivity(value.into()),
        }
    }

    pub fn set_layer(&self, layer: Layer) {
        match self.0.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_layer(layer.into()),
        }
    }

    pub fn kind(&self) -> &SurfaceKind {
        &self.0.kind
    }
}

impl WaylandSurface for LayerSurface {
    fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.0.wl_surface.wl_surface()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceKind {
    Wlr(zwlr_layer_surface_v1::ZwlrLayerSurfaceV1),
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyboardInteractivity {
    /// No keyboard focus is possible.
    ///
    /// This is the default value for all newly created layer shells.
    None,

    /// Request exclusive keyboard focus if the layer is above shell surfaces.
    ///
    /// For [`Layer::Top`] and [`Layer::Overlay`], the seat will always give exclusive access to the layer
    /// which has this interactivity mode set.
    ///
    /// This setting is intended for applications that need to ensure they receive all keyboard events, such
    /// as a lock screen or a password prompt.
    Exclusive,

    /// The compositor should focus and unfocus this surface by the user in an implementation specific manner.
    ///
    /// Compositors may use their normal mechanisms to manage keyboard focus between layers and regular
    /// desktop surfaces.
    ///
    /// This setting is intended for applications which allow keyboard interaction.  
    OnDemand,
}

impl Default for KeyboardInteractivity {
    fn default() -> Self {
        Self::None
    }
}

/// The z-depth of a layer.
///
/// These values indicate which order in which layer surfaces are rendered.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    Background,

    Bottom,

    Top,

    Overlay,
}

/// Error when converting a [`zwlr_layer_shell_v1::Layer`] to a [`Layer`]
#[derive(Debug, thiserror::Error)]
#[error("unknown layer")]
pub struct UnknownLayer;

bitflags! {
    /// Specifies which edges and corners a layer should be placed at in the anchor rectangle.
    ///
    /// A combination of two orthogonal edges will cause the layer's anchor point to be the intersection of
    /// the edges. For example [`Anchor::TOP`] and [`Anchor::LEFT`] will result in an anchor point in the top
    /// left of the anchor rectangle.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Anchor: u32 {
        /// Top edge of the anchor rectangle.
        const TOP = 1;

        /// The bottom edge of the anchor rectangle.
        const BOTTOM = 2;

        /// The left edge of the anchor rectangle.
        const LEFT = 4;

        /// The right edge of the anchor rectangle.
        const RIGHT = 8;
    }
}

/// A layer surface configure.
///
/// A configure describes a compositor request to resize the layer surface or change it's state.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct LayerSurfaceConfigure {
    /// The compositor suggested new size of the layer in surface-local coordinates.
    ///
    /// The size is a hint, meaning the new size can be ignored. A smaller size could be picked to satisfy
    /// some aspect ratio or resize in steps. If the size is smaller than suggested and the layer surface is
    /// anchored to two opposite anchors then the layer surface will be centered on that axis.
    ///
    /// If the width is zero, you may choose any width you want. If the height is zero, you may choose any
    /// height you want.
    pub new_size: (u32, u32),
}

#[derive(Debug)]
pub struct LayerSurfaceData {
    inner: Weak<LayerSurfaceInner>,
}

impl LayerSurfaceData {
    pub fn layer_surface(&self) -> Option<LayerSurface> {
        self.inner.upgrade().map(LayerSurface)
    }
}

#[macro_export]
macro_rules! delegate_layer {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1: $crate::globals::GlobalData
        ] => $crate::shell::wlr_layer::LayerShell);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1: $crate::shell::wlr_layer::LayerSurfaceData
        ] => $crate::shell::wlr_layer::LayerShell);
    };
}

#[derive(Debug)]
struct LayerSurfaceInner {
    wl_surface: Surface,
    kind: SurfaceKind,
}

impl TryFrom<zwlr_layer_shell_v1::Layer> for Layer {
    type Error = UnknownLayer;

    fn try_from(layer: zwlr_layer_shell_v1::Layer) -> Result<Self, Self::Error> {
        match layer {
            zwlr_layer_shell_v1::Layer::Background => Ok(Self::Background),
            zwlr_layer_shell_v1::Layer::Bottom => Ok(Self::Bottom),
            zwlr_layer_shell_v1::Layer::Top => Ok(Self::Top),
            zwlr_layer_shell_v1::Layer::Overlay => Ok(Self::Overlay),
            _ => Err(UnknownLayer),
        }
    }
}

impl From<Layer> for zwlr_layer_shell_v1::Layer {
    fn from(depth: Layer) -> Self {
        match depth {
            Layer::Background => Self::Background,
            Layer::Bottom => Self::Bottom,
            Layer::Top => Self::Top,
            Layer::Overlay => Self::Overlay,
        }
    }
}

impl From<KeyboardInteractivity> for zwlr_layer_surface_v1::KeyboardInteractivity {
    fn from(interactivity: KeyboardInteractivity) -> Self {
        match interactivity {
            KeyboardInteractivity::None => zwlr_layer_surface_v1::KeyboardInteractivity::None,
            KeyboardInteractivity::Exclusive => {
                zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive
            }
            KeyboardInteractivity::OnDemand => {
                zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand
            }
        }
    }
}

impl Drop for LayerSurfaceInner {
    fn drop(&mut self) {
        // Layer shell protocol dictates we must destroy the role object before the surface.
        match self.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.destroy(),
        }

        // Surface will destroy the wl_surface
        // self.wl_surface.destroy();
    }
}
