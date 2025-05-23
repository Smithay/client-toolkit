use std::{
    fmt::{self, Display, Formatter},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use crate::reexports::client::{
    globals::{Global, GlobalList},
    protocol::{wl_pointer, wl_registry::WlRegistry, wl_seat, wl_shm, wl_surface, wl_touch},
    Connection, Dispatch, Proxy, QueueHandle,
};
use crate::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1;
use crate::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_manager_v1::WpCursorShapeManagerV1;
use crate::{
    compositor::SurfaceDataExt,
    globals::GlobalData,
    registry::{ProvidesRegistryState, RegistryHandler},
};

pub mod input_method;
pub mod input_method_v3;
#[cfg(feature = "xkbcommon")]
pub mod keyboard;
pub mod pointer;
pub mod pointer_constraints;
pub mod relative_pointer;
pub mod touch;

use pointer::cursor_shape::CursorShapeManager;
use pointer::{PointerData, PointerDataExt, PointerHandler, ThemeSpec, ThemedPointer, Themes};
use touch::{TouchData, TouchDataExt, TouchHandler};

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
}

#[derive(Debug)]
pub struct SeatState {
    // (name, seat)
    seats: Vec<SeatInner>,
    cursor_shape_manager_state: CursorShapeManagerState,
}

#[derive(Debug)]
enum CursorShapeManagerState {
    NotPresent,
    Pending { registry: WlRegistry, global: Global },
    Bound(CursorShapeManager),
}

impl SeatState {
    pub fn new<D: Dispatch<wl_seat::WlSeat, SeatData> + 'static>(
        global_list: &GlobalList,
        qh: &QueueHandle<D>,
    ) -> SeatState {
        let (seats, cursor_shape_manager) = global_list.contents().with_list(|globals| {
            let global = globals
                .iter()
                .find(|global| global.interface == WpCursorShapeManagerV1::interface().name)
                .map(|global| CursorShapeManagerState::Pending {
                    registry: global_list.registry().clone(),
                    global: global.clone(),
                })
                .unwrap_or(CursorShapeManagerState::NotPresent);

            (
                crate::registry::bind_all(global_list.registry(), globals, qh, 1..=10, |id| {
                    SeatData {
                        has_keyboard: Arc::new(AtomicBool::new(false)),
                        has_pointer: Arc::new(AtomicBool::new(false)),
                        has_touch: Arc::new(AtomicBool::new(false)),
                        name: Arc::new(Mutex::new(None)),
                        id,
                    }
                })
                .expect("failed to bind global"),
                global,
            )
        });

        let mut state =
            SeatState { seats: vec![], cursor_shape_manager_state: cursor_shape_manager };

        for seat in seats {
            let data = seat.data::<SeatData>().unwrap().clone();

            state.seats.push(SeatInner { seat: seat.clone(), data });
        }
        state
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
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
    ) -> Result<wl_pointer::WlPointer, SeatError>
    where
        D: Dispatch<wl_pointer::WlPointer, PointerData> + PointerHandler + 'static,
    {
        self.get_pointer_with_data(qh, seat, PointerData::new(seat.clone()))
    }

    /// Creates a pointer from a seat with the provided theme.
    ///
    /// This will use [`CursorShapeManager`] under the hood when it's available.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a pointer.
    pub fn get_pointer_with_theme<D, S>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        shm: &wl_shm::WlShm,
        surface: wl_surface::WlSurface,
        theme: ThemeSpec,
    ) -> Result<ThemedPointer<PointerData>, SeatError>
    where
        D: Dispatch<wl_pointer::WlPointer, PointerData>
            + Dispatch<wl_surface::WlSurface, S>
            + Dispatch<WpCursorShapeManagerV1, GlobalData>
            + Dispatch<WpCursorShapeDeviceV1, GlobalData>
            + PointerHandler
            + 'static,
        S: SurfaceDataExt + 'static,
    {
        self.get_pointer_with_theme_and_data(
            qh,
            seat,
            shm,
            surface,
            theme,
            PointerData::new(seat.clone()),
        )
    }

    /// Creates a pointer from a seat.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a pointer.
    pub fn get_pointer_with_data<D, U>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        pointer_data: U,
    ) -> Result<wl_pointer::WlPointer, SeatError>
    where
        D: Dispatch<wl_pointer::WlPointer, U> + PointerHandler + 'static,
        U: PointerDataExt + 'static,
    {
        let inner =
            self.seats.iter().find(|inner| &inner.seat == seat).ok_or(SeatError::DeadObject)?;

        if !inner.data.has_pointer.load(Ordering::SeqCst) {
            return Err(SeatError::UnsupportedCapability(Capability::Pointer));
        }

        Ok(seat.get_pointer(qh, pointer_data))
    }

    /// Creates a pointer from a seat with the provided theme and data.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a pointer.
    pub fn get_pointer_with_theme_and_data<D, S, U>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        shm: &wl_shm::WlShm,
        surface: wl_surface::WlSurface,
        theme: ThemeSpec,
        pointer_data: U,
    ) -> Result<ThemedPointer<U>, SeatError>
    where
        D: Dispatch<wl_pointer::WlPointer, U>
            + Dispatch<wl_surface::WlSurface, S>
            + Dispatch<WpCursorShapeManagerV1, GlobalData>
            + Dispatch<WpCursorShapeDeviceV1, GlobalData>
            + PointerHandler
            + 'static,
        S: SurfaceDataExt + 'static,
        U: PointerDataExt + 'static,
    {
        let inner =
            self.seats.iter().find(|inner| &inner.seat == seat).ok_or(SeatError::DeadObject)?;

        if !inner.data.has_pointer.load(Ordering::SeqCst) {
            return Err(SeatError::UnsupportedCapability(Capability::Pointer));
        }

        let wl_ptr = seat.get_pointer(qh, pointer_data);

        if let CursorShapeManagerState::Pending { registry, global } =
            &self.cursor_shape_manager_state
        {
            self.cursor_shape_manager_state =
                match crate::registry::bind_one(registry, &[global.clone()], qh, 1..=2, GlobalData)
                {
                    Ok(bound) => {
                        CursorShapeManagerState::Bound(CursorShapeManager::from_existing(bound))
                    }
                    Err(_) => CursorShapeManagerState::NotPresent,
                }
        }

        let shape_device =
            if let CursorShapeManagerState::Bound(ref bound) = self.cursor_shape_manager_state {
                Some(bound.get_shape_device(&wl_ptr, qh))
            } else {
                None
            };

        Ok(ThemedPointer {
            themes: Arc::new(Mutex::new(Themes::new(theme))),
            pointer: wl_ptr,
            shm: shm.clone(),
            surface,
            shape_device,
            _marker: std::marker::PhantomData,
            _surface_data: std::marker::PhantomData,
        })
    }

    /// Creates a touch handle from a seat.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support touch.
    pub fn get_touch<D>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
    ) -> Result<wl_touch::WlTouch, SeatError>
    where
        D: Dispatch<wl_touch::WlTouch, TouchData> + TouchHandler + 'static,
    {
        self.get_touch_with_data(qh, seat, TouchData::new(seat.clone()))
    }

    /// Creates a touch handle from a seat.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support touch.
    pub fn get_touch_with_data<D, U>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        udata: U,
    ) -> Result<wl_touch::WlTouch, SeatError>
    where
        D: Dispatch<wl_touch::WlTouch, U> + TouchHandler + 'static,
        U: TouchDataExt + 'static,
    {
        let inner =
            self.seats.iter().find(|inner| &inner.seat == seat).ok_or(SeatError::DeadObject)?;

        if !inner.data.has_touch.load(Ordering::SeqCst) {
            return Err(SeatError::UnsupportedCapability(Capability::Touch));
        }

        Ok(seat.get_touch(qh, udata))
    }
}

pub trait SeatHandler: Sized {
    fn seat_state(&mut self) -> &mut SeatState;

    /// A new seat has been created.
    ///
    /// This function only indicates that a seat has been created, you will need to wait for [`new_capability`](SeatHandler::new_capability)
    /// to be called before creating any keyboards,
    fn new_seat(&mut self, conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat);

    /// A new capability is available on the seat.
    ///
    /// This allows you to create the corresponding object related to the capability.
    fn new_capability(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    );

    /// A capability has been removed from the seat.
    ///
    /// If an object has been created from the capability, it should be destroyed.
    fn remove_capability(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    );

    /// A seat has been removed.
    ///
    /// The seat is destroyed and all capability objects created from it are invalid.
    fn remove_seat(&mut self, conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat);
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
            write!(f, "name: \"{name}\" ")?;
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
    id: u32,
}

#[macro_export]
macro_rules! delegate_seat {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_seat::WlSeat: $crate::seat::SeatData
            ] => $crate::seat::SeatState
        );
    };
}

#[derive(Debug)]
struct SeatInner {
    seat: wl_seat::WlSeat,
    data: SeatData,
}

impl<D> Dispatch<wl_seat::WlSeat, SeatData, D> for SeatState
where
    D: Dispatch<wl_seat::WlSeat, SeatData> + SeatHandler,
{
    fn event(
        state: &mut D,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        data: &SeatData,
        conn: &Connection,
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
    D: Dispatch<wl_seat::WlSeat, SeatData> + SeatHandler + ProvidesRegistryState + 'static,
{
    fn new_global(
        state: &mut D,
        conn: &Connection,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        _: u32,
    ) {
        if interface == wl_seat::WlSeat::interface().name {
            let seat = state
                .registry()
                .bind_specific(
                    qh,
                    name,
                    1..=7,
                    SeatData {
                        has_keyboard: Arc::new(AtomicBool::new(false)),
                        has_pointer: Arc::new(AtomicBool::new(false)),
                        has_touch: Arc::new(AtomicBool::new(false)),
                        name: Arc::new(Mutex::new(None)),
                        id: name,
                    },
                )
                .expect("failed to bind global");

            let data = seat.data::<SeatData>().unwrap().clone();

            state.seat_state().seats.push(SeatInner { seat: seat.clone(), data });
            state.new_seat(conn, qh, seat);
        }
    }

    fn remove_global(
        state: &mut D,
        conn: &Connection,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
    ) {
        if interface == wl_seat::WlSeat::interface().name {
            if let Some(seat) = state.seat_state().seats.iter().find(|inner| inner.data.id == name)
            {
                let seat = seat.seat.clone();

                state.remove_seat(conn, qh, seat);
                state.seat_state().seats.retain(|inner| inner.data.id != name);
            }
        }
    }
}
