use std::error::Error;

use smithay_client_toolkit::{
    delegate_foreign_toplevel_list, delegate_registry,
    foreign_toplevel_list::{ForeignToplevelList, ForeignToplevelListHandler},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{globals::registry_queue_init, Connection, QueueHandle};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;

struct State {
    registry_state: RegistryState,
    foreign_toplevel_list: ForeignToplevelList,
}

fn main() -> Result<(), Box<dyn Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();
    let registry_state = RegistryState::new(&globals);
    let foreign_toplevel_list = ForeignToplevelList::new(&globals, &qh);

    let mut state = State { registry_state, foreign_toplevel_list };
    loop {
        event_queue.blocking_dispatch(&mut state)?;
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers! {}
}

impl ForeignToplevelListHandler for State {
    fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelList {
        &mut self.foreign_toplevel_list
    }

    fn new_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel_handle: ExtForeignToplevelHandleV1,
    ) {
        let Some(info) = self.foreign_toplevel_list.info(&toplevel_handle) else {
            return;
        };
        println!("New toplevel: {:?}", info);
    }

    fn update_toplevel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel_handle: ExtForeignToplevelHandleV1,
    ) {
        let Some(info) = self.foreign_toplevel_list.info(&toplevel_handle) else {
            return;
        };
        println!("Update toplevel: {:?}", info);
    }

    fn toplevel_closed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        toplevel_handle: ExtForeignToplevelHandleV1,
    ) {
        let Some(info) = self.foreign_toplevel_list.info(&toplevel_handle) else {
            return;
        };
        println!("Close toplevel: {:?}", info);
    }
}

delegate_foreign_toplevel_list!(State);
delegate_registry!(State);
