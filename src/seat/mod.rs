use std::{
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_keyboard, wl_seat, wl_surface},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy, QueueHandle, WEnum,
};

use crate::registry::{RegistryHandle, RegistryHandler};

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

    /// Creates a keyboard from a seat.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a keyboard.
    pub fn get_keyboard<D>(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
    ) -> Result<wl_keyboard::WlKeyboard, SeatError>
    where
        D: Dispatch<wl_keyboard::WlKeyboard, UserData = SeatData> + 'static,
    {
        let inner =
            self.seats.iter().find(|inner| &inner.seat == seat).ok_or(SeatError::DeadObject)?;

        if !inner.data.has_keyboard.load(Ordering::SeqCst) {
            return Err(SeatError::UnsupportedCapability(Capability::Keyboard));
        }

        let keyboard = seat.get_keyboard(conn, qh, inner.data.clone())?;
        Ok(keyboard)
    }
}

pub trait SeatHandler<D> {
    /// A new seat has been created.
    ///
    /// This function only indicates that a seat has been created, you will need to wait for [`new_capability`](SeatHandle::new_capability)
    /// to be called before creating any keyboards,
    fn new_seat(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        seat: wl_seat::WlSeat,
    );

    /// A new capability is available on the seat.
    ///
    /// This allows you to create the corresponding object related to the capability.
    fn new_capability(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        seat: wl_seat::WlSeat,
        capability: Capability,
    );

    /// A capability has been removed from the seat.
    ///
    /// If an object has been created from the capability, it should be destroyed.
    fn remove_capability(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        seat: wl_seat::WlSeat,
        capability: Capability,
    );

    /// A seat has been removed.
    ///
    /// The seat is destroyed and all capability objects created from it are invalid.
    fn remove_seat(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        seat: wl_seat::WlSeat,
    );

    /// The keyboard focus is set to a surface.
    fn keyboard_focus(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
    );

    /// The keyboard focus is removed from a surface.
    fn keyboard_release_focus(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
    );

    fn keyboard_press_key(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        keyboard: &wl_keyboard::WlKeyboard,
        time: u32,
        key: u32,
    );

    fn keyboard_release_key(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        keyboard: &wl_keyboard::WlKeyboard,
        time: u32,
        key: u32,
    );

    fn keyboard_update_modifiers(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        keyboard: &wl_keyboard::WlKeyboard,
        // TODO: Other params
    );

    /// The keyboard has updated the rate and delay between repeating key inputs.
    fn keyboard_update_repeat_info(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut SeatState,
        keyboard: &wl_keyboard::WlKeyboard,
        rate: u32,
        delay: u32,
    );
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SeatInfo {
    pub name: Option<String>,
    pub has_keyboard: bool,
    pub has_pointer: bool,
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

#[derive(Debug)]
pub struct SeatDispatch<'s, D, H: SeatHandler<D>>(
    pub &'s mut SeatState,
    pub &'s mut H,
    pub PhantomData<D>,
);

#[derive(Debug, Clone)]
pub struct SeatData {
    has_keyboard: Arc<AtomicBool>,
    has_pointer: Arc<AtomicBool>,
    has_touch: Arc<AtomicBool>,
    name: Arc<Mutex<Option<String>>>,
}

#[derive(Debug)]
struct SeatInner {
    global_name: u32,
    seat: wl_seat::WlSeat,
    data: SeatData,
}

impl<D, H> DelegateDispatchBase<wl_seat::WlSeat> for SeatDispatch<'_, D, H>
where
    H: SeatHandler<D>,
{
    type UserData = SeatData;
}

impl<D, H> DelegateDispatch<wl_seat::WlSeat, D> for SeatDispatch<'_, D, H>
where
    D: Dispatch<wl_seat::WlSeat, UserData = Self::UserData>,
    H: SeatHandler<D>,
{
    fn event(
        &mut self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        data: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wl_seat::Event::Capabilities { capabilities } => {
                let capabilities = match capabilities {
                    WEnum::Value(capabilities) => capabilities,

                    WEnum::Unknown(value) => {
                        log::warn!(target: "sctk", "{} sent some unknown capabilities: {}", seat.id(), value);
                        // In a best effort, drop any capabilities we don't understand.
                        wl_seat::Capability::from_bits_truncate(value)
                    }
                };

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
                        true => self.1.new_capability(
                            conn,
                            qh,
                            self.0,
                            seat.clone(),
                            Capability::Keyboard,
                        ),
                        false => self.1.remove_capability(
                            conn,
                            qh,
                            self.0,
                            seat.clone(),
                            Capability::Keyboard,
                        ),
                    }
                }

                if pointer != has_pointer {
                    data.has_pointer.store(pointer, Ordering::SeqCst);

                    match pointer {
                        true => self.1.new_capability(
                            conn,
                            qh,
                            self.0,
                            seat.clone(),
                            Capability::Pointer,
                        ),
                        false => self.1.remove_capability(
                            conn,
                            qh,
                            self.0,
                            seat.clone(),
                            Capability::Pointer,
                        ),
                    }
                }

                if touch != has_touch {
                    data.has_touch.store(touch, Ordering::SeqCst);

                    match touch {
                        true => {
                            self.1.new_capability(conn, qh, self.0, seat.clone(), Capability::Touch)
                        }
                        false => self.1.remove_capability(
                            conn,
                            qh,
                            self.0,
                            seat.clone(),
                            Capability::Touch,
                        ),
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

impl<D, H> DelegateDispatchBase<wl_keyboard::WlKeyboard> for SeatDispatch<'_, D, H>
where
    H: SeatHandler<D>,
{
    type UserData = SeatData;
}

impl<D, H> DelegateDispatch<wl_keyboard::WlKeyboard, D> for SeatDispatch<'_, D, H>
where
    D: Dispatch<wl_keyboard::WlKeyboard, UserData = Self::UserData>,
    H: SeatHandler<D>,
{
    fn event(
        &mut self,
        keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { format, fd: _, size: _ } => {
                match format {
                    WEnum::Value(format) => match format {
                        wl_keyboard::KeymapFormat::NoKeymap => {
                            log::warn!(target: "sctk", "non-xkb compatible keymap, assuming platform codes");
                        }

                        wl_keyboard::KeymapFormat::XkbV1 => {}

                        _ => unreachable!(),
                    },

                    WEnum::Unknown(value) => {
                        log::warn!(target: "sctk", "Unknown keymap format {:x}", value)
                    }
                }

                log::error!(target: "sctk", "TODO: Keymap w/ format {:?}", format);
            }

            wl_keyboard::Event::Enter { serial: _, surface, keys: _ } => {
                // Notify of focus.
                self.1.keyboard_focus(conn, qh, self.0, keyboard, &surface);

                // TODO: Send events to unpress keys that are no longer held.
            }

            wl_keyboard::Event::Leave { serial: _, surface } => {
                // We can send this event without any other checks in the protocol will guarantee a leave is\
                // sent before entering a new surface.
                self.1.keyboard_release_focus(conn, qh, self.0, keyboard, &surface);
            }

            wl_keyboard::Event::Key { serial: _, time, key, state } => match state {
                WEnum::Value(state) => match state {
                    wl_keyboard::KeyState::Released => {
                        self.1.keyboard_release_key(conn, qh, self.0, keyboard, time, key);
                    }

                    wl_keyboard::KeyState::Pressed => {
                        self.1.keyboard_press_key(conn, qh, self.0, keyboard, time, key);
                    }

                    _ => unreachable!(),
                },

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: compositor sends invalid key state: {:x}", keyboard.id(), unknown);
                }
            },

            wl_keyboard::Event::Modifiers {
                serial: _,
                mods_depressed: _,
                mods_latched: _,
                mods_locked: _,
                group: _,
            } => {
                log::error!(target: "sctk", "TODO: modifiers");
            }

            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                self.1.keyboard_update_repeat_info(
                    conn,
                    qh,
                    self.0,
                    keyboard,
                    rate as u32,
                    delay as u32,
                );
            }

            _ => unreachable!(),
        }
    }
}

impl<D, H> RegistryHandler<D> for SeatDispatch<'_, D, H>
where
    D: Dispatch<wl_seat::WlSeat, UserData = SeatData> + 'static,
    H: SeatHandler<D>,
{
    fn new_global(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
        handle: &mut RegistryHandle,
    ) {
        if interface == "wl_seat" {
            let seat = handle
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

            self.0.seats.push(SeatInner { global_name: name, seat, data });
        }
    }

    fn remove_global(&mut self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>, name: u32) {
        if let Some(seat) = self.0.seats.iter().find(|inner| inner.global_name == name) {
            let seat = seat.seat.clone();
            self.1.remove_seat(conn, qh, self.0, seat);

            self.0.seats.retain(|inner| inner.global_name != name);
        }
    }
}
