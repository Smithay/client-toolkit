use std::collections::HashMap;

use smithay_client_toolkit::{
    output::{OutputData, OutputDispatch, OutputHandler, OutputInfo, OutputState},
    registry::{RegistryDispatch, RegistryHandle, RegistryHandler},
};
use wayland_client::{
    delegate_dispatch,
    protocol::{wl_output, wl_registry},
    Connection,
};
use wayland_protocols::unstable::xdg_output::v1::client::{zxdg_output_manager_v1, zxdg_output_v1};

struct ListOutputs {
    inner: InnerApp,
    registry_handle: RegistryHandle,
    output_state: OutputState,
}

struct InnerApp {
    outputs: HashMap<u32, OutputInfo>,
}

impl OutputHandler for InnerApp {
    fn new_output(&mut self, info: OutputInfo) {
        self.outputs.insert(info.id, info);
    }

    fn update_output(&mut self, info: OutputInfo) {
        self.outputs.insert(info.id, info);
    }

    fn output_destroyed(&mut self, info: OutputInfo) {
        self.outputs.remove(&info.id);
    }
}

delegate_dispatch!(ListOutputs: <UserData = OutputData> [wl_output::WlOutput, zxdg_output_v1::ZxdgOutputV1] => OutputDispatch<'_, InnerApp> ; |app| {
    &mut OutputDispatch(&mut app.output_state, &mut app.inner)
});

delegate_dispatch!(ListOutputs: <UserData = ()> [zxdg_output_manager_v1::ZxdgOutputManagerV1] => OutputDispatch<'_, InnerApp> ; |app| {
    &mut OutputDispatch(&mut app.output_state, &mut app.inner)
});

delegate_dispatch!(ListOutputs: <UserData = ()> [wl_registry::WlRegistry] => RegistryDispatch<'_, ListOutputs> ; |app| {
    let handles: Vec<&mut dyn RegistryHandler<ListOutputs>> = vec![&mut app.output_state];

    &mut RegistryDispatch(&mut app.registry_handle, handles)
});

fn main() {
    let cx = Connection::connect_to_env().unwrap();

    let display = cx.handle().display();

    let mut event_queue = cx.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&mut cx.handle(), &qh, ()).unwrap();

    let mut list_outputs = ListOutputs {
        inner: InnerApp { outputs: HashMap::new() },
        registry_handle: RegistryHandle::new(registry),
        output_state: OutputState::new(),
    };
    event_queue.blocking_dispatch(&mut list_outputs).unwrap();
    event_queue.blocking_dispatch(&mut list_outputs).unwrap();

    for output in list_outputs.inner.outputs.values() {
        print_output(output);
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
