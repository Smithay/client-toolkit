use wayland_protocols::unstable::tablet::v2::client::*;

use crate::environment;
use devices::{tablet_seat_cb, DeviceCallback, TabletDeviceEvent};
//use pad::PadCallback;
use std::{
    cell::RefCell,
    cmp,
    rc::{Rc, Weak},
    result::Result,
    sync::Mutex,
};
use tool::{
    tablet_tool_cb, HardwareIdWacom, HardwareSerial, ToolCallback, ToolEvent, ToolListener,
    ToolMetaData,
};
use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Attached, DispatchData, Main,
};

pub mod devices;
//pub mod pad;
pub mod tool;

pub(crate) struct Listeners<C: ?Sized> {
    callbacks: Vec<Weak<RefCell<C>>>,
}

pub struct TabletDeviceListener {
    _cb: Rc<RefCell<DeviceCallback>>,
}

/// Contains TabletManager mapping seats to tablet seats
enum TabletInner {
    Ready {
        mgr: Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>,
        listener_data: Rc<RefCell<ListenerData>>,
    },
    Pending {
        seats: Vec<Attached<wl_seat::WlSeat>>,
    },
}
struct ListenerData {
    tablet_seats: Vec<(Attached<wl_seat::WlSeat>, zwp_tablet_seat_v2::ZwpTabletSeatV2)>,
    device_listeners: Listeners<DeviceCallback>,
    tool_listeners: Listeners<ToolCallback>,
    //pad_listeners: Listeners<PadCallback>,
}

/// Handles tablet device added/removed events
pub struct TabletHandler {
    inner: Rc<RefCell<TabletInner>>,
    _listener: crate::seat::SeatListener,
}

pub trait TabletHandling {
    /// Set global callback for new tablet devices being added/removed to a seat
    fn listen<F: FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<TabletDeviceListener, ()>;
    fn process_tool_events<
        F: FnMut(Attached<zwp_tablet_tool_v2::ZwpTabletToolV2>, ToolEvent, DispatchData) + 'static,
    >(
        &mut self,
        seat: Attached<wl_seat::WlSeat>,
        f: F,
    ) -> Result<ToolListener, ()>;
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

    pub(super) fn invoke_all<F>(&mut self, f: F)
    where
        F: FnMut(&Rc<RefCell<C>>),
    {
        self.update().iter().for_each(f)
    }
}

impl TabletHandling for TabletHandler {
    fn listen<F: FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<TabletDeviceListener, ()> {
        let rc = Rc::new(RefCell::new(callback)) as Rc<_>;
        let ref mut inner = *self.inner.borrow_mut();
        match inner {
            TabletInner::Ready { listener_data, .. } => {
                listener_data.borrow_mut().device_listeners.push(&rc);
                Ok(TabletDeviceListener { _cb: rc })
            }
            TabletInner::Pending { .. } => Err(()),
        }
    }
    fn process_tool_events<
        F: FnMut(Attached<zwp_tablet_tool_v2::ZwpTabletToolV2>, ToolEvent, DispatchData) + 'static,
    >(
        &mut self,
        seat: Attached<wl_seat::WlSeat>,
        f: F,
    ) -> Result<ToolListener, ()> {
        let mut closure = move |wseat: Attached<wl_seat::WlSeat>, tool, tool_event, ddata| {
            if (seat == wseat) {
                f(tool, tool_event, ddata);
            }
        };
        let rc = Rc::new(RefCell::new(closure as ToolCallback)) as Rc<_>;
        let ref mut inner = *self.inner.borrow_mut();
        match inner {
            TabletInner::Ready { listener_data, .. } => {
                listener_data.borrow_mut().tool_listeners.push(&rc);
                Ok(ToolListener { _cb: rc })
            }
            TabletInner::Pending { .. } => Err(()),
        }
    }
}

impl ListenerData {
    fn new() -> Self {
        let tablet_seats = Vec::new();
        let device_listeners = Listeners::new();
        let tool_listeners = Listeners::new();
        //let pad_listeners = Listeners::new();
        Self { tablet_seats, device_listeners, tool_listeners /*, pad_listeners */ }
    }

    fn lookup(
        &self,
        tablet_seat: &zwp_tablet_seat_v2::ZwpTabletSeatV2,
    ) -> &Attached<wl_seat::WlSeat> {
        self.tablet_seats
            .iter()
            .find(|(_, tseat)| *tseat == *tablet_seat)
            .map(|(wseat, _)| wseat)
            .expect("Tablet seat not found in mapping")
    }
}

impl TabletInner {
    fn init_tablet_mgr(&mut self, mgr: Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>) {
        let seats = if let TabletInner::Pending { seats } = self {
            std::mem::replace(seats, Vec::new())
        } else {
            log::warn!("Ignoring second zwp_tablet_manager_v2");
            return;
        };

        let listener_data = Rc::new(RefCell::new(ListenerData::new()));

        for seat in seats {
            let tablet_seat = mgr.get_tablet_seat(&seat);
            // attach tablet seat to global callback for new devices
            let dclone = listener_data.clone();
            tablet_seat.quick_assign(move |t_seat, evt, _| {
                tablet_seat_cb(t_seat, dclone.clone(), evt);
            });
            listener_data.borrow_mut().tablet_seats.push((seat, tablet_seat.detach()));
        }

        *self = Self::Ready { mgr, listener_data };
    }
    fn get_mgr(&self) -> Option<Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>> {
        match self {
            Self::Ready { mgr, .. } => Some(mgr.clone()),
            Self::Pending { .. } => None,
        }
    }
    // A potential new seat is seen
    //
    // should do nothing if the seat is already known
    fn new_seat(&mut self, seat: &Attached<wl_seat::WlSeat>) {
        match self {
            Self::Ready { mgr, listener_data } => {
                let mut datamut = listener_data.borrow_mut();
                if datamut.tablet_seats.iter().any(|(s, _)| *s == *seat) {
                    // the seat already exists, nothing to do
                    return;
                }
                let tablet_seat = mgr.get_tablet_seat(seat);
                datamut.tablet_seats.push((seat.clone(), tablet_seat.detach()));
                // attach tablet seat to global callback for new devices
                let dclone = listener_data.clone();
                tablet_seat.quick_assign(move |t_seat, evt, ddata| {
                    tablet_seat_cb(t_seat, dclone.clone(), evt)
                });
            }
            Self::Pending { seats } => {
                seats.push(seat.clone());
            }
        }
    }

    fn remove_seat(&mut self, seat: &Attached<wl_seat::WlSeat>) {
        match self {
            Self::Ready { listener_data, .. } => {
                listener_data.borrow_mut().tablet_seats.retain(|(s, _)| s != seat)
            }
            Self::Pending { seats } => seats.retain(|s| s != seat),
        }
    }
}

impl TabletHandler {
    /// Initialize TabletHandler
    pub fn init<S>(seat_handler: &mut S) -> TabletHandler
    where
        S: crate::seat::SeatHandling,
    {
        let inner = Rc::new(RefCell::new(TabletInner::Pending { seats: Vec::new() }));
        let handler_inner = inner.clone();
        let listener = seat_handler.listen(move |seat, seat_data, _| {
            if seat_data.defunct {
                inner.borrow_mut().remove_seat(&seat);
            } else {
                inner.borrow_mut().new_seat(&seat);
            }
        });

        TabletHandler { inner: handler_inner, _listener: listener }
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
        self.inner.borrow_mut().init_tablet_mgr((*manager).clone());
    }
    fn get(&self) -> Option<Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>> {
        self.inner.borrow().get_mgr()
    }
}

impl<E: TabletHandling> crate::environment::Environment<E> {
    pub fn listen_for_tablets<
        F: FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static,
    >(
        &self,
        f: F,
    ) -> Result<TabletDeviceListener, ()> {
        self.with_inner(move |inner| TabletHandling::listen(inner, f))
    }
    pub fn process_tool_events<
        F: FnMut(
                Attached<wl_seat::WlSeat>,
                Attached<zwp_tablet_tool_v2::ZwpTabletToolV2>,
                ToolEvent,
                DispatchData,
            ) + 'static,
    >(
        &self,
        seat: wl_seat::WlSeat,
        f: F,
    ) -> Result<ToolListener, ()> {
        self.with_inner(move |inner| TabletHandling::process_tool_events(inner, f))
    }
}

fn map_function<
    A: FnMut(
            Attached<wl_seat::WlSeat>,
            Attached<zwp_tablet_tool_v2::ZwpTabletToolV2>,
            ToolEvent,
            DispatchData,
        ) + 'static,
    B: FnMut(Attached<zwp_tablet_tool_v2::ZwpTabletToolV2>, ToolEvent, DispatchData) + 'static,
>(
    seat: wl_seat::WlSeat,
    f: B,
) -> A {
    move |wseat, device, event, ddata| {
        if seat == *wseat {
            f(device, event, ddata);
        }
    }
}
