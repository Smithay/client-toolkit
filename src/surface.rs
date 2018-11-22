//! Utility functions for creating dpi aware wayland surfaces.
use env::Environment;
use std::sync::Mutex;
use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequest;
use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::Proxy;

struct SurfaceUserData {
    dpi_factor: i32,
    outputs: Vec<Proxy<wl_output::WlOutput>>,
}

impl SurfaceUserData {
    fn new() -> Self {
        SurfaceUserData {
            dpi_factor: 1,
            outputs: Vec::new(),
        }
    }
}

/// Creates a WlSurface from an Environment.
///
/// Computes the dpi factor by taking the maximum dpi value of all the outputs
/// the surface is displayed on. The dpi factor is stored in the Proxy's user
/// data. When the dpi value is updated the caller is notified through the
/// dpi_change closure.
pub fn create_surface<F>(
    environment: &Environment,
    mut dpi_change: F,
) -> Proxy<wl_surface::WlSurface>
where
    F: FnMut(i32, Proxy<wl_surface::WlSurface>) + Send + 'static,
{
    environment
        .compositor
        .create_surface(move |surface| {
            let output_manager = environment.outputs.clone();
            surface.implement(
                move |event, surface| {
                    let mut user_data = surface
                        .user_data::<Mutex<SurfaceUserData>>()
                        .unwrap()
                        .lock()
                        .unwrap();
                    match event {
                        wl_surface::Event::Enter { output } => {
                            user_data.outputs.push(output);
                        }
                        wl_surface::Event::Leave { output } => {
                            user_data.outputs.retain(|output2| {
                                output.id() != output2.id()
                            });
                        }
                    };
                    let mut scale_factor = 1;
                    for output in &user_data.outputs {
                        let scale_factor2 = output_manager
                            .with_info(&output, |_id, info| info.scale_factor)
                            .unwrap();
                        scale_factor = ::std::cmp::max(scale_factor, scale_factor2);
                    }
                    if user_data.dpi_factor != scale_factor {
                        user_data.dpi_factor = scale_factor;
                        dpi_change(scale_factor, surface.clone());
                    }
                },
                Mutex::new(SurfaceUserData::new()),
            )
        }).unwrap()
}

/// Returns the current dpi factor of a surface.
pub fn get_dpi_factor(surface: &Proxy<wl_surface::WlSurface>) -> i32 {
    surface
        .user_data::<Mutex<SurfaceUserData>>()
        .expect("Surface was not created with create_surface.")
        .lock()
        .unwrap()
        .dpi_factor
}

/// Returns a list of outputs the surface is displayed on.
pub fn get_outputs<'a>(
    surface: &'a Proxy<wl_surface::WlSurface>,
) -> Vec<Proxy<wl_output::WlOutput>> {
    surface
        .user_data::<Mutex<SurfaceUserData>>()
        .expect("Surface was not created with create_surface.")
        .lock()
        .unwrap()
        .outputs
        .clone()
}
