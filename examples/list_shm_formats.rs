use smithay_client_toolkit::{
    registry::{RegistryDispatch, RegistryHandle, RegistryHandler},
    shm::{ShmDispatch, ShmHandler, ShmState},
};
use wayland_client::{
    delegate_dispatch,
    protocol::{wl_registry, wl_shm},
    Connection,
};

struct ListShmFormats {
    inner: InnerApp,
    registry_handle: RegistryHandle,
    shm_state: ShmState,
}

struct InnerApp;

impl ShmHandler for InnerApp {
    fn supported_format(&mut self, format: wl_shm::Format) {
        println!("{:?}", format);
    }
}

fn main() {
    let cx = Connection::connect_to_env().unwrap();

    let display = cx.handle().display();

    let mut event_queue = cx.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&mut cx.handle(), &qh, ()).unwrap();
    let mut list_formats = ListShmFormats {
        inner: InnerApp,
        registry_handle: RegistryHandle::new(registry),
        shm_state: ShmState::new(),
    };

    event_queue.blocking_dispatch(&mut list_formats).unwrap();
    println!("Supported formats:");
    event_queue.blocking_dispatch(&mut list_formats).unwrap();
}

delegate_dispatch!(ListShmFormats: <UserData = ()> [wl_shm::WlShm] => ShmDispatch<'_, InnerApp>; |app| {
    &mut ShmDispatch(&mut app.shm_state, &mut app.inner)
});

delegate_dispatch!(ListShmFormats: <UserData = ()> [wl_registry::WlRegistry] => RegistryDispatch<'_, ListShmFormats> ; |app| {
    let handles: Vec<&mut dyn RegistryHandler<ListShmFormats>> = vec![&mut app.shm_state];

    &mut RegistryDispatch(&mut app.registry_handle, handles)
});
