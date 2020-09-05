use super::tool::{tablet_tool_cb, HardwareIdWacom, HardwareSerial, ToolMetaData};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Mutex,
};
use wayland_client::{protocol::wl_seat, Main};
use wayland_client::{Attached, DispatchData};
use wayland_protocols::unstable::tablet::v2::client::*;

/// Callback to get informed about new devices being added to a seat
pub type TabletDeviceCallback =
    dyn FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static;

pub(crate) struct Listeners<C: ?Sized> {
    callbacks: Vec<Weak<RefCell<C>>>,
}

pub(super) struct SharedData {
    pub tablet_seats: Vec<(Attached<wl_seat::WlSeat>, zwp_tablet_seat_v2::ZwpTabletSeatV2)>,
    /// Global callback for new tablet devices
    pub listeners: Listeners<TabletDeviceCallback>,
}
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

impl<C: ?Sized> Listeners<C> {
    pub(super) fn update(&mut self) -> Vec<Rc<RefCell<C>>> {
        let mut vector = Vec::new();
        self.callbacks.retain(|lst| {
            if let Some(cb) = Weak::upgrade(lst) {
                vector.push(cb);
                true
            } else {
                false
            }
        });
        vector
    }

    pub(super) fn push(&mut self, callback: &Rc<RefCell<C>>) {
        self.callbacks.push(Rc::downgrade(callback))
    }

    pub(super) fn new() -> Self {
        Self { callbacks: Vec::new() }
    }
}

impl Default for TabletMetaData {
    fn default() -> Self {
        TabletMetaData { name: "Default".into(), vid: 0, pid: 0, path: "".into() }
    }
}

impl SharedData {
    pub(super) fn lookup(
        &self,
        tablet_seat: &zwp_tablet_seat_v2::ZwpTabletSeatV2,
    ) -> Option<&Attached<wl_seat::WlSeat>> {
        self.tablet_seats.iter().find(|(_, tseat)| *tseat == *tablet_seat).map(|(wseat, _)| wseat)
    }
}

pub(super) fn tablet_seat_cb(
    tablet_seat: Main<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    handler_data: Rc<RefCell<SharedData>>,
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
                    handler_data.clone(),
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
                    handler_data.clone(),
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
    handler_data: Rc<RefCell<SharedData>>,
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
            let mut shared_data = handler_data.borrow_mut();
        }
        zwp_tablet_v2::Event::Removed => {}
        _ => {}
    }
}
