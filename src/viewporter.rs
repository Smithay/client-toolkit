use crate::reexports::client::globals::{BindError, GlobalList};
use crate::reexports::client::protocol::wl_surface::WlSurface;
use crate::reexports::client::{Connection, Dispatch, Proxy, QueueHandle};
use crate::reexports::protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};

use crate::globals::GlobalData;

#[derive(Debug)]
pub struct Viewporter {
    viewporter: WpViewporter,
}

impl Viewporter {
    pub fn bind<State>(
        globals: &GlobalList,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, BindError>
    where
        State: Dispatch<WpViewporter, GlobalData, State> + 'static,
    {
        let viewporter = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Viewporter { viewporter })
    }

    pub fn get_viewport<State>(
        &self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<State>,
    ) -> WpViewport
    where
        State: Dispatch<WpViewport, Viewport> + 'static,
    {
        let viewport_data = Viewport::new(surface.clone());
        let viewport = self.viewporter.get_viewport(&surface, queue_handle, viewport_data);
        viewport
    }
}

impl<D> Dispatch<WpViewport, Viewport, D> for Viewporter
where
    D: Dispatch<WpViewport, Viewport>,
{
    fn event(
        _: &mut D,
        _: &WpViewport,
        _: <WpViewport as Proxy>::Event,
        _: &Viewport,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("wp_viewport has no events")
    }
}

impl<D> Dispatch<WpViewporter, GlobalData, D> for Viewporter
where
    D: Dispatch<WpViewporter, GlobalData>,
{
    fn event(
        _: &mut D,
        _: &WpViewporter,
        _: <WpViewporter as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("wp_viewporter has no events")
    }
}

/// The data assoctiated with the subsurface.
#[derive(Debug)]
pub struct Viewport {
    /// The surface used when creating this subsurface.
    surface: WlSurface,
}

impl Viewport {
    pub(crate) fn new(surface: WlSurface) -> Self {
        Self { surface }
    }

    /// Get the surface used when creating the given viewport.
    pub fn surface(&self) -> &WlSurface {
        &self.surface
    }

    /*
    /// Set the source rectangle.
    pub fn set_source(&self, x: f64, y: f64, width: f64, height: f64) {
        self.viewport.set_source(x, y, width, height);
    }

    /// Set the destination size.
    pub fn set_destination(&self, width: i32, height: i32) {
        self.viewport.set_destination(width, height);
    }
    */
}

#[macro_export]
macro_rules! delegate_viewporter {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter: $crate::globals::GlobalData
            ] => $crate::viewporter::Viewporter
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
        [
            $crate::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport: $crate::viewporter::Viewport
        ] => $crate::viewporter::Viewporter
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, subsurface: [$($subsurface: ty),*$(,)?]) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter: $crate::globals::GlobalData
            ] => $crate::viewporter::Viewporter
        );
        $(
            $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                    $crate::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport: $subsurface
            ] => $crate::viewporter::Viewporter
            );
        )*
    };
}
