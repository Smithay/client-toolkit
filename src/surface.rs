//! Utility functions for creating dpi aware wayland surfaces.
use output::OutputMgr;
use std::sync::{Arc, Mutex};
use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequest;
use wayland_client::protocol::{wl_compositor, wl_output, wl_surface};
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
            let scale_factor2 = self
                .output_manager
                .with_info(&output, |_id, info| info.scale_factor)
                .unwrap();
            scale_factor = ::std::cmp::max(scale_factor, scale_factor2);
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
    compositor: &Proxy<wl_compositor::WlCompositor>,
    output_manager: &OutputMgr,
    dpi_change: Box<F>,
) -> Proxy<wl_surface::WlSurface>
where
    F: FnMut(i32, Proxy<wl_surface::WlSurface>) + Send + 'static,
{
    compositor
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
                    output_manager.clone(),
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

/// Surface Manager
#[derive(Clone)]
pub struct SurfaceManager {
    output_manager: OutputMgr,
    surfaces: Arc<Mutex<Vec<Proxy<wl_surface::WlSurface>>>>,
}

impl SurfaceManager {
    /// Creates a new Surface Manager
    pub fn new(output_manager: OutputMgr) -> Self {
        SurfaceManager {
            output_manager,
            surfaces: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a new dpi aware surface
    ///
    /// The provided callback will be fired whenever the DPI factor associated to it
    /// changes.
    ///
    /// The DPI factor associated to a surface is defined as the maximum of the DPI
    /// factors of the outputs it is displayed on.
    pub fn create_surface<F>(&self, compositor: &Proxy<wl_compositor::WlCompositor>, dpi_change: F) -> Proxy<wl_surface::WlSurface>
    where
        F: FnMut(i32, Proxy<wl_surface::WlSurface>) + Send + 'static,
    {
        let surface = create_surface(compositor, &self.output_manager, Box::new(dpi_change));
        self.surfaces.lock().unwrap().push(surface.clone());
        surface
    }

    /// Some compositors don't send a leave notification to the surface when an
    /// output is destroyed. Hook this up to the GlobalManager to make dpi
    /// scale work.
    pub fn leave_output(&self, id: u32) {
        let output = self.output_manager
            .find_id(id, |output, _info| output.clone())
            .unwrap();
        for surface in &*self.surfaces.lock().unwrap() {
            surface
                .user_data::<Mutex<SurfaceUserData>>()
                .expect("Surface was not created with create_surface.")
                .lock()
                .unwrap()
                .leave(&output, surface.clone())
        }
    }
}
