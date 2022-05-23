use std::sync::{
    atomic::{AtomicI32, Ordering},
    Mutex,
};

use wayland_client::{
    protocol::{wl_callback, wl_compositor, wl_output, wl_region, wl_surface},
    Connection, DelegateDispatch, Dispatch, Proxy, QueueHandle,
};

use crate::{
    error::GlobalError,
    output::OutputData,
    registry::{ProvidesRegistryState, RegistryHandler},
};

pub trait CompositorHandler: Sized {
    fn compositor_state(&mut self) -> &mut CompositorState;

    /// The surface has either been moved into or out of an output and the output has a different scale factor.
    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        new_factor: i32,
    );

    /// A frame callback has been completed.
    ///
    /// This function will be called after sending a [`WlSurface::frame`](wl_surface::WlSurface::frame) request
    /// and committing the surface.
    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        time: u32,
    );
}

#[derive(Debug)]
pub struct CompositorState {
    wl_compositor: Option<wl_compositor::WlCompositor>,
}

impl CompositorState {
    pub fn new() -> CompositorState {
        CompositorState { wl_compositor: None }
    }

    pub fn wl_compositor(&self) -> Option<&wl_compositor::WlCompositor> {
        self.wl_compositor.as_ref()
    }

    pub fn create_surface<D>(
        &self,
        qh: &QueueHandle<D>,
    ) -> Result<wl_surface::WlSurface, GlobalError>
    where
        D: Dispatch<wl_surface::WlSurface, SurfaceData> + 'static,
    {
        let compositor =
            self.wl_compositor.as_ref().ok_or(GlobalError::MissingGlobals(&["wl_compositor"]))?;

        let surface = compositor.create_surface(
            qh,
            SurfaceData { scale_factor: AtomicI32::new(1), outputs: Mutex::new(vec![]) },
        )?;

        Ok(surface)
    }

    pub fn create_region<D>(&self, qh: &QueueHandle<D>) -> Result<wl_region::WlRegion, GlobalError>
    where
        D: Dispatch<wl_region::WlRegion, ()> + 'static,
    {
        let compositor =
            self.wl_compositor.as_ref().ok_or(GlobalError::MissingGlobals(&["wl_compositor"]))?;

        compositor.create_region(qh, ()).map_err(Into::into)
    }
}

/// Data associated with a [`WlSurface`](wl_surface::WlSurface).
#[derive(Debug)]
pub struct SurfaceData {
    /// The scale factor of the output with the highest scale factor.
    pub(crate) scale_factor: AtomicI32,

    /// The outputs the surface is currently inside.
    pub(crate) outputs: Mutex<Vec<wl_output::WlOutput>>,
}

#[macro_export]
macro_rules! delegate_compositor {
    ($ty: ty) => {
        type __WlCompositor = $crate::reexports::client::protocol::wl_compositor::WlCompositor;
        type __WlSurface = $crate::reexports::client::protocol::wl_surface::WlSurface;
        type __WlRegion = $crate::reexports::client::protocol::wl_region::WlRegion;
        type __WlCallback = $crate::reexports::client::protocol::wl_callback::WlCallback;

        $crate::reexports::client::delegate_dispatch!($ty:
            [
                __WlCompositor: (),
                __WlSurface: $crate::compositor::SurfaceData,
                __WlRegion: (),
                __WlCallback: __WlSurface,
            ] => $crate::compositor::CompositorState
        );
    };
}

impl<D> DelegateDispatch<wl_surface::WlSurface, SurfaceData, D> for CompositorState
where
    D: Dispatch<wl_surface::WlSurface, SurfaceData>
        + Dispatch<wl_output::WlOutput, OutputData>
        + CompositorHandler,
{
    fn event(
        state: &mut D,
        surface: &wl_surface::WlSurface,
        event: wl_surface::Event,
        data: &SurfaceData,
        conn: &Connection,
        qh: &QueueHandle<D>,
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
                state.scale_factor_changed(conn, qh, surface, factor);
            }
        }
    }
}

impl<D> DelegateDispatch<wl_region::WlRegion, (), D> for CompositorState
where
    D: Dispatch<wl_region::WlRegion, ()> + CompositorHandler,
{
    fn event(
        _: &mut D,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("wl_region has no events")
    }
}

impl<D> DelegateDispatch<wl_compositor::WlCompositor, (), D> for CompositorState
where
    D: Dispatch<wl_compositor::WlCompositor, ()> + CompositorHandler,
{
    fn event(
        _: &mut D,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("wl_compositor has no events")
    }
}

impl<D> DelegateDispatch<wl_callback::WlCallback, wl_surface::WlSurface, D> for CompositorState
where
    D: Dispatch<wl_callback::WlCallback, wl_surface::WlSurface> + CompositorHandler,
{
    fn event(
        state: &mut D,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        surface: &wl_surface::WlSurface,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wl_callback::Event::Done { callback_data } => {
                state.frame(conn, qh, surface, callback_data);
            }

            _ => unreachable!(),
        }
    }
}

impl<D> RegistryHandler<D> for CompositorState
where
    D: Dispatch<wl_compositor::WlCompositor, ()>
        + CompositorHandler
        + ProvidesRegistryState
        + 'static,
{
    fn new_global(
        state: &mut D,
        _conn: &Connection,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        if interface == "wl_compositor" {
            let compositor = state
                .registry()
                .bind_once::<wl_compositor::WlCompositor, _, _>(qh, name, u32::min(version, 4), ())
                .expect("Failed to bind global");

            state.compositor_state().wl_compositor = Some(compositor);
        }
    }

    fn remove_global(_state: &mut D, _conn: &Connection, _qh: &QueueHandle<D>, _name: u32) {
        // Capability global, removal is unlikely
    }
}
