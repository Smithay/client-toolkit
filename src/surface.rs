//! Utility functions for creating dpi aware wayland surfaces.
use env::Environment;
use output::OutputMgr;
use std::sync::Mutex;
use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequest;
use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::Proxy;

pub(crate) struct SurfaceUserData {
    dpi_factor: i32,
    outputs: Vec<Proxy<wl_output::WlOutput>>,
    output_manager: OutputMgr,
    dpi_change_cb: Box<FnMut(i32, Proxy<wl_surface::WlSurface>) + Send + 'static>,
}

impl SurfaceUserData {
    fn new(
        output_manager: OutputMgr,
        dpi_change_cb: Box<FnMut(i32, Proxy<wl_surface::WlSurface>) + Send + 'static>,
    ) -> Self {
        SurfaceUserData {
            dpi_factor: 1,
            outputs: Vec::new(),
            output_manager,
            dpi_change_cb,
        }
    }

    pub(crate) fn enter(
        &mut self,
        output: Proxy<wl_output::WlOutput>,
        surface: Proxy<wl_surface::WlSurface>,
    ) {
        self.outputs.push(output);
        self.compute_dpi_factor(surface);
    }

    pub(crate) fn leave(
        &mut self,
        output: &Proxy<wl_output::WlOutput>,
        surface: Proxy<wl_surface::WlSurface>,
    ) {
        self.outputs.retain(|output2| !output.equals(output2));
        self.compute_dpi_factor(surface);
    }

    fn compute_dpi_factor(&mut self, surface: Proxy<wl_surface::WlSurface>) {
        let mut scale_factor = 1;
        for output in &self.outputs {
            if let Some(scale_factor2) = self
                .output_manager
                .with_info(&output, |_id, info| info.scale_factor)
            {
                scale_factor = ::std::cmp::max(scale_factor, scale_factor2);
            }
        }
        if self.dpi_factor != scale_factor {
            self.dpi_factor = scale_factor;
            (self.dpi_change_cb)(scale_factor, surface.clone());
        }
    }
}

/// Creates a WlSurface from an Environment.
///
/// Computes the dpi factor by taking the maximum dpi value of all the outputs
/// the surface is displayed on. The dpi factor is stored in the Proxy's user
/// data. When the dpi value is updated the caller is notified through the
/// dpi_change closure.
pub(crate) fn create_surface<F>(
    environment: &Environment,
    dpi_change: Box<F>,
) -> Proxy<wl_surface::WlSurface>
where
    F: FnMut(i32, Proxy<wl_surface::WlSurface>) + Send + 'static,
{
    environment
        .compositor
        .create_surface(move |surface| {
            surface.implement(
                move |event, surface| {
                    let mut user_data = surface
                        .user_data::<Mutex<SurfaceUserData>>()
                        .unwrap()
                        .lock()
                        .unwrap();
                    match event {
                        wl_surface::Event::Enter { output } => {
                            user_data.enter(output, surface.clone());
                        }
                        wl_surface::Event::Leave { output } => {
                            user_data.leave(&output, surface.clone());
                        }
                    };
                },
                Mutex::new(SurfaceUserData::new(
                    environment.outputs.clone(),
                    dpi_change,
                )),
            )
        })
        .unwrap()
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
pub fn get_outputs(surface: &Proxy<wl_surface::WlSurface>) -> Vec<Proxy<wl_output::WlOutput>> {
    surface
        .user_data::<Mutex<SurfaceUserData>>()
        .expect("Surface was not created with create_surface.")
        .lock()
        .unwrap()
        .outputs
        .clone()
}
