use wayland_protocols::unstable::tablet::v2::client::*;

use crate::environment;
use std::{
    cell::RefCell,
    cmp,
    rc::{Rc, Weak},
};
use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Attached, DispatchData,
};

use bitflags::bitflags;

mod tablet;

/// Callback to get informed about new devices being added to a seat
type TabletDeviceCallback =
    dyn FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static;

pub struct TabletDeviceListener {
    _cb: Rc<TabletDeviceCallback>,
}

/// Contains TabletManager mapping seats to tablet seats
enum TabletInner {
    Ready {
        mgr: Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>,
        tablet_seats: Vec<(wl_seat::WlSeat, zwp_tablet_seat_v2::ZwpTabletSeatV2)>,
        /// Global callback for new tablet devices
        listeners: Vec<Weak<TabletDeviceCallback>>,
        tools: Vec<(zwp_tablet_tool_v2::ZwpTabletToolV2, bool)>,
    },
    Pending {
        seats: Vec<wl_seat::WlSeat>,
    },
}

/// Handles tablet device added/removed events
pub struct TabletHandler {
    inner: Rc<RefCell<TabletInner>>,
    _listener: crate::seat::SeatListener,
}

pub enum TabletDeviceEvent {
    ToolAdded { tool: zwp_tablet_tool_v2::ZwpTabletToolV2 },
    ToolRemoved { tool: zwp_tablet_tool_v2::ZwpTabletToolV2 },
}

pub enum HardwareToolId {
    Serial { hardware_serial_hi: u32, hardware_serial_lo: u32 },
    Wacom { hardware_id_hi: u32, hardware_id_lo: u32 },
}

bitflags! {
    struct ToolDataState: u8 {
        const NEW              = 0b00000000;
        const GOT_TYPE         = 0b00000001;
        const GOT_CAPABILITIES = 0b00000010;
        const GOT_HW_ID        = 0b00000100;
        const READY            = Self::GOT_TYPE.bits | Self::GOT_CAPABILITIES.bits | Self::GOT_HW_ID.bits;
    }
}

pub struct ToolMetaData {
    pub capabilities: Vec<zwp_tablet_tool_v2::Capability>,
    pub hardware_id: HardwareToolId,
    pub tool_type: zwp_tablet_tool_v2::Type,
    state: ToolDataState,
}

pub trait TabletHandling {
    /// Set global callback for new tablet devices being added/removed to a seat
    fn listen<F: FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<TabletDeviceListener, ()>;
}
/*
impl TabletHandling for TabletHandler {
    fn listen<F: FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<TabletDeviceListener, ()> {
        let rc = Rc::new(callback) as Rc<TabletDeviceCallback>;
        match self.inner.borrow_mut() {
            TabletInner::Ready { listeners, .. } => {
                listeners.push(Rc::downgrade(&rc));
                Ok(TabletDeviceListener { _cb: rc })
            }
            TabletInner::Pending { .. } => Err(()),
        }
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

        let mut tablet_seats = Vec::new();
        let listeners = Vec::new();
        for seat in seats {
            let my_seat = seat.clone();
            let tablet_seat = mgr.get_tablet_seat(&my_seat).detach();
            tablet_seats.push((my_seat, tablet_seat))
        }

        *self = TabletInner::Ready { tools: Vec::new(), mgr, tablet_seats, listeners }
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
    fn new_seat(&mut self, seat: &wl_seat::WlSeat) {
        match self {
            Self::Ready { mgr, tablet_seats, .. } => {
                if tablet_seats.iter().any(|(s, _)| s == seat) {
                    // the seat already exists, nothing to do
                    return;
                }
                let tablet_seat = mgr.get_tablet_seat(seat).detach();
                tablet_seats.push((seat.clone(), tablet_seat));
            }
            Self::Pending { seats } => {
                seats.push(seat.clone());
            }
        }
    }

    fn remove_seat(&mut self, seat: &wl_seat::WlSeat) {
        match self {
            Self::Ready { tablet_seats, .. } => tablet_seats.retain(|(s, _)| s != seat),
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
