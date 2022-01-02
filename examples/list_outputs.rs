//! Test application to list all available outputs.

use std::marker::PhantomData;

use smithay_client_toolkit::{
    delegate_output, delegate_registry,
    output::{OutputDispatch, OutputHandler, OutputInfo, OutputState},
    registry::RegistryHandle,
};
use wayland_client::{protocol::wl_output, Connection, ConnectionHandle, QueueHandle};

struct ListOutputs {
    inner: InnerApp,
    registry_handle: RegistryHandle,
    output_state: OutputState,
}

struct InnerApp;

// OutputHandler's functions are called as outputs are made available, updated and destroyed.
impl OutputHandler<ListOutputs> for InnerApp {
    fn new_output(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<ListOutputs>,
        _state: &OutputState,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<ListOutputs>,
        _state: &OutputState,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<ListOutputs>,
        _state: &OutputState,
        _output: wl_output::WlOutput,
    ) {
    }
}

delegate_output!(ListOutputs => InnerApp: |app| {
    &mut OutputDispatch(&mut app.output_state, &mut app.inner, PhantomData)
});

// Delegate wl_registry to provide the wl_output globals to OutputState
delegate_registry!(ListOutputs:
    |app| {
        &mut app.registry_handle
    },
    handlers = [
        { &mut OutputDispatch(&mut app.output_state, &mut app.inner, PhantomData) }
    ]
);

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let display = conn.handle().display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&mut conn.handle(), &qh, ()).unwrap();

    let mut list_outputs = ListOutputs {
        inner: InnerApp,

        registry_handle: RegistryHandle::new(registry),
        output_state: OutputState::new(),
    };
    event_queue.blocking_dispatch(&mut list_outputs).unwrap();
    event_queue.blocking_dispatch(&mut list_outputs).unwrap();

    for output in list_outputs.output_state.outputs() {
        print_output(&list_outputs.output_state.info(&output).unwrap());
    }
}

fn print_output(info: &OutputInfo) {
    println!("{}", info.model);

    if let Some(name) = info.name.as_ref() {
        println!("\tname: {}", name);
    }

    if let Some(description) = info.description.as_ref() {
        println!("\tdescription: {}", description);
    }

    println!("\tmake: {}", info.make);
    println!("\tx: {}, y: {}", info.location.0, info.location.1);
    println!("\tsubpixel: {:?}", info.subpixel);
    println!("\tphysical_size: {}×{}mm", info.physical_size.0, info.physical_size.1);
    println!("\tmodes:");

    for mode in &info.modes {
        println!("\t\t{}", mode);
    }
}
