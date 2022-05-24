use smithay_client_toolkit::{
    delegate_registry, delegate_shm,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shm::{ShmHandler, ShmState},
};
use wayland_client::Connection;

struct ListShmFormats {
    registry_state: RegistryState,
    shm_state: ShmState,
}

fn main() {
    env_logger::init();
    let conn = Connection::connect_to_env().unwrap();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut list_formats = ListShmFormats {
        registry_state: RegistryState::new(&conn, &qh),
        shm_state: ShmState::new(),
    };

    while !list_formats.registry_state.ready() {
        event_queue.blocking_dispatch(&mut list_formats).unwrap();
    }
    event_queue.sync_roundtrip(&mut list_formats).unwrap();
    println!("Supported formats:");

    for format in list_formats.shm_state.formats() {
        println!("{:?}", format);
    }
}

impl ShmHandler for ListShmFormats {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
    }
}

delegate_shm!(ListShmFormats);

delegate_registry!(ListShmFormats);

impl ProvidesRegistryState for ListShmFormats {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(ShmState);
}
