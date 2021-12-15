use smithay_client_toolkit::shm::{ShmDispatch, ShmHandler, ShmState};
use wayland_client::{
    delegate_dispatch,
    protocol::{wl_registry, wl_shm},
    Connection, Dispatch,
};

struct ListShmFormats {
    inner: InnerApp,
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

    let _registry = display.get_registry(&mut cx.handle(), &qh, ()).unwrap();
    let mut list_formats = ListShmFormats { inner: InnerApp, shm_state: ShmState::new() };

    event_queue.blocking_dispatch(&mut list_formats).unwrap();
    println!("Supported formats:");
    event_queue.blocking_dispatch(&mut list_formats).unwrap();
}

delegate_dispatch!(ListShmFormats: <UserData = ()> [wl_shm::WlShm] => ShmDispatch<'_, InnerApp>; |app| {
    &mut ShmDispatch(&mut app.shm_state, &mut app.inner)
});

impl Dispatch<wl_registry::WlRegistry> for ListShmFormats {
    type UserData = ();

    fn event(
        &mut self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &Self::UserData,
        cx: &mut wayland_client::ConnectionHandle,
        qh: &wayland_client::QueueHandle<Self>,
        _: &mut wayland_client::DataInit<'_>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, .. } => {
                match &interface[..] {
                    "wl_shm" => {
                        let wl_shm = registry
                            .bind::<wl_shm::WlShm, _>(cx, name, 1, qh, ())
                            .expect("Failed to bind global");

                        // Very temporary hack.
                        #[allow(deprecated)]
                        self.shm_state.shm_bind(wl_shm);
                        println!("Bind [{}] {} @ v{}", name, &interface, 1);
                    }

                    _ => (),
                }
            }

            wl_registry::Event::GlobalRemove { name: _ } => todo!(),

            _ => unreachable!(),
        }
    }
}
