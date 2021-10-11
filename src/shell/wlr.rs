/// Wlr shell abstraction
use std::cell::Cell;
use std::rc::Rc;

use wayland_client::{
    protocol::{wl_output, wl_surface},
    Attached, Main,
};

use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

#[derive(PartialEq, Copy, Clone)]
pub enum RenderEvent {
    Configure { width: u32, height: u32 },
    Closed,
}

/// Just a tiny abstraction making layers easier.  A layer specifies a z depth for where something is drawn. A normal xdg shell is normally drawn somewhere in between Bottom and Top
#[derive(PartialEq, Copy, Clone)]
pub enum Layer {
    /// All the way in the back, a example usecase is for backgrounds
    Background,

    /// In front of background but behind normal xdg shells
    Bottom,

    /// Behind overlay but in front normal xdg shells
    Top,

    /// All the way in the front, overiding what other applications are trying to draw.
    Overlay,
}

/// Just a tiny abstraction making anchors easier. Anchor says something about where on the screen the shell is rendered

#[derive(PartialEq, Copy, Clone)]
pub enum Anchor {
    /// Top of the screen
    Top,

    /// Bottom of the screen
    Bottom,

    /// Left side of the screen
    Left,

    /// Right side of the screen
    Right,
}

impl Anchor {
    pub(crate) fn too_raw(&self) -> zwlr_layer_surface_v1::Anchor {
        match *self {
            Anchor::Top => zwlr_layer_surface_v1::Anchor::Top,
            Anchor::Bottom => zwlr_layer_surface_v1::Anchor::Bottom,
            Anchor::Left => zwlr_layer_surface_v1::Anchor::Left,
            Anchor::Right => zwlr_layer_surface_v1::Anchor::Right,
        }
    }
}

impl Layer {
    pub(crate) fn too_raw(&self) -> zwlr_layer_shell_v1::Layer {
        match *self {
            Layer::Background => zwlr_layer_shell_v1::Layer::Background,
            Layer::Bottom => zwlr_layer_shell_v1::Layer::Bottom,
            Layer::Top => zwlr_layer_shell_v1::Layer::Top,
            Layer::Overlay => zwlr_layer_shell_v1::Layer::Overlay,
        }
    }
}

/// A struct representing the wlr shell

pub struct WlrShell {
    /// The raw wl_surface
    pub surface: wl_surface::WlSurface,

    pub(crate) layer_surface: Main<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,

    /// On what layer it should be positioned
    pub layer: Layer,

    /// Where on the screen it is positioned
    pub anchor: Anchor,

    /// The dimensions of the wlr surface
    pub dimensions: (u32, u32),

    /// The next render event
    pub render_event: Rc<Cell<Option<RenderEvent>>>,
}

impl WlrShell {
    /// Create a new wlr shell

    pub fn new(
        output: &wl_output::WlOutput,
        surface: wl_surface::WlSurface,
        layer_shell: &Attached<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
        layer: Layer,
        anchor: Anchor,
        dimensions: (u32, u32),
    ) -> Self {
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            Some(output),
            layer.too_raw(),
            "example".to_owned(),
        );

        layer_surface.set_size(dimensions.0, dimensions.1);
        // Anchor to the top left corner of the output
        layer_surface.set_anchor(anchor.too_raw());

        let next_render_event = Rc::new(Cell::new(None::<RenderEvent>));
        let next_render_event_handle = Rc::clone(&next_render_event);
        layer_surface.quick_assign(move |layer_surface, event, _| {
            match (event, next_render_event_handle.get()) {
                (zwlr_layer_surface_v1::Event::Closed, _) => {
                    next_render_event_handle.set(Some(RenderEvent::Closed));
                }
                (zwlr_layer_surface_v1::Event::Configure { serial, width, height }, next)
                    if next != Some(RenderEvent::Closed) =>
                {
                    layer_surface.ack_configure(serial);
                    next_render_event_handle.set(Some(RenderEvent::Configure { width, height }));
                }
                (_, _) => {}
            }
        });

        // Commit so that the server will send a configure event
        surface.commit();

        Self {
            surface,
            layer_surface,
            layer,
            anchor,
            render_event: next_render_event,
            dimensions: (dimensions.0 as u32, dimensions.1 as u32),
        }
    }
}

impl Drop for WlrShell {
    fn drop(&mut self) {
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}
