use smithay_client_toolkit::{
    delegate_registry, delegate_shm,
    registry::{ProvidesRegistryState, RegistryState},
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

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&qh, ()).unwrap();
    let mut list_formats =
        ListShmFormats { registry_state: RegistryState::new(registry), shm_state: ShmState::new() };

    event_queue.blocking_dispatch(&mut list_formats).unwrap();
    event_queue.blocking_dispatch(&mut list_formats).unwrap();
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

delegate_registry!(ListShmFormats: [
    ShmState,
]);

impl ProvidesRegistryState for ListShmFormats {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
}
