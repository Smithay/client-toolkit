use wayland_client::{
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, QueueHandle,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

use crate::registry::{ProvidesRegistryState, RegistryHandler};

use super::{
    LayerHandler, LayerState, LayerSurface, LayerSurfaceConfigure, LayerSurfaceData, SurfaceKind,
};

impl<D> RegistryHandler<D> for LayerState
where
    D: Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, UserData = ()>
        + LayerHandler
        + ProvidesRegistryState
        + 'static,
{
    fn new_global(
        data: &mut D,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        if interface == "zwlr_layer_shell_v1" {
            if data.layer_state().wlr_layer_shell.is_some() {
                return;
            }

            data.layer_state().wlr_layer_shell = Some(
                data.registry()
                    .bind_once::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                        conn,
                        qh,
                        name,
                        u32::min(version, 4),
                        (),
                    )
                    .expect("failed to bind wlr layer shell"),
            );
        }
    }

    fn remove_global(_: &mut D, _: &mut ConnectionHandle, _: &QueueHandle<D>, _: u32) {
        // Unlikely to ever occur and the surfaces become inert if this happens.
    }
}

impl DelegateDispatchBase<zwlr_layer_shell_v1::ZwlrLayerShellV1> for LayerState {
    type UserData = ();
}

impl<D> DelegateDispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, D> for LayerState
where
    D: Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, UserData = ()> + LayerHandler + 'static,
{
    fn event(
        _: &mut D,
        _: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zwlr_layer_shell_v1 has no events")
    }
}

impl DelegateDispatchBase<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1> for LayerState {
    type UserData = LayerSurfaceData;
}

impl<D> DelegateDispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, D> for LayerState
where
    D: Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, UserData = Self::UserData>
        + LayerHandler
        + 'static,
{
    fn event(
        data: &mut D,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _udata: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                if let Some(layer) = data
                    .layer_state()
                    .surfaces
                    .iter()
                    .find(|layer| match layer.kind() {
                        SurfaceKind::Wlr(wlr) => wlr == surface,
                    })
                    .map(LayerSurface::impl_clone)
                {
                    surface.ack_configure(conn, serial);

                    let configure = LayerSurfaceConfigure { new_size: (width, height) };

                    data.configure(conn, qh, &layer, configure, serial);
                }
            }

            zwlr_layer_surface_v1::Event::Closed => {
                if let Some(layer) = data
                    .layer_state()
                    .surfaces
                    .iter()
                    .find(|layer| match layer.kind() {
                        SurfaceKind::Wlr(wlr) => wlr == surface,
                    })
                    .map(LayerSurface::impl_clone)
                {
                    data.closed(conn, qh, &layer);
                }
            }

            _ => unreachable!(),
        }

        // Perform cleanup of any dropped layers
        data.layer_state().cleanup(conn);
    }
}
