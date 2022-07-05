use wayland_client::{Connection, DelegateDispatch, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::{
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
    registry::{ProvidesRegistryState, RegistryHandler},
};

use super::{LayerHandler, LayerState, LayerSurfaceConfigure, LayerSurfaceData};

impl<D> RegistryHandler<D> for LayerState
where
    D: Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalData>
        + LayerHandler
        + ProvidesRegistryState
        + 'static,
{
    fn ready(data: &mut D, _conn: &Connection, qh: &QueueHandle<D>) {
        data.layer_state().wlr_layer_shell =
            data.registry().bind_one(qh, 1..=4, GlobalData(())).into();
    }
}

// Layer shell has only added requests and enum variants in versions 2-4, so its client-facing API
// is still compatible.
impl ProvidesBoundGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1, 1> for LayerState {
    fn bound_global(&self) -> Result<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalError> {
        self.wlr_layer_shell.get().cloned()
    }
}

impl ProvidesBoundGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1, 2> for LayerState {
    fn bound_global(&self) -> Result<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalError> {
        self.wlr_layer_shell.get().cloned()
    }
}

impl ProvidesBoundGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1, 3> for LayerState {
    fn bound_global(&self) -> Result<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalError> {
        self.wlr_layer_shell.get().cloned()
    }
}

impl ProvidesBoundGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1, 4> for LayerState {
    fn bound_global(&self) -> Result<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalError> {
        self.wlr_layer_shell.get().cloned()
    }
}

impl<D> DelegateDispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalData, D> for LayerState
where
    D: Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalData> + LayerHandler + 'static,
{
    fn event(
        _: &mut D,
        _: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zwlr_layer_shell_v1 has no events")
    }
}

impl<D> DelegateDispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, LayerSurfaceData, D>
    for LayerState
where
    D: Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, LayerSurfaceData>
        + LayerHandler
        + 'static,
{
    fn event(
        data: &mut D,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _udata: &LayerSurfaceData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        // Remove any surfaces that have been dropped
        data.layer_state().surfaces.retain(|surface| surface.upgrade().is_some());

        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                if let Some(layer_surface) = data.layer_state().get_wlr_surface(surface) {
                    surface.ack_configure(serial);

                    let configure = LayerSurfaceConfigure { new_size: (width, height) };

                    data.configure(conn, qh, &layer_surface, configure, serial);
                }
            }

            zwlr_layer_surface_v1::Event::Closed => {
                if let Some(layer_surface) = data.layer_state().get_wlr_surface(surface) {
                    data.closed(conn, qh, &layer_surface);
                }
            }

            _ => unreachable!(),
        }
    }
}
