use wayland_protocols::unstable::tablet::v2::client::{
    zwp_tablet_manager_v2, zwp_tablet_seat_v2, zwp_tablet_v2,
};

use crate::environment;
use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    cmp,
    rc::{Rc, Weak},
    sync::Mutex,
};
use tablet::TabletEvent;
use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Attached, DispatchData, Main,
};

mod tablet;

/// Callback to get informed about new devices being added to a seat
type TabletDeviceCallback =
    dyn FnMut(wl_seat::WlSeat, TabletDeviceEvent, DispatchData) + 'static;

/// Contains TabletManager mapping seats to tablet seats
enum TabletInner {
    Ready {
        mgr: Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>,
        tabletSeats: Vec<(wl_seat::WlSeat, zwp_tablet_seat_v2::ZwpTabletSeatV2)>,
        /// Global callback for new tablet devices
        callback: Rc<RefCell<Box<TabletDeviceCallback>>>,
    },
    Pending {
        seats: Vec<wl_seat::WlSeat>,
    },
}

/// Handles tablet device added/removed events
pub struct TabletHandler {
    inner: TabletInner,
    _listener: crate::seat::SeatListener,
}

enum TabletDeviceEvent {
    ToolAdded,
    TabletAdded,
    PadAdded,
}

trait TabletHandling {
    /// Set global callback for new tablet devices being added/removed to a seat
    fn set_callback<F: FnMut(wl_seat::WlSeat, TabletDeviceEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<(),()>;
}
/*
impl TabletHandling for TabletHandler {
    fn set_callback<F: FnMut(wl_seat::WlSeat, TabletDeviceEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<(),()>{
        self.inner.borrow_mut().set_callback(callback)
    }
}*/

impl TabletInner {
    fn init_tablet_mgr(&mut self, mgr: Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>) {
        let seats = if let TabletInner::Pending { seats } = self {
            std::mem::replace(seats, Vec::new())
        } else {
            log::warn!("Ignoring second zwp_tablet_manager_v2");
            return;
        };

        let mut tabletSeats = Vec::new();
        let callback = Rc::new(RefCell::new(Box::new(|_, _: TabletDeviceEvent, _: DispatchData| {})
            as Box<dyn FnMut(_, TabletDeviceEvent, DispatchData)>));
        for seat in seats {
            let cb = callback.clone();
            let my_seat = seat.clone();
            let tablet_seat = mgr.get_tablet_seat(&my_seat).detach();
            tabletSeats.push((my_seat, tablet_seat))
        }

        *self = TabletInner::Ready {mgr, tabletSeats, callback}
    }    
    fn get_mgr(&self) -> Option<Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>> {
        match self {
            Self::Ready { mgr,..} => Some(mgr.clone()),
            Self::Pending { .. } => None
        }
    }
}

impl TabletHandler {
    pub fn init<S>(seat_handler: &mut S) -> TabletHandler where S: crate::seat::SeatHandling,
    {
        let inner = TabletInner::Pending{seats:Vec::new()};
        let listener = seat_handler.listen(move |seat, seat_data, _| {
            if seat_data.defunct {
                //seat_inner.borrow_mut().remove_seat(&seat);
            } else {
                //seat_inner.borrow_mut().new_seat(&seat);
            }
        });

        TabletHandler {inner, _listener:listener}
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

/// Represents a physical table device
pub struct TabletData {
    pub name: String,
    pub vid: u32,
    pub pid: u32,
    pub path: String,
    device: zwp_tablet_v2::ZwpTabletV2,
}