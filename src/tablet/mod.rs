use wayland_protocols::unstable::tablet::v2::client::*;

use crate::environment;
use std::{
    cell::RefCell,
    cmp,
    rc::{Rc, Weak},
};
use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Attached, DispatchData, Main,
};

use bitflags::bitflags;

mod tablet;

/// Callback to get informed about new devices being added to a seat
type TabletDeviceCallback =
    dyn FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static;

type TabletListeners = Vec<Weak<RefCell<TabletDeviceCallback>>>;

pub struct TabletDeviceListener {
    _cb: Rc<RefCell<TabletDeviceCallback>>,
}

/// Contains TabletManager mapping seats to tablet seats
enum TabletInner {
    Ready {
        mgr: Attached<zwp_tablet_manager_v2::ZwpTabletManagerV2>,
        data: Rc<RefCell<SharedData>>,
    },
    Pending {
        seats: Vec<Attached<wl_seat::WlSeat>>,
    },
}

struct SharedData {
    tablet_seats: Vec<(Attached<wl_seat::WlSeat>, zwp_tablet_seat_v2::ZwpTabletSeatV2)>,
    /// Global callback for new tablet devices
    listeners: TabletListeners,
    tools: Vec<(zwp_tablet_tool_v2::ZwpTabletToolV2, ToolMetaData)>,
}

/// Handles tablet device added/removed events
pub struct TabletHandler {
    inner: Rc<RefCell<TabletInner>>,
    _listener: crate::seat::SeatListener,
}

pub enum TabletDeviceEvent {
    ToolAdded { tool: Attached<zwp_tablet_tool_v2::ZwpTabletToolV2> },
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

impl TabletHandling for TabletHandler {
    fn listen<F: FnMut(Attached<wl_seat::WlSeat>, TabletDeviceEvent, DispatchData) + 'static>(
        &mut self,
        callback: F,
    ) -> Result<TabletDeviceListener, ()> {
        let rc = Rc::new(RefCell::new(callback)) as Rc<_>;
        let ref mut inner = *self.inner.borrow_mut();
        match inner {
            TabletInner::Ready { data, .. } => {
                data.borrow_mut().listeners.push(Rc::downgrade(&rc));
                Ok(TabletDeviceListener { _cb: rc })
            }
            TabletInner::Pending { .. } => Err(()),
        }
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

        let tablet_seats = Vec::new();
        let listeners = Vec::new();

        let data = Rc::new(RefCell::new(SharedData { tablet_seats, listeners, tools: Vec::new() }));
        for seat in seats {
            let tablet_seat = mgr.get_tablet_seat(&seat);
            // attach tablet seat to global callback for new devices
            let dclone = data.clone();
            tablet_seat.quick_assign(move |t_seat, evt, _| {
                tablet_seat_cb(t_seat, dclone.clone(), evt);
            });
            data.borrow_mut().tablet_seats.push((seat, tablet_seat.detach()));
        }

        *self = TabletInner::Ready { mgr, data };
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
            Self::Ready { mgr, data, .. } => {
                let mut datamut = data.borrow_mut();
                if datamut.tablet_seats.iter().any(|(s, _)| *s == *seat) {
                    // the seat already exists, nothing to do
                    return;
                }
                let tablet_seat = mgr.get_tablet_seat(seat);
                datamut.tablet_seats.push((seat.clone(), tablet_seat.detach()));
                // attach tablet seat to global callback for new devices
                let dclone = data.clone();
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
            Self::Ready { data, .. } => data.borrow_mut().tablet_seats.retain(|(s, _)| s != seat),
            Self::Pending { seats } => seats.retain(|s| s != seat),
        }
    }
}

fn tablet_seat_cb(
    tablet_seat: Main<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    handler_data: Rc<RefCell<SharedData>>,
    event: zwp_tablet_seat_v2::Event,
) {
    match event {
        zwp_tablet_seat_v2::Event::ToolAdded { id } => {
            // set callback for tool events
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
        _ => {
            todo!();
        }
    }
}

fn tablet_tool_cb(
    tablet_seat: Attached<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    tablet_tool: Main<zwp_tablet_tool_v2::ZwpTabletToolV2>,
    handler_data: Rc<RefCell<SharedData>>,
    event: zwp_tablet_tool_v2::Event,
    mut ddata: DispatchData,
) {
    match event {
        zwp_tablet_tool_v2::Event::Type { .. } => {}
        zwp_tablet_tool_v2::Event::Done => {
            //emit tool added event
            //need reference to inner to call respective listener
            handler_data.borrow_mut().listeners.retain(|lst| {
                if let Some(cb) = Weak::upgrade(lst) {
                    let wl_seat = handler_data
                        .borrow_mut()
                        .tablet_seats
                        .iter()
                        .find(|(_, tseat)| *tseat == *tablet_seat)
                        .map(|(wseat, _)| wseat.clone());
                    match wl_seat {
                        Some(wseat) => {
                            (&mut *cb.borrow_mut())(
                                wseat,
                                TabletDeviceEvent::ToolAdded { tool: tablet_tool.clone().into() },
                                ddata.reborrow(),
                            );
                            true
                        }
                        None => false,
                    }
                } else {
                    false
                }
            });
        }
        zwp_tablet_tool_v2::Event::Removed => {
            //emit tool removed event
        }
        _ => todo!(),
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
