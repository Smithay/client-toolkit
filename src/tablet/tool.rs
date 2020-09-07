use super::{devices::notify_devices, ListenerData, TabletDeviceEvent};
use std::{cell::RefCell, rc::Rc, sync::Mutex};
use wayland_client::{protocol::wl_surface, Attached, DispatchData, Main};
use wayland_protocols::unstable::tablet::v2::client::*;

pub(super) type ToolCallback =
    dyn FnMut(Attached<zwp_tablet_tool_v2::ZwpTabletToolV2>, ToolEvent, DispatchData) + 'static;

#[derive(Clone)]
pub struct HardwareIdWacom {
    pub hardware_id_hi: u32,
    pub hardware_id_lo: u32,
}
#[derive(Clone)]
pub struct HardwareSerial {
    pub hardware_serial_hi: u32,
    pub hardware_serial_lo: u32,
}

#[derive(Clone)]
pub struct ToolMetaData {
    pub capabilities: Vec<zwp_tablet_tool_v2::Capability>,
    pub hardware_id_wacom: HardwareIdWacom,
    pub hardware_serial: HardwareSerial,
    pub tool_type: zwp_tablet_tool_v2::Type,
}

#[derive(Clone)]
pub enum ToolEvent {
    ProximityIn { serial: u32, tablet: zwp_tablet_v2::ZwpTabletV2, surface: wl_surface::WlSurface },
    ProximityOut,
    Down { serial: u32 },
    Up,
    Motion { x: f64, y: f64 },
    Pressure { pressure: u32 },
    Distance { distance: u32 },
    Tilt { tilt_x: f64, tilt_y: f64 },
    Rotation { degrees: f64 },
    Slider { position: i32 },
    Wheel { degrees: f64, clicks: i32 },
    Button { serial: u32, button: u32, state: zwp_tablet_tool_v2::ButtonState },
    Frame { time: u32 },
}

impl Default for ToolMetaData {
    fn default() -> Self {
        ToolMetaData {
            capabilities: Vec::new(),
            hardware_id_wacom: HardwareIdWacom { hardware_id_hi: 0, hardware_id_lo: 0 },
            hardware_serial: HardwareSerial { hardware_serial_hi: 0, hardware_serial_lo: 0 },
            tool_type: zwp_tablet_tool_v2::Type::Pen {},
        }
    }
}

pub(super) fn tablet_tool_cb(
    tablet_seat: Attached<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    tablet_tool: Main<zwp_tablet_tool_v2::ZwpTabletToolV2>,
    listener_data: Rc<RefCell<ListenerData>>,
    event: zwp_tablet_tool_v2::Event,
    ddata: DispatchData,
) {
    println!("Tablet tool event {:?}", event);
    match event {
        zwp_tablet_tool_v2::Event::Type { tool_type } => {
            let tool_data = tablet_tool.as_ref().user_data().get::<Mutex<ToolMetaData>>().unwrap();
            let mut guard = tool_data.lock().unwrap();
            guard.tool_type = tool_type;
        }
        zwp_tablet_tool_v2::Event::HardwareSerial { hardware_serial_hi, hardware_serial_lo } => {
            let hw_id = HardwareSerial { hardware_serial_hi, hardware_serial_lo };
            let tool_data = tablet_tool.as_ref().user_data().get::<Mutex<ToolMetaData>>().unwrap();
            let mut guard = tool_data.lock().unwrap();
            guard.hardware_serial = hw_id;
        }
        zwp_tablet_tool_v2::Event::HardwareIdWacom { hardware_id_hi, hardware_id_lo } => {
            let hw_id = HardwareIdWacom { hardware_id_hi, hardware_id_lo };
            let tool_data = tablet_tool.as_ref().user_data().get::<Mutex<ToolMetaData>>().unwrap();
            let mut guard = tool_data.lock().unwrap();
            guard.hardware_id_wacom = hw_id;
        }
        zwp_tablet_tool_v2::Event::Capability { capability } => {
            let tool_data = tablet_tool.as_ref().user_data().get::<Mutex<ToolMetaData>>().unwrap();
            let mut guard = tool_data.lock().unwrap();
            guard.capabilities.push(capability);
        }
        zwp_tablet_tool_v2::Event::Done => {
            //emit tool added event
            notify_devices(
                &listener_data,
                TabletDeviceEvent::ToolAdded { tool: tablet_tool.clone().into() },
                ddata,
                &tablet_seat,
            )
        }
        zwp_tablet_tool_v2::Event::Removed => {
            //emit tool removed event
            notify_devices(
                &listener_data,
                TabletDeviceEvent::ToolRemoved { tool: tablet_tool.detach() },
                ddata,
                &tablet_seat,
            )
        }
        zwp_tablet_tool_v2::Event::ProximityIn { serial, tablet, surface } => notify_tools(
            &listener_data,
            ToolEvent::ProximityIn { serial, tablet, surface },
            ddata,
            &tablet_tool,
        ),
        zwp_tablet_tool_v2::Event::ProximityOut {} => {
            notify_tools(&listener_data, ToolEvent::ProximityOut, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Up {} => {
            notify_tools(&listener_data, ToolEvent::Up, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Down { serial } => {
            notify_tools(&listener_data, ToolEvent::Down { serial }, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Motion { x, y } => {
            notify_tools(&listener_data, ToolEvent::Motion { x, y }, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Pressure { pressure } => {
            notify_tools(&listener_data, ToolEvent::Pressure { pressure }, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Distance { distance } => {
            notify_tools(&listener_data, ToolEvent::Distance { distance }, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Tilt { tilt_x, tilt_y } => {
            notify_tools(&listener_data, ToolEvent::Tilt { tilt_x, tilt_y }, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Rotation { degrees } => {
            notify_tools(&listener_data, ToolEvent::Rotation { degrees }, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Slider { position } => {
            notify_tools(&listener_data, ToolEvent::Slider { position }, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Wheel { degrees, clicks } => {
            notify_tools(&listener_data, ToolEvent::Wheel { degrees, clicks }, ddata, &tablet_tool)
        }
        zwp_tablet_tool_v2::Event::Button { serial, button, state } => notify_tools(
            &listener_data,
            ToolEvent::Button { serial, button, state },
            ddata,
            &tablet_tool,
        ),
        zwp_tablet_tool_v2::Event::Frame { time } => {
            notify_tools(&listener_data, ToolEvent::Frame { time }, ddata, &tablet_tool)
        }
        _ => {
            println!("Tool event not recognized");
        }
    }
}

fn notify_tools(
    listener_data: &Rc<RefCell<ListenerData>>,
    event: ToolEvent,
    mut ddata: DispatchData,
    tablet_tool: &Attached<zwp_tablet_tool_v2::ZwpTabletToolV2>,
) {
    let mut shared_data = listener_data.borrow_mut();
    shared_data.tool_listeners.invoke_all(move |cb| {
        (&mut *cb.borrow_mut())(tablet_tool.clone(), event.clone(), ddata.reborrow());
    });
}

pub fn clone_tool_data(tablet: &zwp_tablet_tool_v2::ZwpTabletToolV2) -> Option<ToolMetaData> {
    if let Some(ref udata_mutex) = tablet.as_ref().user_data().get::<Mutex<ToolMetaData>>() {
        let udata = udata_mutex.lock().unwrap();
        Some(udata.clone())
    } else {
        None
    }
}

pub fn with_tool_data<T, F: FnOnce(&ToolMetaData) -> T>(
    seat: &zwp_tablet_v2::ZwpTabletV2,
    f: F,
) -> Option<T> {
    if let Some(ref udata_mutex) = seat.as_ref().user_data().get::<Mutex<ToolMetaData>>() {
        let udata = udata_mutex.lock().unwrap();
        Some(f(&*udata))
    } else {
        None
    }
}
