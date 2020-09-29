use wayland_protocols::unstable::tablet::v2::client::*;

use crate::environment;
//use devices::{tablet_seat_cb, DeviceCallback, TabletDeviceEvent};
//use pad::PadCallback;
use std::{
    cell::RefCell,
    cmp,
    rc::{Rc, Weak},
    result::Result,
    sync::Mutex,
};
/*use tool::{
    tablet_tool_cb, HardwareIdWacom, HardwareSerial, ToolCallback, ToolEvent, ToolMetaData,
};*/
use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Attached, DispatchData, Main,
};
use tool::{ToolCallback, ToolCb};
use devices::{DeviceCallback, DeviceCb, TabletDeviceEvent, tablet_seat_cb};

pub mod devices;
//pub mod pad;
pub mod tool;
pub(crate) struct Listeners<C: ?Sized> {
    callbacks: Vec<Weak<RefCell<Box<C>>>>,
}

pub(crate) struct CallbackHandle<C: ?Sized> {
    _cb: Rc<RefCell<Box<C>>>,
}

/// Handles tablet device added/removed events
pub struct TabletHandler {
    tablet_manager: Option<Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>>,
    tablet_seats: Vec<TabletSeat>,
}

struct ListenerData {
    device_listeners: Listeners<DeviceCb>,
    tool_listeners: Listeners<ToolCb>,
}

/// Tablet seat corresponding to a wayland seat
#[derive(Clone)]
pub struct TabletSeat {
    wl_seat: Attached<wl_seat::WlSeat>,
    zwp_tablet_seat: Attached<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    //pads,tablets
    listener_data: Rc<RefCell<ListenerData>>,
}

pub trait TabletHandling {
    fn get_tablet_seat(&mut self, seat: &Attached<wl_seat::WlSeat>) -> TabletSeat;
    fn get_tablet_manager(&self) -> zwp_tablet_manager_v2::ZwpTabletManagerV2;
}

impl TabletSeat {
    fn process_tool_events(&self, mut f: impl ToolCallback) -> CallbackHandle<ToolCb> {
        let rc = Rc::new(RefCell::new(Box::new(f) as Box<ToolCb>));
        self.listener_data.borrow_mut().tool_listeners.push(&rc);
        CallbackHandle { _cb: rc }
    }
    fn process_device_events(&self, mut f: impl DeviceCallback) -> CallbackHandle<DeviceCb> {
        let s = self.wl_seat.clone();
        let callback = move |seat: Attached<wl_seat::WlSeat>, event: TabletDeviceEvent, ddata: DispatchData| {
            if seat == s {
                f.callback(event, ddata);
            }
        };
        let rc = Rc::new(RefCell::new(Box::new(callback) as Box<DeviceCb>));
        self.listener_data.borrow_mut().device_listeners.push(&rc);
        CallbackHandle{_cb: rc}
    }
    fn new(wl_seat: Attached<wl_seat::WlSeat>, zwp_tablet_seat: Attached<zwp_tablet_seat_v2::ZwpTabletSeatV2>) -> TabletSeat {
        let listener_data = Rc::new(RefCell::new(ListenerData::new()));
        TabletSeat{wl_seat, zwp_tablet_seat, listener_data}
    }
}

impl ListenerData {
    fn new() -> ListenerData{
        ListenerData{tool_listeners: Listeners::new(), device_listeners: Listeners::new()}
    }
}

impl<C: ?Sized> Listeners<C> {
    pub(super) fn update(&mut self) -> Vec<Rc<RefCell<Box<C>>>> {
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

    pub(super) fn push(&mut self, callback: &Rc<RefCell<Box<C>>>) {
        self.callbacks.push(Rc::downgrade(callback))
    }

    pub(super) fn new() -> Self {
        Self { callbacks: Vec::new() }
    }

    pub(super) fn invoke_all<F>(&mut self, f: F)
    where
        F: FnMut(&Rc<RefCell<Box<C>>>),
    {
        self.update().iter().for_each(f)
    }
}

impl<C: ?Sized> Clone for Listeners<C>{
    fn clone(&self) -> Self {
        Self{callbacks: self.callbacks.clone()}
    }    
}

impl TabletHandling for TabletHandler {
    fn get_tablet_seat(&mut self, seat: &Attached<wl_seat::WlSeat>) -> TabletSeat {
        let find_seat = (&mut self.tablet_seats).into_iter().find(move |iter_seat| 
        iter_seat.wl_seat == *seat);
        match find_seat {
            Some(found_seat) => found_seat.clone(),
            None => {
                let zwp_tablet_seat = self.tablet_manager.as_ref().unwrap().get_tablet_seat(seat);
                let wl_seat = seat.clone();
                let mut tablet_seat = TabletSeat::new(wl_seat, zwp_tablet_seat.clone().into());
                let listener_data = tablet_seat.listener_data.clone();
                zwp_tablet_seat.quick_assign(move |zwp_tab_seat, event, _| {
                    tablet_seat_cb(zwp_tab_seat, listener_data.clone(), event);
                });
                self.tablet_seats.push(tablet_seat.clone());
                tablet_seat
            }
        }
    }
    fn get_tablet_manager(&self) -> zwp_tablet_manager_v2::ZwpTabletManagerV2 {
        self.tablet_manager.as_ref().unwrap().detach().clone()
    }
}

impl environment::GlobalHandler<zwp_tablet_manager_v2::ZwpTabletManagerV2> for TabletHandler {
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        _: DispatchData,
    ) {
        let version = cmp::min(version, 1);
        let manager = registry.bind::<zwp_tablet_manager_v2::ZwpTabletManagerV2>(version, id);
        self.tablet_manager = Some(manager.into());
    }
    fn get(&self) -> Option<Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>> {
        self.tablet_manager.clone()
    }
}
