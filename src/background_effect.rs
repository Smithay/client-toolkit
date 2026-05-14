use wayland_client::{
    globals::GlobalList, protocol::wl_surface, Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1, ext_background_effect_surface_v1,
};

use crate::{dispatch2::Dispatch2, error::GlobalError, globals::GlobalData, registry::GlobalProxy};

#[derive(Debug)]
pub struct BackgroundEffectState {
    manager: GlobalProxy<ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1>,
    capabilities: Option<ext_background_effect_manager_v1::Capability>,
}

impl BackgroundEffectState {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1, GlobalData>
            + 'static,
    {
        let manager = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { manager, capabilities: None }
    }

    /// Capabilities advertised by the compositor.
    ///
    /// Returns `None` if the compositor has not yet advertised capabilities.
    pub fn capabilities(&self) -> Option<ext_background_effect_manager_v1::Capability> {
        self.capabilities
    }

    /// Get `ext_background_effect_surface_v1` for a given `wl_surface`.
    ///
    /// Returns error if `ext_background_effect_manager_v1` global is not present.
    pub fn get_background_effect<D>(
        &self,
        surface: &wl_surface::WlSurface,
        qh: &QueueHandle<D>,
    ) -> Result<ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1, GlobalError>
    where
        D: Dispatch<ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1, GlobalData>
            + 'static,
    {
        Ok(self.manager.get()?.get_background_effect(surface, qh, GlobalData))
    }

    /// The `ext_background_effect_manager_v1` global, if any.
    pub fn ext_background_effect_manager_v1(
        &self,
    ) -> Result<&ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1, GlobalError> {
        self.manager.get()
    }
}

pub trait BackgroundEffectHandler {
    fn background_effect_state(&mut self) -> &mut BackgroundEffectState;

    /// Compositor has advertised background effect capabilities.
    ///
    /// Call [`BackgroundEffectState::capabilities`] to access capabilities.
    fn update_capabilities(&mut self);
}

impl<D> Dispatch2<ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1, D> for GlobalData
where
    D: BackgroundEffectHandler,
{
    fn event(
        &self,
        data: &mut D,
        _manager: &ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
        event: ext_background_effect_manager_v1::Event,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        match event {
            ext_background_effect_manager_v1::Event::Capabilities { flags } => {
                let flags = match flags {
                    WEnum::Value(value) => value,
                    WEnum::Unknown(value) => {
                        ext_background_effect_manager_v1::Capability::from_bits_retain(value)
                    }
                };
                data.background_effect_state().capabilities = Some(flags);
                data.update_capabilities();
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch2<ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1, D>
    for GlobalData
{
    fn event(
        &self,
        _data: &mut D,
        _surface: &ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
        _event: ext_background_effect_surface_v1::Event,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}
