use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Mutex,
    },
};

use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_callback, wl_compositor, wl_output, wl_surface},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy, QueueHandle,
};

use crate::{
    output::OutputData,
    registry::{RegistryHandle, RegistryHandler},
};

/// An error caused by creating a surface.
#[derive(Debug, thiserror::Error)]
pub enum SurfaceError {
    /// The compositor global is not available.
    #[error("the compositor global is not available")]
    MissingCompositorGlobal,

    /// Protocol error.
    #[error(transparent)]
    Protocol(#[from] InvalidId),
}

pub trait SurfaceHandler<D> {
    /// The surface has either been moved into or out of an output and the output has a different scale factor.
    fn scale_factor_changed(
        &mut self,
        state: &mut CompositorState,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        surface: &wl_surface::WlSurface,
        new_factor: i32,
    );

    /// A frame callback has been completed.
    ///
    /// This function will be called after sending a [`WlSurface::frame`](wl_surface::WlSurface::frame) request
    /// and committing the surface.
    fn frame(
        &mut self,
        state: &mut CompositorState,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        surface: &wl_surface::WlSurface,
        time: u32,
    );
}

#[derive(Debug)]
pub struct CompositorState {
    wl_compositor: Option<(u32, wl_compositor::WlCompositor)>,
}

impl CompositorState {
    pub fn new() -> CompositorState {
        CompositorState { wl_compositor: None }
    }

    pub fn create_surface<D>(
        &self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) -> Result<wl_surface::WlSurface, SurfaceError>
    where
        D: Dispatch<wl_surface::WlSurface, UserData = SurfaceData> + 'static,
    {
        let (_, compositor) =
            self.wl_compositor.as_ref().ok_or(SurfaceError::MissingCompositorGlobal)?;

        let surface = compositor.create_surface(
            conn,
            qh,
            SurfaceData {
                scale_factor: AtomicI32::new(1),
                outputs: Mutex::new(vec![]),
                has_role: AtomicBool::new(false),
            },
        )?;

        Ok(surface)
    }
}

/// Data associated with a [`WlSurface`](wl_surface::WlSurface).
#[derive(Debug)]
pub struct SurfaceData {
    /// The scale factor of the output with the highest scale factor.
    pub(crate) scale_factor: AtomicI32,

    /// The outputs the surface is currently inside.
    pub(crate) outputs: Mutex<Vec<wl_output::WlOutput>>,

    /// Whether the surface has a role object.
    pub(crate) has_role: AtomicBool,
}

#[derive(Debug)]
pub struct SurfaceDispatch<'s, D, H: SurfaceHandler<D>>(
    pub &'s mut CompositorState,
    pub &'s mut H,
    pub PhantomData<D>,
);

impl<D, H: SurfaceHandler<D>> DelegateDispatchBase<wl_surface::WlSurface>
    for SurfaceDispatch<'_, D, H>
{
    type UserData = SurfaceData;
}

impl<D, H> DelegateDispatch<wl_surface::WlSurface, D> for SurfaceDispatch<'_, D, H>
where
    H: SurfaceHandler<D>,
    D: Dispatch<wl_surface::WlSurface, UserData = Self::UserData>
        + Dispatch<wl_output::WlOutput, UserData = OutputData>,
{
    fn event(
        &mut self,
        surface: &wl_surface::WlSurface,
        event: wl_surface::Event,
        data: &Self::UserData,
        conn: &mut ConnectionHandle,
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
                self.1.scale_factor_changed(self.0, conn, qh, surface, factor);
            }
        }
    }
}

impl<D, H: SurfaceHandler<D>> DelegateDispatchBase<wl_compositor::WlCompositor>
    for SurfaceDispatch<'_, D, H>
{
    type UserData = ();
}

impl<D, H> DelegateDispatch<wl_compositor::WlCompositor, D> for SurfaceDispatch<'_, D, H>
where
    H: SurfaceHandler<D>,
    D: Dispatch<wl_compositor::WlCompositor, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        unreachable!("wl_compositor has no events")
    }
}

impl<D, H: SurfaceHandler<D>> DelegateDispatchBase<wl_callback::WlCallback>
    for SurfaceDispatch<'_, D, H>
{
    type UserData = wl_surface::WlSurface;
}

impl<D, H> DelegateDispatch<wl_callback::WlCallback, D> for SurfaceDispatch<'_, D, H>
where
    H: SurfaceHandler<D>,
    D: Dispatch<wl_callback::WlCallback, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        surface: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wl_callback::Event::Done { callback_data } => {
                self.1.frame(self.0, conn, qh, surface, callback_data);
            }

            _ => unreachable!(),
        }
    }
}

impl<D> RegistryHandler<D> for CompositorState
where
    D: Dispatch<wl_compositor::WlCompositor, UserData = ()> + 'static,
{
    fn new_global(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
        handle: &mut RegistryHandle,
    ) {
        if interface == "wl_compositor" {
            let compositor = handle
                .bind_once::<wl_compositor::WlCompositor, _, _>(
                    conn,
                    qh,
                    name,
                    u32::min(version, 4),
                    (),
                )
                .expect("Failed to bind global");

            self.wl_compositor = Some((name, compositor));
        }
    }

    fn remove_global(&mut self, _conn: &mut ConnectionHandle, _qh: &QueueHandle<D>, name: u32) {
        if self
            .wl_compositor
            .as_ref()
            .map(|(compositor_name, _)| *compositor_name == name)
            .unwrap_or(false)
        {
            self.wl_compositor.take();
        }
    }
}
