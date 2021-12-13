use std::collections::HashMap;

use smithay_client_toolkit::output::{OutputData, OutputHandler, OutputInfo, OutputState};
use wayland_client::{
    protocol::{wl_output, wl_registry},
    Connection, ConnectionHandle, DataInit, Dispatch, QueueHandle, delegate_dispatch,
};
use wayland_protocols::unstable::xdg_output::v1::client::zxdg_output_manager_v1;

struct ListOutputs {
    inner: InnerApp,
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

impl Dispatch<wl_registry::WlRegistry> for ListOutputs {
    type UserData = ();

    fn event(
        &mut self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &Self::UserData,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        _init: &mut DataInit<'_>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } => match &interface[..] {
                "wl_output" => {
                    let output = registry
                        .bind::<wl_output::WlOutput, _>(
                            cx,
                            name,
                            u32::min(version, 4),
                            qh,
                            OutputData::new(),
                        )
                        .expect("Failed to bind global");

                    // Very temporary hack.
                    #[allow(deprecated)]
                    self.output_state.new_output(cx, qh, name, output);

                    println!("Bind [{}] {} @ v{}", name, &interface, version);
                }

                "zxdg_output_manager_v1" => {
                    let manager = registry
                        .bind::<zxdg_output_manager_v1::ZxdgOutputManagerV1, _>(
                            cx,
                            name,
                            u32::min(version, 3),
                            qh,
                            (),
                        )
                        .expect("Failed to bind globals");

                    // Very temporary hack.
                    #[allow(deprecated)]
                    self.output_state.zxdg_manager_bound(cx, qh, manager);

                    println!("Bind zxdg manager");
                }

                _ => (),
            },

            wl_registry::Event::GlobalRemove { name: _ } => todo!(),

            _ => unreachable!(),
        }
    }
}

delegate_dispatch!(ListOutputs: <UserData = OutputData> [wl_output::WlOutput, zxdg_output_v1::ZxdgOutputV1] => OutputDispatch<'_, InnerApp> ; |app| {
    &mut OutputDispatch(&mut app.output_state, &mut app.inner)
});

delegate_dispatch!(ListOutputs: <UserData = ()> [zxdg_output_manager_v1::ZxdgOutputManagerV1] => OutputDispatch<'_, InnerApp> ; |app| {
    &mut OutputDispatch(&mut app.output_state, &mut app.inner)
});

fn main() {
    let cx = Connection::connect_to_env().unwrap();

    let display = cx.handle().display();

    let mut event_queue = cx.new_event_queue();
    let qh = event_queue.handle();

    let _registry = display.get_registry(&mut cx.handle(), &qh, ()).unwrap();

    let mut list_outputs = ListOutputs {
        inner: InnerApp { outputs: HashMap::new() },
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
