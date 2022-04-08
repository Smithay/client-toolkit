mod dispatch;

use std::{
    convert::TryFrom,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use bitflags::bitflags;
use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_output, wl_surface},
    ConnectionHandle, Dispatch, QueueHandle,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

#[derive(Debug)]
pub struct LayerState {
    wlr_layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    surfaces: Vec<LayerSurface>,
}

impl LayerState {
    pub fn new() -> LayerState {
        LayerState { wlr_layer_shell: None, surfaces: Vec::new() }
    }

    /// Returns whether the layer shell is available.
    ///
    /// The layer shell is not supported by all compositors and this function may be used to determine if
    /// compositor support is available.
    pub fn is_available(&self) -> bool {
        self.wlr_layer_shell.is_some()
    }

    pub fn wlr_layer_shell(&self) -> Option<&zwlr_layer_shell_v1::ZwlrLayerShellV1> {
        self.wlr_layer_shell.as_ref()
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
    fn closed(&mut self, conn: &mut ConnectionHandle, qh: &QueueHandle<Self>, layer: &LayerSurface);

    /// Called when the compositor has sent a configure event to an layer
    ///
    /// A configure atomically indicates that a sequence of events describing how a surface has changed have
    /// all been sent.
    fn configure(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        serial: u32,
    );
}

#[derive(Debug, thiserror::Error)]
pub enum LayerSurfaceError {
    /// The layer shell global is not available.
    #[error("the layer shell global is not available")]
    MissingRequiredGlobals,

    /// Protocol error.
    #[error(transparent)]
    Protocol(#[from] InvalidId),
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
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        layer_state: &mut LayerState,
        surface: wl_surface::WlSurface,
        layer: Layer,
    ) -> Result<LayerSurface, LayerSurfaceError>
    where
        D: Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, UserData = LayerSurfaceData>
            + 'static,
    {
        // The layer is required in ext-layer-shell-v1 but is not part of the factory request. So the param
        // will stay for ext-layer-shell-v1 support.

        let layer_shell =
            layer_state.wlr_layer_shell().ok_or(LayerSurfaceError::MissingRequiredGlobals)?;
        let layer_surface = layer_shell.get_layer_surface(
            conn,
            &surface,
            self.output.as_ref(),
            layer.into(),
            self.namespace.unwrap_or_default(),
            qh,
            LayerSurfaceData {},
        )?;

        // Set data for initial commit
        if let Some(size) = self.size {
            layer_surface.set_size(conn, size.0, size.1);
        }

        if let Some(anchor) = self.anchor {
            // We currently rely on the bitsets matching
            layer_surface
                .set_anchor(conn, zwlr_layer_surface_v1::Anchor::from_bits_truncate(anchor.bits()));
        }

        if let Some(zone) = self.zone {
            layer_surface.set_exclusive_zone(conn, zone);
        }

        if let Some(margin) = self.margin {
            layer_surface.set_margin(conn, margin.0, margin.1, margin.2, margin.3);
        }

        if let Some(interactivity) = self.interactivity {
            layer_surface.set_keyboard_interactivity(conn, interactivity.into())
        }

        // Initial commit
        surface.commit(conn);

        let layer_surface = LayerSurface {
            kind: SurfaceKind::Wlr(layer_surface),
            wl_surface: surface,
            primary: true,
            death_signal: Arc::new(AtomicBool::new(false)),
        };

        layer_state.surfaces.push(layer_surface.impl_clone());

        Ok(layer_surface)
    }
}

#[derive(Debug)]
pub struct LayerSurface {
    kind: SurfaceKind,

    wl_surface: wl_surface::WlSurface,

    /// Whether this is the primary handle to the layer.
    ///
    /// This is only true for [`Layer`] given the user from [`LayerBuilder::map`]. Since we pass
    /// a reference to a [`Layer`] in some traits the user implements, we need to make sure the layer isn't
    /// actually destroyed while the user still holds the layer. If this field is true, the drop implementation
    /// will mark the layer as dead and will clean up when possible.
    pub(crate) primary: bool,

    /// Indicates whether the primary handle to the layer has been destroyed.
    ///
    /// Since we can't destroy wayland objects without a connection handle, we need to mark the layer for
    /// cleanup.
    pub(crate) death_signal: Arc<AtomicBool>,
}

impl PartialEq for LayerSurface {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl LayerSurface {
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

    pub fn set_size(&self, conn: &mut ConnectionHandle, width: u32, height: u32) {
        match self.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_size(conn, width, height),
        }
    }

    pub fn set_anchor(&self, conn: &mut ConnectionHandle, anchor: Anchor) {
        match self.kind {
            // We currently rely on the bitsets being the same
            SurfaceKind::Wlr(ref wlr) => wlr
                .set_anchor(conn, zwlr_layer_surface_v1::Anchor::from_bits_truncate(anchor.bits())),
        }
    }

    pub fn set_exclusive_zone(&self, conn: &mut ConnectionHandle, zone: i32) {
        match self.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_exclusive_zone(conn, zone),
        }
    }

    pub fn set_margin(
        &self,
        conn: &mut ConnectionHandle,
        top: i32,
        right: i32,
        bottom: i32,
        left: i32,
    ) {
        match self.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_margin(conn, top, right, bottom, left),
        }
    }

    pub fn set_keyboard_interactivity(
        &self,
        conn: &mut ConnectionHandle,
        value: KeyboardInteractivity,
    ) {
        match self.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_keyboard_interactivity(conn, value.into()),
        }
    }

    pub fn set_layer(&self, conn: &mut ConnectionHandle, depth: Layer) {
        match self.kind {
            SurfaceKind::Wlr(ref wlr) => wlr.set_layer(conn, depth.into()),
        }
    }

    pub fn kind(&self) -> &SurfaceKind {
        &self.kind
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.wl_surface
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
    // This is empty right now, but may be populated in the future.
}

#[macro_export]
macro_rules! delegate_layer {
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty: [
            $crate::reexports::protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1,
            $crate::reexports::protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1
        ] => $crate::shell::layer::LayerState);
    };
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

impl LayerSurface {
    /// Clone is an implementation detail of Layer.
    ///
    /// This function creates another layer handle that is not marked as a primary handle.
    pub(crate) fn impl_clone(&self) -> LayerSurface {
        LayerSurface {
            kind: self.kind.clone(),
            wl_surface: self.wl_surface.clone(),
            primary: false,
            death_signal: self.death_signal.clone(),
        }
    }
}

impl Drop for LayerSurface {
    fn drop(&mut self) {
        // If we are the primary handle (an owned value given to the user), mark ourselves for cleanup.
        if self.primary {
            self.death_signal.store(true, Ordering::SeqCst);
        }
    }
}

impl LayerState {
    pub(crate) fn cleanup(&mut self, conn: &mut ConnectionHandle) {
        self.surfaces.retain(|layer| {
            let alive = !layer.death_signal.load(Ordering::SeqCst);

            if !alive {
                // Layer shell protocol dictates we must destroy the role object before the surface.
                match layer.kind() {
                    SurfaceKind::Wlr(wlr) => wlr.destroy(conn),
                }

                layer.wl_surface().destroy(conn);
            }

            alive
        })
    }
}
