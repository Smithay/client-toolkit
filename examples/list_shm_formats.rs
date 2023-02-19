/// Example app showing how to use delegate types from Smithay's client toolkit and initializing state.
use smithay_client_toolkit::{
    delegate_shm,
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::wl_registry,
    Connection, Dispatch, QueueHandle,
};

struct ListShmFormats {
    shm_state: Shm,
}

fn main() {
    // Initialize logging for Rust backend.
    env_logger::init();

    // Connect to the compositor.
    let conn = Connection::connect_to_env().unwrap();

    // Create an event queue and get the initial global list.
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    // Create the state to dispatch.
    let mut list_formats = ListShmFormats {
        // Bind ShmState to implement wl_shm handling.
        shm_state: Shm::bind(&globals, &qh).expect("wl_shm is not available"),
    };

    // Roundtrip to get the supported wl_shm formats.
    event_queue.roundtrip(&mut list_formats).unwrap();
    println!("Supported formats:");

    for format in list_formats.shm_state.formats() {
        println!("{format:?}");
    }
}

impl ShmHandler for ListShmFormats {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

// Delegate handling of the wl_shm protocol to our state object.
delegate_shm!(ListShmFormats);

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for ListShmFormats {
    fn event(
        _state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We don't need any other globals.
    }
}
