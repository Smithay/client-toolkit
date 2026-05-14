use crate::reexports::client::globals::{BindError, GlobalList};
use crate::reexports::client::protocol::wl_compositor::WlCompositor;
use crate::reexports::client::protocol::wl_subcompositor::WlSubcompositor;
use crate::reexports::client::protocol::wl_subsurface::WlSubsurface;
use crate::reexports::client::protocol::wl_surface::WlSurface;
use crate::reexports::client::{Connection, Dispatch, Proxy, QueueHandle};

use crate::compositor::SurfaceData;
use crate::dispatch2::Dispatch2;
use crate::globals::GlobalData;

#[derive(Debug)]
pub struct SubcompositorState {
    compositor: WlCompositor,
    subcompositor: WlSubcompositor,
}

impl SubcompositorState {
    pub fn bind<State>(
        compositor: WlCompositor,
        globals: &GlobalList,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, BindError>
    where
        State: Dispatch<WlSubcompositor, GlobalData, State> + 'static,
    {
        let subcompositor = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(SubcompositorState { compositor, subcompositor })
    }

    pub fn create_subsurface<State>(
        &self,
        parent: WlSurface,
        queue_handle: &QueueHandle<State>,
    ) -> (WlSubsurface, WlSurface)
    where
        State:
            Dispatch<WlSurface, SurfaceData<()>> + Dispatch<WlSubsurface, SubsurfaceData> + 'static,
    {
        let surface_data = SurfaceData::new(Some(parent.clone()), 1, ());
        let surface = self.compositor.create_surface(queue_handle, surface_data);
        let subsurface_data = SubsurfaceData::new(surface.clone());
        let subsurface =
            self.subcompositor.get_subsurface(&surface, &parent, queue_handle, subsurface_data);
        (subsurface, surface)
    }

    pub fn subsurface_from_surface<State>(
        &self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<State>,
    ) -> Option<WlSubsurface>
    where
        State:
            Dispatch<WlSurface, SurfaceData<()>> + Dispatch<WlSubsurface, SubsurfaceData> + 'static,
    {
        let parent = surface.data::<SurfaceData<()>>().unwrap().parent_surface();
        let subsurface_data = SubsurfaceData::new(surface.clone());
        parent.map(|parent| {
            self.subcompositor.get_subsurface(surface, parent, queue_handle, subsurface_data)
        })
    }
}

impl<D> Dispatch2<WlSubsurface, D> for SubsurfaceData {
    fn event(
        &self,
        _: &mut D,
        _: &WlSubsurface,
        _: <WlSubsurface as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("wl_subsurface has no events")
    }
}

impl<D> Dispatch2<WlSubcompositor, D> for GlobalData {
    fn event(
        &self,
        _: &mut D,
        _: &WlSubcompositor,
        _: <WlSubcompositor as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("wl_subcompositor has no events")
    }
}

/// The data assoctiated with the subsurface.
#[derive(Debug)]
pub struct SubsurfaceData {
    /// The surface used when creating this subsurface.
    surface: WlSurface,
}

impl SubsurfaceData {
    pub(crate) fn new(surface: WlSurface) -> Self {
        Self { surface }
    }

    /// Get the surface used when creating the given subsurface.
    pub fn surface(&self) -> &WlSurface {
        &self.surface
    }
}
