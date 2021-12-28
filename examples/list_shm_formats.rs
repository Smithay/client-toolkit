use smithay_client_toolkit::{
    delegate_shm,
    registry::{RegistryDispatch, RegistryHandle, RegistryHandler},
    shm::ShmState,
};
use wayland_client::{delegate_dispatch, protocol::wl_registry, Connection};

struct ListShmFormats {
    registry_handle: RegistryHandle,
    shm_state: ShmState,
}

fn main() {
    let cx = Connection::connect_to_env().unwrap();

    let display = cx.handle().display();

    let mut event_queue = cx.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&mut cx.handle(), &qh, ()).unwrap();
    let mut list_formats = ListShmFormats {
        registry_handle: RegistryHandle::new(registry),
        shm_state: ShmState::new(),
    };

    event_queue.blocking_dispatch(&mut list_formats).unwrap();
    event_queue.blocking_dispatch(&mut list_formats).unwrap();
    println!("Supported formats:");

    for format in list_formats.shm_state.formats() {
        println!("{:?}", format);
    }
}

delegate_shm!(ListShmFormats ; |app| {
    &mut app.shm_state
});

delegate_dispatch!(ListShmFormats: <UserData = ()> [wl_registry::WlRegistry] => RegistryDispatch<'_, ListShmFormats> ; |app| {
    let handles: Vec<&mut dyn RegistryHandler<ListShmFormats>> = vec![&mut app.shm_state];

    &mut RegistryDispatch(&mut app.registry_handle, handles)
});
