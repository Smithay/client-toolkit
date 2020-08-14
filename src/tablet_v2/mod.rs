use wayland_protocols::unstable::tablet::v2::client::{
    zwp_tablet_manager_v2, zwp_tablet_seat_v2, zwp_tablet_v2,
};

use crate::environment;
use std::{
    cell::RefCell,
    cmp,
    rc::{Rc, Weak},
    sync::Mutex,
};
use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Attached, DispatchData, Main,
};

type TabletManagerCallback = dyn FnMut(Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>, &TabletManagerData, DispatchData)
    + 'static;

pub struct TabletManagerHandler {
    managers: Vec<(u32, Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>)>,
    listeners: Rc<RefCell<Vec<Weak<RefCell<TabletManagerCallback>>>>>,
}

pub struct TabletManagerData {}

impl TabletManagerData {
    pub fn new() -> TabletManagerData {
        TabletManagerData {}
    }
}

impl environment::MultiGlobalHandler<zwp_tablet_manager_v2::ZwpTabletManagerV2>
    for TabletManagerHandler
{
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        _: DispatchData,
    ) {
        let version = cmp::min(version, 1);
        let manager = registry.bind::<zwp_tablet_manager_v2::ZwpTabletManagerV2>(version, id);
        manager.as_ref().user_data().set_threadsafe(|| Mutex::new(TabletManagerData::new()));
        let cb_listeners = self.listeners.clone();
        manager.quick_assign(move |manager, event, ddata| {
            process_manager_event(manager, event, &cb_listeners, ddata)
        });
        self.managers.push((id, (*manager).clone()));
    }
    fn removed(&mut self, id: u32, ddata: DispatchData) {
        todo!()
    }
    fn get_all(&self) -> Vec<Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>> {
        todo!()
    }
}

fn process_manager_event(
    manager: Main<zwp_tablet_manager_v2::ZwpTabletManagerV2>,
    event: zwp_tablet_manager_v2::Event,
    listeners: &RefCell<Vec<Weak<RefCell<TabletManagerCallback>>>>,
    mut ddata: DispatchData,
) {
    
}
