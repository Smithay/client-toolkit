//! Types for automatically handling seats
//!
//!

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, Weak};
use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Attached, Main,
};

type SeatCallback = dyn Fn(Attached<wl_seat::WlSeat>, &SeatData) + Send + Sync + 'static;

/// The metadata associated with a seat
pub struct SeatData {
    /// The name of this seat
    ///
    /// It can be used as an identifier for the seat
    pub name: String,
    /// Whether this seat has a pointer available
    pub has_pointer: bool,
    /// Whether this seat has a keyboard available
    pub has_keyboard: bool,
    /// Whether this seat has a touchscreen available
    pub has_touch: bool,
    /// Whether this seat has been removed from the registry
    ///
    /// Once a seat is removed, you will no longer receive any
    /// event on any of its associated devices (pointer, keyboard or touch).
    ///
    /// You can thus cleanup all your state associated with this seat.
    pub defunct: bool,
}

impl SeatData {
    fn new() -> SeatData {
        SeatData {
            name: String::new(),
            has_pointer: false,
            has_keyboard: false,
            has_touch: false,
            defunct: false,
        }
    }
}

/// A simple handler for seats
///
/// This handler will manage seats and track their capabilities.
///
/// You can register callbacks using the [`SeatHandling::listen`](trait.SeatHandling.html)
/// to be notified whenever a seat is created, destroyed, or its capabilities change.
pub struct SeatHandler {
    seats: Vec<(u32, Attached<wl_seat::WlSeat>)>,
    listeners: Rc<RefCell<Vec<Weak<SeatCallback>>>>,
}

impl SeatHandler {
    /// Create a new SeatHandler
    pub fn new() -> SeatHandler {
        SeatHandler {
            seats: Vec::new(),
            listeners: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

/// A handle to an seat listener callback
///
/// Dropping it disables the associated callback and frees the closure.
pub struct SeatListener {
    _cb: Arc<SeatCallback>,
}

impl crate::environment::MultiGlobalHandler<wl_seat::WlSeat> for SeatHandler {
    fn created(&mut self, registry: Attached<wl_registry::WlRegistry>, id: u32, version: u32) {
        // Seat is supported up to version 6
        let version = std::cmp::min(version, 6);
        let seat = registry.bind::<wl_seat::WlSeat>(version, id);
        seat.as_ref()
            .user_data()
            .set_threadsafe(|| Mutex::new(SeatData::new()));
        let cb_listeners = self.listeners.clone();
        seat.assign_mono(move |seat, event| process_seat_event(seat, event, &cb_listeners));
        self.seats.push((id, (*seat).clone()));
    }
    fn removed(&mut self, id: u32) {
        let mut listeners = self.listeners.borrow_mut();
        self.seats.retain(|&(i, ref seat)| {
            if i != id {
                true
            } else {
                // This data must be `Mutex<SeatData>` if this seat is in our vec
                let data = seat.as_ref().user_data().get::<Mutex<SeatData>>().unwrap();
                let mut guard = data.lock().unwrap();
                guard.defunct = true;
                // notify the listeners that the seat is dead
                listeners.retain(|lst| {
                    if let Some(cb) = Weak::upgrade(lst) {
                        cb((*seat).clone(), &*guard);
                        true
                    } else {
                        false
                    }
                });
                false
            }
        });
    }
    fn get_all(&self) -> Vec<Attached<wl_seat::WlSeat>> {
        self.seats.iter().map(|(_, s)| s.clone()).collect()
    }
}

fn process_seat_event(
    seat: Main<wl_seat::WlSeat>,
    event: wl_seat::Event,
    listeners: &RefCell<Vec<Weak<SeatCallback>>>,
) {
    let data = seat.as_ref().user_data().get::<Mutex<SeatData>>().unwrap();
    let mut guard = data.lock().unwrap();
    match event {
        wl_seat::Event::Name { name } => guard.name = name,
        wl_seat::Event::Capabilities { capabilities } => {
            guard.has_pointer = capabilities.contains(wl_seat::Capability::Pointer);
            guard.has_keyboard = capabilities.contains(wl_seat::Capability::Keyboard);
            guard.has_touch = capabilities.contains(wl_seat::Capability::Touch);
        }
        _ => unreachable!(),
    }
    // only advertize a seat once it is initialized, meaining it has a name
    // and at least one capability
    if !guard.name.is_empty() && (guard.has_pointer || guard.has_keyboard || guard.has_touch) {
        listeners.borrow_mut().retain(|lst| {
            if let Some(cb) = Weak::upgrade(lst) {
                cb((*seat).clone(), &*guard);
                true
            } else {
                false
            }
        });
    }
}

/// Access the data associated with this seat
///
/// The provided closure is given the [`SeatData`](struct.SeatData.html) as argument,
/// and its return value is returned from this function.
///
/// If the provided `WlSeat` has not yet been initialized or is not managed by SCTK, `None` is returned.
///
/// If the seat has been removed by the compositor, the `defunct` field of the `SeatData`
/// will be set to `true`. This handler will not automatically detroy the output by calling its
/// `release` method, to avoid interfering with your logic.
pub fn with_seat_data<T, F: FnOnce(&SeatData) -> T>(seat: &wl_seat::WlSeat, f: F) -> Option<T> {
    if let Some(ref udata_mutex) = seat.as_ref().user_data().get::<Mutex<SeatData>>() {
        let udata = udata_mutex.lock().unwrap();
        Some(f(&*udata))
    } else {
        None
    }
}

/// Trait representing the SeatHandler functions
///
/// Implementing this trait on your inner environment struct used with the
/// [`environment!`](../macro.environment.html) by delegating it to its
/// [`SeatHandler`](struct.SeatHandler.html) field will make available the seat-associated
/// method on your [`Environment`](../environment/struct.Environment.html).
pub trait SeatHandling {
    /// Insert a listener for seat events
    fn listen<F: Fn(Attached<wl_seat::WlSeat>, &SeatData) + Send + Sync + 'static>(
        &mut self,
        f: F,
    ) -> SeatListener;
}

impl SeatHandling for SeatHandler {
    fn listen<F: Fn(Attached<wl_seat::WlSeat>, &SeatData) + Send + Sync + 'static>(
        &mut self,
        f: F,
    ) -> SeatListener {
        let arc = Arc::new(f) as Arc<_>;
        self.listeners.borrow_mut().push(Arc::downgrade(&arc));
        SeatListener { _cb: arc }
    }
}

impl<E: SeatHandling> crate::environment::Environment<E> {
    /// Insert a new listener for seats
    ///
    /// The provided closure will be invoked whenever a `wl_seat` is made available,
    /// removed, or see its capabilities changed.
    ///
    /// The returned [`SeatListener`](../seat/struct.SeatListener.hmtl) keeps your callback alive,
    /// dropping it will disable it.
    pub fn listen_for_seats<F: Fn(Attached<wl_seat::WlSeat>, &SeatData) + Send + Sync + 'static>(
        &self,
        f: F,
    ) -> SeatListener {
        self.with_inner(move |inner| SeatHandling::listen(inner, f))
    }
}

impl<E: crate::environment::MultiGlobalHandler<wl_seat::WlSeat>>
    crate::environment::Environment<E>
{
    /// Shorthand method to retrieve the list of seats
    pub fn get_all_seats(&self) -> Vec<wl_seat::WlSeat> {
        self.get_all_globals::<wl_seat::WlSeat>()
            .into_iter()
            .map(|o| o.detach())
            .collect()
    }
}
