//! Utility functions for creating dpi aware wayland surfaces.

use std::sync::{Arc, Mutex};
use wayland_client::Proxy;
use wayland_client::protocol::wl_surface;
use wayland_client::protocol::wl_compositor::{RequestsTrait as CompositorRequest};
use crate::env::Environment;

/// Creates a WlSurface from an Environment.
/// Takes a callback for notification of dpi changes.
pub fn create_surface<F, T>(
    environment: &Environment,
    dpi_change: F,
) -> Proxy<wl_surface::WlSurface>
where
    F: Fn(i32) -> T + Send + 'static
{
    environment.compositor.create_surface(|surface| {
        let output_manager = environment.outputs.clone();
        let outputs = Arc::new(Mutex::new(Vec::new()));
        surface.implement(move |event, surface| {
            let mut outputs = outputs.lock().unwrap();
            let old_scale_factor = get_dpi_factor(&surface);
            match event {
                wl_surface::Event::Enter { output } => {
                    outputs.push(output);
                },
                wl_surface::Event::Leave { output } => {
                    outputs.retain(|output2| output.id() != output2.id());
                },
            };
            let mut scale_factor = 1;
            for output in &*outputs {
                let scale_factor2 = output_manager
                    .with_info(&output, |_id, info| info.scale_factor)
                    .unwrap();
                scale_factor = std::cmp::max(scale_factor, scale_factor2);
            }
            if old_scale_factor != scale_factor {
                {
                    let mut ref_scale_factor = surface.user_data::<Mutex<i32>>().unwrap().lock().unwrap();
                    *ref_scale_factor = scale_factor;
                }
                dpi_change(scale_factor);
            }
        }, Mutex::new(1))
    }).unwrap()
}

/// Returns the current dpi factor of a surface.
pub fn get_dpi_factor(surface: &Proxy<wl_surface::WlSurface>) -> i32 {
    *surface.user_data::<Mutex<i32>>().unwrap().lock().unwrap()
}
