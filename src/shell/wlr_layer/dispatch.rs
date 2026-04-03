use wayland_client::{Connection, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::{
    dispatch2::Dispatch2,
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
};

use super::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure, LayerSurfaceData};

// Layer shell has only added requests and enum variants in versions 2-4, so its client-facing API
// is still compatible.
impl ProvidesBoundGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1, 1> for LayerShell {
    fn bound_global(&self) -> Result<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalError> {
        Ok(self.wlr_layer_shell.clone())
    }
}

impl ProvidesBoundGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1, 2> for LayerShell {
    fn bound_global(&self) -> Result<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalError> {
        Ok(self.wlr_layer_shell.clone())
    }
}

impl ProvidesBoundGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1, 3> for LayerShell {
    fn bound_global(&self) -> Result<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalError> {
        Ok(self.wlr_layer_shell.clone())
    }
}

impl ProvidesBoundGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1, 4> for LayerShell {
    fn bound_global(&self) -> Result<zwlr_layer_shell_v1::ZwlrLayerShellV1, GlobalError> {
        Ok(self.wlr_layer_shell.clone())
    }
}

impl<D> Dispatch2<zwlr_layer_shell_v1::ZwlrLayerShellV1, D> for GlobalData
where
    D: LayerShellHandler + 'static,
{
    fn event(
        &self,
        _: &mut D,
        _: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zwlr_layer_shell_v1 has no events")
    }
}

impl<D> Dispatch2<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, D> for LayerSurfaceData
where
    D: LayerShellHandler + 'static,
{
    fn event(
        &self,
        data: &mut D,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        if let Some(layer_surface) = LayerSurface::from_wlr_surface(surface) {
            match event {
                zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                    surface.ack_configure(serial);

                    let configure = LayerSurfaceConfigure { new_size: (width, height) };
                    data.configure(conn, qh, &layer_surface, configure, serial);
                }

                zwlr_layer_surface_v1::Event::Closed => {
                    data.closed(conn, qh, &layer_surface);
                }

                _ => unreachable!(),
            }
        }
    }
}
