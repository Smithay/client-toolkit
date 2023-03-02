use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc, Mutex,
};

use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::{
        wl_callback, wl_compositor, wl_output, wl_region,
        wl_surface::{self, WlSurface},
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

use crate::{
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
    output::{OutputData, OutputHandler, OutputState, ScaleWatcherHandle},
};

pub trait CompositorHandler: Sized {
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

pub trait SurfaceDataExt: Send + Sync {
    fn surface_data(&self) -> &SurfaceData;
}

impl SurfaceDataExt for SurfaceData {
    fn surface_data(&self) -> &SurfaceData {
        self
    }
}

#[derive(Debug)]
pub struct CompositorState {
    wl_compositor: wl_compositor::WlCompositor,
}

impl CompositorState {
    pub fn bind<State>(
        globals: &GlobalList,
        qh: &QueueHandle<State>,
    ) -> Result<CompositorState, BindError>
    where
        State: Dispatch<wl_compositor::WlCompositor, GlobalData, State> + 'static,
    {
        let wl_compositor = globals.bind(qh, 1..=5, GlobalData)?;
        Ok(CompositorState { wl_compositor })
    }

    pub fn wl_compositor(&self) -> &wl_compositor::WlCompositor {
        &self.wl_compositor
    }

    pub fn create_surface<D>(&self, qh: &QueueHandle<D>) -> wl_surface::WlSurface
    where
        D: Dispatch<wl_surface::WlSurface, SurfaceData> + 'static,
    {
        self.create_surface_with_data(qh, Default::default())
    }

    pub fn create_surface_with_data<D, U>(
        &self,
        qh: &QueueHandle<D>,
        data: U,
    ) -> wl_surface::WlSurface
    where
        D: Dispatch<wl_surface::WlSurface, U> + 'static,
        U: SurfaceDataExt + 'static,
    {
        self.wl_compositor.create_surface(qh, data)
    }
}

/// Data associated with a [`WlSurface`](wl_surface::WlSurface).
#[derive(Debug)]
pub struct SurfaceData {
    /// The scale factor of the output with the highest scale factor.
    pub(crate) scale_factor: AtomicI32,

    /// Parent surface used when creating subsurfaces.
    ///
    /// For top-level surfaces this is always `None`.
    pub(crate) parent_surface: Option<WlSurface>,

    /// The inner mutable storage.
    inner: Mutex<SurfaceDataInner>,
}

#[derive(Debug, Default)]
struct SurfaceDataInner {
    /// The outputs the surface is currently inside.
    outputs: Vec<wl_output::WlOutput>,

    /// A handle to the OutputInfo callback that dispatches scale updates.
    watcher: Option<ScaleWatcherHandle>,
}

impl SurfaceData {
    /// Create a new surface that initially reports the given scale factor and parent.
    pub fn new(parent_surface: Option<WlSurface>, scale_factor: i32) -> Self {
        Self {
            scale_factor: AtomicI32::new(scale_factor),
            parent_surface,
            inner: Default::default(),
        }
    }

    /// The scale factor of the output with the highest scale factor.
    pub fn scale_factor(&self) -> i32 {
        self.scale_factor.load(Ordering::Relaxed)
    }

    /// The parent surface used for this surface.
    ///
    /// The surface is `Some` for primarily for subsurfaces,
    /// since they must have a parent surface.
    pub fn parent_surface(&self) -> Option<&WlSurface> {
        self.parent_surface.as_ref()
    }

    /// The outputs the surface is currently inside.
    pub fn outputs(&self) -> impl Iterator<Item = wl_output::WlOutput> {
        self.inner.lock().unwrap().outputs.clone().into_iter()
    }
}

impl Default for SurfaceData {
    fn default() -> Self {
        Self::new(None, 1)
    }
}

/// An owned [`WlSurface`](wl_surface::WlSurface).
///
/// This destroys the surface on drop.
#[derive(Debug)]
pub struct Surface(wl_surface::WlSurface);

impl Surface {
    pub fn new<D>(
        compositor: &impl ProvidesBoundGlobal<wl_compositor::WlCompositor, 5>,
        qh: &QueueHandle<D>,
    ) -> Result<Self, GlobalError>
    where
        D: Dispatch<wl_surface::WlSurface, SurfaceData> + 'static,
    {
        Self::with_data(compositor, qh, Default::default())
    }

    pub fn with_data<D, U>(
        compositor: &impl ProvidesBoundGlobal<wl_compositor::WlCompositor, 5>,
        qh: &QueueHandle<D>,
        data: U,
    ) -> Result<Self, GlobalError>
    where
        D: Dispatch<wl_surface::WlSurface, U> + 'static,
        U: Send + Sync + 'static,
    {
        Ok(Surface(compositor.bound_global()?.create_surface(qh, data)))
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.0
    }
}

impl From<wl_surface::WlSurface> for Surface {
    fn from(surface: wl_surface::WlSurface) -> Self {
        Surface(surface)
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.0.destroy();
    }
}

#[macro_export]
macro_rules! delegate_compositor {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_compositor::WlCompositor: $crate::globals::GlobalData
            ] => $crate::compositor::CompositorState
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
        [
            $crate::reexports::client::protocol::wl_surface::WlSurface: $crate::compositor::SurfaceData
        ] => $crate::compositor::CompositorState
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
        [
            $crate::reexports::client::protocol::wl_callback::WlCallback: $crate::reexports::client::protocol::wl_surface::WlSurface
        ] => $crate::compositor::CompositorState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, surface: [$($surface: ty),*$(,)?]) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_compositor::WlCompositor: $crate::globals::GlobalData
            ] => $crate::compositor::CompositorState
        );
        $(
            $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                    $crate::reexports::client::protocol::wl_surface::WlSurface: $surface
            ] => $crate::compositor::CompositorState
            );
        )*
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
        [
            $crate::reexports::client::protocol::wl_callback::WlCallback: $crate::reexports::client::protocol::wl_surface::WlSurface
        ] => $crate::compositor::CompositorState
        );
    };
}

impl<D, U> Dispatch<wl_surface::WlSurface, U, D> for CompositorState
where
    D: Dispatch<wl_surface::WlSurface, U> + CompositorHandler + OutputHandler + 'static,
    U: SurfaceDataExt + 'static,
{
    fn event(
        state: &mut D,
        surface: &wl_surface::WlSurface,
        event: wl_surface::Event,
        data: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let data = data.surface_data();
        let mut inner = data.inner.lock().unwrap();

        match event {
            wl_surface::Event::Enter { output } => {
                inner.outputs.push(output);
            }

            wl_surface::Event::Leave { output } => {
                inner.outputs.retain(|o| o != &output);
            }

            _ => unreachable!(),
        }

        inner.watcher.get_or_insert_with(|| {
            // Avoid storing the WlSurface inside the closure as that would create a reference
            // cycle.  Instead, store the ID and re-create the proxy.
            let id = surface.id();
            OutputState::add_scale_watcher(state, move |state, conn, qh, _| {
                let id = id.clone();
                if let Ok(surface) = wl_surface::WlSurface::from_id(conn, id) {
                    if let Some(data) = surface.data::<U>() {
                        let data = data.surface_data();
                        let inner = data.inner.lock().unwrap();
                        let current = data.scale_factor.load(Ordering::Relaxed);
                        let factor = match inner
                            .outputs
                            .iter()
                            .filter_map(|output| {
                                output.data::<OutputData>().map(OutputData::scale_factor)
                            })
                            .reduce(i32::max)
                        {
                            None => return,
                            Some(factor) if factor == current => return,
                            Some(factor) => factor,
                        };

                        data.scale_factor.store(factor, Ordering::Relaxed);
                        drop(inner);
                        state.scale_factor_changed(conn, qh, &surface, factor);
                    }
                }
            })
        });

        // Compute the new max of the scale factors for all outputs this surface is displayed on.
        let current = data.scale_factor.load(Ordering::Relaxed);

        let factor = match inner
            .outputs
            .iter()
            .filter_map(|output| output.data::<OutputData>().map(OutputData::scale_factor))
            .reduce(i32::max)
        {
            // If no scale factor is found, because the surface has left its only output, do not
            // change the scale factor.
            None => return,
            Some(factor) if factor == current => return,
            Some(factor) => factor,
        };

        data.scale_factor.store(factor, Ordering::Relaxed);

        // Drop the mutex before we send of any events.
        drop(inner);

        state.scale_factor_changed(conn, qh, surface, factor);
    }
}

/// A trivial wrapper around a [`WlRegion`][wl_region::WlRegion].
///
/// This destroys the region on drop.
#[derive(Debug)]
pub struct Region(wl_region::WlRegion);

impl Region {
    pub fn new(
        compositor: &impl ProvidesBoundGlobal<wl_compositor::WlCompositor, 5>,
    ) -> Result<Region, GlobalError> {
        compositor
            .bound_global()
            .map(|c| {
                c.send_constructor(wl_compositor::Request::CreateRegion {}, Arc::new(RegionData))
                    .unwrap_or_else(|_| Proxy::inert(c.backend().clone()))
            })
            .map(Region)
    }

    pub fn add(&self, x: i32, y: i32, width: i32, height: i32) {
        self.0.add(x, y, width, height)
    }

    pub fn subtract(&self, x: i32, y: i32, width: i32, height: i32) {
        self.0.subtract(x, y, width, height)
    }

    pub fn wl_region(&self) -> &wl_region::WlRegion {
        &self.0
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        self.0.destroy()
    }
}

struct RegionData;

impl wayland_client::backend::ObjectData for RegionData {
    fn event(
        self: Arc<Self>,
        _: &wayland_client::backend::Backend,
        _: wayland_client::backend::protocol::Message<
            wayland_client::backend::ObjectId,
            wayland_backend::io_lifetimes::OwnedFd,
        >,
    ) -> Option<Arc<(dyn wayland_client::backend::ObjectData + 'static)>> {
        unreachable!("wl_region has no events");
    }
    fn destroyed(&self, _: wayland_client::backend::ObjectId) {}
}

impl<D> Dispatch<wl_compositor::WlCompositor, GlobalData, D> for CompositorState
where
    D: Dispatch<wl_compositor::WlCompositor, GlobalData> + CompositorHandler,
{
    fn event(
        _: &mut D,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("wl_compositor has no events")
    }
}

impl ProvidesBoundGlobal<wl_compositor::WlCompositor, 5> for CompositorState {
    fn bound_global(&self) -> Result<wl_compositor::WlCompositor, GlobalError> {
        Ok(self.wl_compositor.clone())
    }
}

impl<D> Dispatch<wl_callback::WlCallback, wl_surface::WlSurface, D> for CompositorState
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
