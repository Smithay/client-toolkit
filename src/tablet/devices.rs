use super::{
    tool::{tablet_tool_cb, HardwareIdWacom, HardwareSerial, ToolMetaData},
    ListenerData, TabletInner,
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Mutex,
};
use wayland_client::{protocol::wl_seat, Main};
use wayland_client::{Attached, DispatchData};
use wayland_protocols::unstable::tablet::v2::client::*;

/// Callback to get informed about new devices being added to a seat
pub type DeviceCallback =
    dyn FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static;

#[derive(Clone)]
pub struct TabletMetaData {
    pub name: String,
    pub vid: u32,
    pub pid: u32,
    pub path: String,
}

pub enum TabletDeviceEvent {
    ToolAdded { tool: Attached<zwp_tablet_tool_v2::ZwpTabletToolV2> },
    ToolRemoved { tool: zwp_tablet_tool_v2::ZwpTabletToolV2 },
    PadAdded { pad: Attached<zwp_tablet_pad_v2::ZwpTabletPadV2> },
    PadRemoved { pad: zwp_tablet_pad_v2::ZwpTabletPadV2 },
    TabletAdded { tablet: Attached<zwp_tablet_v2::ZwpTabletV2> },
    TabletRemoved { tablet: zwp_tablet_v2::ZwpTabletV2 },
}

impl Default for TabletMetaData {
    fn default() -> Self {
        TabletMetaData { name: "Default".into(), vid: 0, pid: 0, path: "".into() }
    }
}

pub(super) fn tablet_seat_cb(
    tablet_seat: Main<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    listener_data: Rc<RefCell<ListenerData>>,
    event: zwp_tablet_seat_v2::Event,
) {
    match event {
        zwp_tablet_seat_v2::Event::ToolAdded { id } => {
            // set callback for tool events
            println!("Tool added {:?}", id);
            id.as_ref().user_data().set(|| Mutex::new(ToolMetaData::default()));
            id.quick_assign(move |tool, event, ddata| {
                tablet_tool_cb(
                    tablet_seat.clone().into(),
                    tool,
                    listener_data.clone(),
                    event,
                    ddata,
                );
            })
        }
        zwp_tablet_seat_v2::Event::TabletAdded { id } => {
            println!("Tablet added {:?}", id);
            id.as_ref().user_data().set(|| Mutex::new(TabletMetaData::default()));
            id.quick_assign(move |tablet, event, ddata| {
                tablet_tablet_cb(
                    tablet_seat.clone().into(),
                    tablet,
                    listener_data.clone(),
                    event,
                    ddata,
                )
            })
        }
        zwp_tablet_seat_v2::Event::PadAdded { id } => {
            println!("Pad added {:?}", id);
        }
        _ => {}
    }
}

fn tablet_tablet_cb(
    tablet_seat: Attached<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    tablet_device: Main<zwp_tablet_v2::ZwpTabletV2>,
    listener_data: Rc<RefCell<ListenerData>>,
    event: zwp_tablet_v2::Event,
    mut ddata: DispatchData,
) {
    match event {
        zwp_tablet_v2::Event::Name { name } => {
            let tablet_data =
                tablet_device.as_ref().user_data().get::<Mutex<TabletMetaData>>().unwrap();
            let mut guard = tablet_data.lock().unwrap();
            guard.name = name;
        }
        zwp_tablet_v2::Event::Path { path } => {
            let tablet_data =
                tablet_device.as_ref().user_data().get::<Mutex<TabletMetaData>>().unwrap();
            let mut guard = tablet_data.lock().unwrap();
            guard.path = path;
        }
        zwp_tablet_v2::Event::Id { vid, pid } => {
            let tablet_data =
                tablet_device.as_ref().user_data().get::<Mutex<TabletMetaData>>().unwrap();
            let mut guard = tablet_data.lock().unwrap();
            guard.vid = vid;
            guard.pid = pid;
        }
        zwp_tablet_v2::Event::Done => {
            let mut shared_data = listener_data.borrow_mut();
            let wl_seat = shared_data.lookup(&*tablet_seat).map(Attached::clone);
            match wl_seat {
                Some(wseat) => {
                    shared_data.device_listeners.update().iter().for_each(|cb| {
                        (&mut *cb.borrow_mut())(
                            wseat.clone(),
                            TabletDeviceEvent::TabletAdded { tablet: tablet_device.clone().into() },
                            ddata.reborrow(),
                        );
                    });
                }
                None => {}
            }
        }
        zwp_tablet_v2::Event::Removed => {
            let mut shared_data = listener_data.borrow_mut();
            let wl_seat = shared_data.lookup(&*tablet_seat).map(Attached::clone);
            match wl_seat {
                Some(wseat) => {
                    shared_data.device_listeners.update().iter().for_each(|cb| {
                        (&mut *cb.borrow_mut())(
                            wseat.clone(),
                            TabletDeviceEvent::TabletRemoved { tablet: tablet_device.detach() },
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
