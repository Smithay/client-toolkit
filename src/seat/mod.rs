#[cfg(feature = "xkbcommon_keyboard")]
pub mod keyboard;
pub mod pointer;
pub mod touch;

use std::{
    fmt::{self, Display, Formatter},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_pointer, wl_seat, wl_touch},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy, QueueHandle,
};

use crate::registry::{ProvidesRegistryState, RegistryHandler};

use self::{
    pointer::{PointerData, PointerHandler},
    touch::{TouchData, TouchHandler},
};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Keyboard,

    Pointer,

    Touch,
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Capability::Keyboard => write!(f, "keyboard"),
            Capability::Pointer => write!(f, "pointer"),
            Capability::Touch => write!(f, "touch"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SeatError {
    #[error("the capability \"{0}\" is not supported")]
    /// The capability is not supported.
    UnsupportedCapability(Capability),

    /// The seat is dead.
    #[error("the seat is dead")]
    DeadObject,

    /// Protocol error.
    #[error(transparent)]
    Protocol(#[from] InvalidId),
}

#[derive(Debug)]
pub struct SeatState {
    // (name, seat)
    seats: Vec<SeatInner>,
}

impl SeatState {
    pub fn new() -> SeatState {
        SeatState { seats: vec![] }
    }

    /// Returns an iterator over all the seats.
    pub fn seats(&self) -> impl Iterator<Item = wl_seat::WlSeat> {
        self.seats.iter().map(|inner| inner.seat.clone()).collect::<Vec<_>>().into_iter()
    }

    /// Returns information about a seat.
    ///
    /// This will return [`None`] if the seat is dead.
    pub fn info(&self, seat: &wl_seat::WlSeat) -> Option<SeatInfo> {
        self.seats.iter().find(|inner| &inner.seat == seat).map(|inner| {
            let name = inner.data.name.lock().unwrap().clone();

            SeatInfo {
                name,
                has_keyboard: inner.data.has_keyboard.load(Ordering::SeqCst),
                has_pointer: inner.data.has_pointer.load(Ordering::SeqCst),
                has_touch: inner.data.has_touch.load(Ordering::SeqCst),
            }
        })
    }

    /// Creates a pointer from a seat.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a pointer.
    pub fn get_pointer<D>(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
    ) -> Result<wl_pointer::WlPointer, SeatError>
    where
        D: Dispatch<wl_pointer::WlPointer, UserData = PointerData> + PointerHandler + 'static,
    {
        let inner =
            self.seats.iter().find(|inner| &inner.seat == seat).ok_or(SeatError::DeadObject)?;

        if !inner.data.has_pointer.load(Ordering::SeqCst) {
            return Err(SeatError::UnsupportedCapability(Capability::Pointer));
        }

        let pointer = seat.get_pointer(conn, qh, PointerData::default())?;
        Ok(pointer)
    }

    /// Creates a touch handle from a seat.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support touch.
    pub fn get_touch<D>(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
    ) -> Result<wl_touch::WlTouch, SeatError>
    where
        D: Dispatch<wl_touch::WlTouch, UserData = TouchData> + TouchHandler + 'static,
    {
        let inner =
            self.seats.iter().find(|inner| &inner.seat == seat).ok_or(SeatError::DeadObject)?;

        if !inner.data.has_touch.load(Ordering::SeqCst) {
            return Err(SeatError::UnsupportedCapability(Capability::Touch));
        }

        let touch = seat.get_touch(conn, qh, TouchData::default())?;
        Ok(touch)
    }
}

pub trait SeatHandler: Sized {
    fn seat_state(&mut self) -> &mut SeatState;

    /// A new seat has been created.
    ///
    /// This function only indicates that a seat has been created, you will need to wait for [`new_capability`](SeatHandler::new_capability)
    /// to be called before creating any keyboards,
    fn new_seat(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
    );

    /// A new capability is available on the seat.
    ///
    /// This allows you to create the corresponding object related to the capability.
    fn new_capability(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    );

    /// A capability has been removed from the seat.
    ///
    /// If an object has been created from the capability, it should be destroyed.
    fn remove_capability(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    );

    /// A seat has been removed.
    ///
    /// The seat is destroyed and all capability objects created from it are invalid.
    fn remove_seat(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
    );
}

/// Description of a seat.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SeatInfo {
    /// The name of the seat.
    pub name: Option<String>,

    /// Does the seat support a keyboard.
    pub has_keyboard: bool,

    /// Does the seat support a pointer.
    pub has_pointer: bool,

    /// Does the seat support touch input.
    pub has_touch: bool,
}

impl Display for SeatInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(ref name) = self.name {
            write!(f, "name: \"{}\" ", name)?;
        }

        write!(f, "capabilities: (")?;

        if !self.has_keyboard && !self.has_pointer && !self.has_touch {
            write!(f, "none")?;
        } else {
            if self.has_keyboard {
                write!(f, "keyboard")?;

                if self.has_pointer || self.has_touch {
                    write!(f, ", ")?;
                }
            }

            if self.has_pointer {
                write!(f, "pointer")?;

                if self.has_touch {
                    write!(f, ", ")?;
                }
            }

            if self.has_touch {
                write!(f, "touch")?;
            }
        }

        write!(f, ")")
    }
}

#[derive(Debug, Clone)]
pub struct SeatData {
    has_keyboard: Arc<AtomicBool>,
    has_pointer: Arc<AtomicBool>,
    has_touch: Arc<AtomicBool>,
    name: Arc<Mutex<Option<String>>>,
}

#[macro_export]
macro_rules! delegate_seat {
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty:
            [
                $crate::reexports::client::protocol::wl_seat::WlSeat
            ] => $crate::seat::SeatState
        );
    };
}

#[derive(Debug)]
struct SeatInner {
    global_name: u32,
    seat: wl_seat::WlSeat,
    data: SeatData,
}

impl DelegateDispatchBase<wl_seat::WlSeat> for SeatState {
    type UserData = SeatData;
}

impl<D> DelegateDispatch<wl_seat::WlSeat, D> for SeatState
where
    D: Dispatch<wl_seat::WlSeat, UserData = Self::UserData> + SeatHandler,
{
    fn event(
        state: &mut D,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        data: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wl_seat::Event::Capabilities { capabilities } => {
                let capabilities = wl_seat::Capability::from_bits_truncate(capabilities.into());

                let keyboard = capabilities.contains(wl_seat::Capability::Keyboard);
                let has_keyboard = data.has_keyboard.load(Ordering::SeqCst);
                let pointer = capabilities.contains(wl_seat::Capability::Pointer);
                let has_pointer = data.has_pointer.load(Ordering::SeqCst);
                let touch = capabilities.contains(wl_seat::Capability::Touch);
                let has_touch = data.has_touch.load(Ordering::SeqCst);

                // Update capabilities as necessary
                if keyboard != has_keyboard {
                    data.has_keyboard.store(keyboard, Ordering::SeqCst);

                    match keyboard {
                        true => state.new_capability(conn, qh, seat.clone(), Capability::Keyboard),
                        false => {
                            state.remove_capability(conn, qh, seat.clone(), Capability::Keyboard)
                        }
                    }
                }

                if pointer != has_pointer {
                    data.has_pointer.store(pointer, Ordering::SeqCst);

                    match pointer {
                        true => state.new_capability(conn, qh, seat.clone(), Capability::Pointer),
                        false => {
                            state.remove_capability(conn, qh, seat.clone(), Capability::Pointer)
                        }
                    }
                }

                if touch != has_touch {
                    data.has_touch.store(touch, Ordering::SeqCst);

                    match touch {
                        true => state.new_capability(conn, qh, seat.clone(), Capability::Touch),
                        false => state.remove_capability(conn, qh, seat.clone(), Capability::Touch),
                    }
                }
            }

            wl_seat::Event::Name { name } => {
                *data.name.lock().unwrap() = Some(name);
            }

            _ => unreachable!(),
        }
    }
}

impl<D> RegistryHandler<D> for SeatState
where
    D: Dispatch<wl_seat::WlSeat, UserData = SeatData>
        + SeatHandler
        + ProvidesRegistryState
        + 'static,
{
    fn new_global(
        state: &mut D,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        if interface == "wl_seat" {
            let seat = state
                .registry()
                .bind_cached(conn, qh, name, || {
                    (
                        u32::min(version, 7),
                        SeatData {
                            has_keyboard: Arc::new(AtomicBool::new(false)),
                            has_pointer: Arc::new(AtomicBool::new(false)),
                            has_touch: Arc::new(AtomicBool::new(false)),
                            name: Arc::new(Mutex::new(None)),
                        },
                    )
                })
                .expect("failed to bind global");

            let data = seat.data::<SeatData>().unwrap().clone();

            state.seat_state().seats.push(SeatInner {
                global_name: name,
                seat: seat.clone(),
                data,
            });
            state.new_seat(conn, qh, seat);
        }
    }

    fn remove_global(state: &mut D, conn: &mut ConnectionHandle, qh: &QueueHandle<D>, name: u32) {
        if let Some(seat) = state.seat_state().seats.iter().find(|inner| inner.global_name == name)
        {
            let seat = seat.seat.clone();

            state.remove_seat(conn, qh, seat);
            state.seat_state().seats.retain(|inner| inner.global_name != name);
        }
    }
}
