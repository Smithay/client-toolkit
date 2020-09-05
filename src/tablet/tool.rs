use super::{SharedData, TabletDeviceEvent};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Mutex,
};
use wayland_client::{Attached, DispatchData, Main};
use wayland_protocols::unstable::tablet::v2::client::*;

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
    handler_data: Rc<RefCell<SharedData>>,
    event: zwp_tablet_tool_v2::Event,
    mut ddata: DispatchData,
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
            let mut shared_data = handler_data.borrow_mut();
            let wl_seat = shared_data.lookup(&*tablet_seat).map(Attached::clone);
            match wl_seat {
                Some(wseat) => {
                    shared_data.listeners.update().iter().for_each(|cb| {
                        (&mut *cb.borrow_mut())(
                            wseat.clone(),
                            TabletDeviceEvent::ToolAdded { tool: tablet_tool.clone().into() },
                            ddata.reborrow(),
                        );
                    });
                }
                None => {}
            }
        }
        zwp_tablet_tool_v2::Event::Removed => {
            //emit tool removed event
            let mut shared_data = handler_data.borrow_mut();
            let wl_seat = shared_data.lookup(&*tablet_seat).map(Attached::clone);
            match wl_seat {
                Some(wseat) => {
                    shared_data.listeners.update().iter().for_each(|cb| {
                        (&mut *cb.borrow_mut())(
                            wseat.clone(),
                            TabletDeviceEvent::ToolRemoved { tool: tablet_tool.detach() },
                            ddata.reborrow(),
                        );
                    });
                }
                None => {}
            }
        }
        _ => {}
    }
}
