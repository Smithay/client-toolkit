use std::sync::{
    atomic::{AtomicI32, Ordering},
    Mutex,
};

use wayland_client::{
    protocol::{wl_compositor, wl_output, wl_surface},
    ConnectionHandle, DataInit, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy,
    QueueHandle,
};

use crate::output::OutputData;

pub trait SurfaceHandler {
    /// The surface has either been moved into or out of an output and the output has a different scale factor.
    fn scale_factor_changed(&mut self, surface: &wl_surface::WlSurface, new_factor: i32);
}

#[derive(Debug)]
pub struct CompositorState {
    wl_compositor: Option<wl_compositor::WlCompositor>,
}

impl CompositorState {
    pub fn new() -> CompositorState {
        CompositorState { wl_compositor: None }
    }

    #[deprecated = "This is a temporary hack until some way to notify delegates a global was created is available."]
    pub fn compositor_bind(&mut self, wl_compositor: wl_compositor::WlCompositor) {
        self.wl_compositor = Some(wl_compositor);
    }
}

/// Data associated with a [`WlSurface`](wl_surface::WlSurface).
#[derive(Debug)]
pub struct SurfaceData {
    /// The scale factor of the output with the highest scale factor.
    scale_factor: AtomicI32,
    /// The outputs the surface is currently inside.
    outputs: Mutex<Vec<wl_output::WlOutput>>,
}

#[derive(Debug)]
pub struct SurfaceDispatch<'s, H: SurfaceHandler>(pub &'s mut CompositorState, pub &'s mut H);

impl<H: SurfaceHandler> DelegateDispatchBase<wl_surface::WlSurface> for SurfaceDispatch<'_, H> {
    type UserData = SurfaceData;
}

impl<D, H> DelegateDispatch<wl_surface::WlSurface, D> for SurfaceDispatch<'_, H>
where
    H: SurfaceHandler,
    D: Dispatch<wl_surface::WlSurface, UserData = Self::UserData>
        + Dispatch<wl_output::WlOutput, UserData = OutputData>,
{
    fn event(
        &mut self,
        surface: &wl_surface::WlSurface,
        event: wl_surface::Event,
        data: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
        _: &mut DataInit<'_>,
    ) {
        let mut outputs = data.outputs.lock().unwrap();

        match event {
            wl_surface::Event::Enter { output } => {
                outputs.push(output);
            }

            wl_surface::Event::Leave { output } => {
                outputs.retain(|o| o != &output);
            }

            _ => unreachable!(),
        }

        // Compute the new max of the scale factors for all outputs this surface is displayed on.
        let current = data.scale_factor.load(Ordering::SeqCst);

        let largest_factor = outputs
            .iter()
            .filter_map(|output| output.data::<OutputData>().map(OutputData::scale_factor))
            .reduce(i32::max);

        // Drop the mutex before we send of any events.
        drop(outputs);

        // If no scale factor is found, because the surface has left it's only output, do not change the scale factor.
        if let Some(factor) = largest_factor {
            data.scale_factor.store(factor, Ordering::SeqCst);

            if current != factor {
                self.1.scale_factor_changed(surface, factor);
            }
        }
    }
}
