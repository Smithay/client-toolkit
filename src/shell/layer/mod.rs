mod dispatch;

use std::{
    convert::TryFrom,
    sync::{Arc, Weak},
};

use bitflags::bitflags;
use wayland_client::{
    protocol::{wl_output, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::registry::GlobalProxy;
use crate::{error::GlobalError, globals::ProvidesBoundGlobal};

#[derive(Debug)]
pub struct LayerState {
    wlr_layer_shell: GlobalProxy<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
}

impl LayerState {
    pub fn new() -> LayerState {
        LayerState { wlr_layer_shell: GlobalProxy::NotReady }
    }

    /// Returns whether the layer shell is available.
    ///
    /// The layer shell is not supported by all compositors and this function may be used to determine if
    /// compositor support is available.
    pub fn is_available(&self) -> bool {
        self.wlr_layer_shell.get().is_ok()
    }
}

pub trait LayerHandler: Sized {
    fn layer_state(&mut self) -> &mut LayerState;

    /// Called when the surface will no longer be shown.
    ///
    /// This may occur as a result of the output the layer is placed on being destroyed or the user has caused
    /// the layer to be removed.
    ///
    /// You should drop the layer you have when this event is received.
    fn closed(&mut self, conn: &Connection, qh: &QueueHandle<Self>, layer: &LayerSurface);

    /// Called when the compositor has sent a configure event to an layer
    ///
    /// A configure atomically indicates that a sequence of events describing how a surface has changed have
    /// all been sent.
    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        serial: u32,
    );
}

#[derive(Debug)]
pub struct LayerSurfaceBuilder {
    output: Option<wl_output::WlOutput>,
    namespace: Option<String>,
    size: Option<(u32, u32)>,
    anchor: Option<Anchor>,
    zone: Option<i32>,
    // top, right, bottom, left
    margin: Option<(i32, i32, i32, i32)>,
    interactivity: Option<KeyboardInteractivity>,
}

impl LayerSurfaceBuilder {
    pub fn namespace(self, namespace: impl Into<String>) -> Self {
        Self { namespace: Some(namespace.into()), ..self }
    }

    pub fn output(self, output: &wl_output::WlOutput) -> Self {
        Self { output: Some(output.clone()), ..self }
    }

    pub fn size(self, size: (u32, u32)) -> Self {
        Self { size: Some(size), ..self }
    }

    pub fn anchor(self, anchor: Anchor) -> Self {
        Self { anchor: Some(anchor), ..self }
    }

    pub fn exclusive_zone(self, zone: i32) -> Self {
        Self { zone: Some(zone), ..self }
    }

    pub fn margin(self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        Self { margin: Some((top, right, bottom, left)), ..self }
    }

    pub fn keyboard_interactivity(self, interactivity: KeyboardInteractivity) -> Self {
        Self { interactivity: Some(interactivity), ..self }
    }

    /// Build and map the layer
    ///
    /// This function will create the layer and send the initial commit.
    ///
    /// # Protocol errors
    ///
    /// If the surface already has a role object, the compositor will raise a protocol error.
    ///
    /// A surface is considered to have a role object if some other type of surface was created using the
    /// surface. For example, creating a window, popup, layer or subsurface all assign a role object to a
    /// surface.
    ///
    /// The function here takes an owned reference to the surface to hint the surface will be consumed by the
    /// layer.
    ///
    /// [`WlSurface`]: wl_surface::WlSurface
    #[must_use = "The layer is destroyed if dropped"]
    pub fn map<D>(
        self,
        qh: &QueueHandle<D>,
        shell: &impl ProvidesBoundGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1, 4>,
        surface: wl_surface::WlSurface,
        layer: Layer,
    ) -> Result<LayerSurface, GlobalError>
    where
        D: Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, LayerSurfaceData> + 'static,
    {
        // The layer is required in ext-layer-shell-v1 but is not part of the factory request. So the param
        // will stay for ext-layer-shell-v1 support.

        // We really need an Arc::try_new_cyclic function to handle errors during creation.
        // Emulate that function by creating an Arc containing None in the error case, and use the
        // closure's context to pass the real error back to the caller so the invalid Arc is never
        // returned.
        let mut err = Ok(());
        let inner = Arc::new_cyclic(|weak| {
            let wlr_layer_shell = shell.bound_global().map_err(|e| err = Err(e)).ok()?;

            let layer_surface = wlr_layer_shell
                .get_layer_surface(
                    &surface,
                    self.output.as_ref(),
                    layer.into(),
                    self.namespace.unwrap_or_default(),
                    qh,
                    LayerSurfaceData { inner: weak.clone() },
                )
                .map_err(|e| err = Err(e.into()))
                .ok()?;

            Some(LayerSurfaceInner {
                wl_surface: surface.clone(),
                kind: SurfaceKind::Wlr(layer_surface),
            })
        });

        // This assert checks that err was set properly above; it should be impossible to trigger,
        // so it's not a run-time assert.
        debug_assert!(inner.is_some() || err.is_err());

        let layer_surface = err.map(|()| LayerSurface(inner))?;

        // Set data for initial commit
        if let Some(size) = self.size {
            layer_surface.set_size(size.0, size.1);
        }

        if let Some(anchor) = self.anchor {
            // We currently rely on the bitsets matching
            layer_surface.set_anchor(anchor);
        }

        if let Some(zone) = self.zone {
            layer_surface.set_exclusive_zone(zone);
        }

        if let Some(margin) = self.margin {
            layer_surface.set_margin(margin.0, margin.1, margin.2, margin.3);
        }

        if let Some(interactivity) = self.interactivity {
            layer_surface.set_keyboard_interactivity(interactivity);
        }

        // Initial commit
        surface.commit();

        Ok(layer_surface)
    }
}

#[derive(Debug, Clone)]
pub struct LayerSurface(Arc<Option<LayerSurfaceInner>>);

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

    // TODO(from_wl_surface): This will require us to initialize the surface.

    pub fn builder() -> LayerSurfaceBuilder {
        LayerSurfaceBuilder {
            output: None,
            namespace: None,
            size: None,
            anchor: None,
            zone: None,
            margin: None,
            interactivity: None,
        }
    }

    // TODO: get_popup

    // Double buffered state

    pub fn set_size(&self, width: u32, height: u32) {
        match self.inner().kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_size(width, height),
        }
    }

    pub fn set_anchor(&self, anchor: Anchor) {
        match self.inner().kind {
            // We currently rely on the bitsets being the same
            SurfaceKind::Wlr(ref wlr) => {
                wlr.set_anchor(zwlr_layer_surface_v1::Anchor::from_bits_truncate(anchor.bits()))
            }
        }
    }

    pub fn set_exclusive_zone(&self, zone: i32) {
        match self.inner().kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_exclusive_zone(zone),
        }
    }

    pub fn set_margin(&self, top: i32, right: i32, bottom: i32, left: i32) {
        match self.inner().kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_margin(top, right, bottom, left),
        }
    }

    pub fn set_keyboard_interactivity(&self, value: KeyboardInteractivity) {
        match self.inner().kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_keyboard_interactivity(value.into()),
        }
    }

    pub fn set_layer(&self, layer: Layer) {
        match self.inner().kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_layer(layer.into()),
        }
    }

    pub fn kind(&self) -> &SurfaceKind {
        &self.inner().kind
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.inner().wl_surface
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
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

/// The configure state of a layer
///
/// This type indicates compositor changes to the layer, such as a new size.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct LayerSurfaceConfigure {
    /// The compositor suggested new size of the layer.
    ///
    /// The size is a hint, meaning the client is free to ignore the new size (if the client does not resize),
    /// pick a smaller size to satisfy aspect ratio or resize in steps. If you pick a small size and the
    /// surface is anchored to two opposite anchors, then surface will be centered on the axis.
    ///
    /// If either the width or height is 0, the compositor may choose any size for that specific width or height.
    pub new_size: (u32, u32),
}

#[derive(Debug)]
pub struct LayerSurfaceData {
    inner: Weak<Option<LayerSurfaceInner>>,
}

impl LayerSurfaceData {
    pub fn layer_surface(&self) -> Option<LayerSurface> {
        self.inner.upgrade().map(LayerSurface)
    }
}

#[macro_export]
macro_rules! delegate_layer {
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty: [
            $crate::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1: $crate::globals::GlobalData,
            $crate::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1: $crate::shell::layer::LayerSurfaceData,
        ] => $crate::shell::layer::LayerState);
    };
}

impl LayerSurface {
    fn inner(&self) -> &LayerSurfaceInner {
        Option::as_ref(&self.0).expect("The contents of an initialized LayerSurface cannot be None")
    }
}

#[derive(Debug)]
struct LayerSurfaceInner {
    wl_surface: wl_surface::WlSurface,
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

        self.wl_surface.destroy();
    }
}
