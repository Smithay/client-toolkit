/// WLR layer abstraction
use std::cell::Cell;
use std::rc::Rc;

use wayland_client::{
    protocol::{wl_output, wl_surface},
    Attached, Main,
};

use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

pub use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_shell_v1::Layer, zwlr_layer_surface_v1::Anchor,
};

/// Render event

#[derive(PartialEq, Copy, Clone)]
pub enum RenderEvent {
    /// Surface wants a reconfiguration/configuration
    Configure {
        /// The new width of the surface
        width: u32,

        /// The new height of the surface
        height: u32,
    },

    /// Surface has closed and wants to be closed here also.
    Closed,
}

/// A struct representing the layer surface (wlr)

pub struct LayerSurface {
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

impl LayerSurface {
    /// Create a new wlr shell

    pub fn new(
        output: &wl_output::WlOutput,
        surface: wl_surface::WlSurface,
        layer_shell: &Attached<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
        layer: Layer,
        anchor: Anchor,
        dimensions: (u32, u32),
    ) -> Self {
        let layer_surface =
            layer_shell.get_layer_surface(&surface, Some(output), layer, "example".to_owned());

        layer_surface.set_size(dimensions.0, dimensions.1);
        // Anchor to the top left corner of the output
        layer_surface.set_anchor(anchor);

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

impl Drop for LayerSurface {
    fn drop(&mut self) {
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}
