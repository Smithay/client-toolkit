use drm_fourcc::{DrmFourcc, DrmModifier};
use smithay_client_toolkit::{
    dmabuf::{DmabufFeedback, DmabufFormat, DmabufHandler, DmabufState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{globals::registry_queue_init, protocol::wl_buffer, Connection, QueueHandle};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1, zwp_linux_dmabuf_feedback_v1,
};

struct AppData {
    registry_state: RegistryState,
    dmabuf_state: DmabufState,
    feedback: Option<DmabufFeedback>,
}

impl DmabufHandler for AppData {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_feedback(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _proxy: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        feedback: DmabufFeedback,
    ) {
        self.feedback = Some(feedback);
    }

    fn created(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _params: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
        _buffer: wl_buffer::WlBuffer,
    ) {
    }

    fn failed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _params: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
    ) {
    }

    fn released(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _buffer: &wl_buffer::WlBuffer,
    ) {
    }
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![,];
}

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let mut app_data = AppData {
        registry_state: RegistryState::new(&globals),
        dmabuf_state: DmabufState::new(&globals, &qh),
        feedback: None,
    };

    match app_data.dmabuf_state.version() {
        None => println!("`zwp_linux_dmabuf_v1` version `>3` not supported by compositor."),
        Some(0..=2) => unreachable!(),
        Some(3) => {
            println!("Version `3` of `zwp_linux_dmabuf_v1` supported. Showing modifiers.\n");

            // Roundtrip after binding global to receive modifier events.
            event_queue.roundtrip(&mut app_data).unwrap();

            for entry in app_data.dmabuf_state.modifiers() {
                print_format(entry);
            }
        }
        Some(ver @ 4..) => {
            println!("Version `{}` of `zwp_linux_dmabuf_v1` supported. Showing default dmabuf feedback.\n", ver);

            app_data.dmabuf_state.get_default_feedback(&qh).unwrap();

            let feedback = loop {
                event_queue.blocking_dispatch(&mut app_data).unwrap();
                if let Some(feedback) = app_data.feedback.as_ref() {
                    break feedback;
                }
            };

            println!("Main device: 0x{:x}", feedback.main_device());
            println!("Tranches:");
            let format_table = feedback.format_table();
            for tranche in feedback.tranches() {
                println!("  Device: 0x{:x}", tranche.device);
                println!("  Flags: {:?}", tranche.flags);
                println!("  Formats");
                for idx in &tranche.formats {
                    print!("    ");
                    print_format(&format_table[*idx as usize]);
                }
            }
        }
    }
}

fn print_format(format: &DmabufFormat) {
    print!("Format: ");
    match DrmFourcc::try_from(format.format) {
        Ok(format) => print!("{:?}", format),
        Err(err) => print!("{:?}", err),
    }
    println!(", Modifier: {:?}", DrmModifier::from(format.modifier));
}

smithay_client_toolkit::delegate_dmabuf!(AppData);
smithay_client_toolkit::delegate_registry!(AppData);
