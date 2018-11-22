//! Utility functions for creating dpi aware wayland surfaces.
use env::Environment;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};
use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequest;
use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::Proxy;

struct SurfaceUserData {
    dpi_factor: Arc<Mutex<i32>>,
    outputs: Arc<RwLock<Vec<Proxy<wl_output::WlOutput>>>>,
}

impl SurfaceUserData {
    fn new() -> Self {
        SurfaceUserData {
            dpi_factor: Arc::new(Mutex::new(1)),
            outputs: Arc::new(RwLock::new(Vec::new())),
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
    F: FnMut(&Proxy<wl_surface::WlSurface>, i32) + Send + 'static,
{
    environment
        .compositor
        .create_surface(move |surface| {
            let output_manager = environment.outputs.clone();
            surface.implement(
                move |event, surface| {
                    let mut outputs = surface
                        .user_data::<SurfaceUserData>()
                        .unwrap()
                        .outputs
                        .write()
                        .unwrap();
                    let old_scale_factor = get_dpi_factor(&surface);
                    match event {
                        wl_surface::Event::Enter { output } => {
                            outputs.push(output);
                        }
                        wl_surface::Event::Leave { output } => {
                            outputs.retain(|output2| output.id() != output2.id());
                        }
                    };
                    let mut scale_factor = 1;
                    for output in &*outputs {
                        let scale_factor2 = output_manager
                            .with_info(&output, |_id, info| info.scale_factor)
                            .unwrap();
                        scale_factor = ::std::cmp::max(scale_factor, scale_factor2);
                    }
                    if old_scale_factor != scale_factor {
                        {
                            let mut ref_scale_factor = surface
                                .user_data::<SurfaceUserData>()
                                .unwrap()
                                .dpi_factor
                                .lock()
                                .unwrap();
                            *ref_scale_factor = scale_factor;
                        }
                        dpi_change(&surface, scale_factor);
                    }
                },
                SurfaceUserData::new(),
            )
        }).unwrap()
}

/// Returns the current dpi factor of a surface.
pub fn get_dpi_factor(surface: &Proxy<wl_surface::WlSurface>) -> i32 {
    *surface
        .user_data::<SurfaceUserData>()
        .expect("Surface was not created with create_surface.")
        .dpi_factor
        .lock()
        .unwrap()
}

/// Returns a list of outputs the surface is displayed on.
pub fn get_outputs<'a>(
    surface: &'a Proxy<wl_surface::WlSurface>,
) -> RwLockReadGuard<Vec<Proxy<wl_output::WlOutput>>> {
    surface
        .user_data::<SurfaceUserData>()
        .expect("Surface was not created with create_surface.")
        .outputs
        .read()
        .unwrap()
}
