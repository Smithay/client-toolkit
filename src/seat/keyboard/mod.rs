//! Utilities for keymap interpretation of keyboard input
//!
//! This module provides an implementation for `wl_keyboard`
//! objects using `libxkbcommon` to interpret the keyboard input
//! given the user keymap.
//!
//! The entry point of this module is the [`map_keyboard`](fn.map_keyboard.html)
//! function which, given a `wl_seat` and a callback, setup keymap interpretation
//! and key repetition for the `wl_keyboard` of this seat.
//!
//! Key repetition relies on an event source, that needs to be inserted in your
//! calloop event loop. Not doing so will prevent key repetition to work
//! (but the rest of the functionnality will not be affected).

#[cfg(feature = "calloop")]
use std::num::NonZeroU32;
#[cfg(feature = "calloop")]
use std::time::Duration;
use std::{
    cell::RefCell,
    convert::TryInto,
    fs::File,
    os::unix::io::{FromRawFd, RawFd},
    rc::Rc,
};

pub use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_client::{
    protocol::{wl_keyboard, wl_seat, wl_surface},
    Attached,
};

#[rustfmt::skip]
mod ffi;
mod state;
#[rustfmt::skip]
pub mod keysyms;

use self::state::KbState;
pub use self::state::{ModifiersState, RMLVO};

#[cfg(feature = "calloop")]
const MICROS_IN_SECOND: u32 = 1000000;

/// Possible kinds of key repetition
pub enum RepeatKind {
    /// keys will be repeated at a set rate and delay
    Fixed {
        /// The number of repetitions per second that should occur.
        rate: u32,
        /// delay (in milliseconds) between a key press and the start of repetition
        delay: u32,
    },
    /// keys will be repeated at a rate and delay set by the wayland server
    System,
}

#[derive(Debug)]
/// An error that occurred while trying to initialize a mapped keyboard
pub enum Error {
    /// libxkbcommon is not available
    XKBNotFound,
    /// Provided RMLVO specified a keymap that would not be loaded
    BadNames,
    /// The provided seat does not have the keyboard capability
    NoKeyboard,
    /// Failed to init timers for repetition
    TimerError(std::io::Error),
}

/// Events received from a mapped keyboard
pub enum Event<'a> {
    /// The keyboard focus has entered a surface
    Enter {
        /// serial number of the event
        serial: u32,
        /// surface that was entered
        surface: wl_surface::WlSurface,
        /// raw values of the currently pressed keys
        rawkeys: &'a [u32],
        /// interpreted symbols of the currently pressed keys
        keysyms: &'a [u32],
    },
    /// The keyboard focus has left a surface
    Leave {
        /// serial number of the event
        serial: u32,
        /// surface that was left
        surface: wl_surface::WlSurface,
    },
    /// The key modifiers have changed state
    Modifiers {
        /// current state of the modifiers
        modifiers: ModifiersState,
    },
    /// A key event occurred
    Key {
        /// serial number of the event
        serial: u32,
        /// time at which the keypress occurred
        time: u32,
        /// raw value of the key
        rawkey: u32,
        /// interpreted symbol of the key
        keysym: u32,
        /// new state of the key
        state: KeyState,
        /// utf8 interpretation of the entered text
        ///
        /// will always be `None` on key release events
        utf8: Option<String>,
    },
    /// A key repetition event
    Repeat {
        /// time at which the repetition occured
        time: u32,
        /// raw value of the key
        rawkey: u32,
        /// interpreted symbol of the key
        keysym: u32,
        /// utf8 interpretation of the entered text
        utf8: Option<String>,
    },
}

/// Implement a keyboard for keymap translation with key repetition
///
/// This requires you to provide a callback to receive the events after they
/// have been interpreted with the keymap.
///
/// The keymap will be loaded from the provided RMLVO rules, or from the compositor
/// provided keymap if `None`.
///
/// Returns an error if xkbcommon could not be initialized, the RMLVO specification
/// contained invalid values, or if the provided seat does not have keyboard capability.
///
/// **Note:** This adapter does not handle key repetition. See `map_keyboard_repeat` for that.
pub fn map_keyboard<F>(
    seat: &Attached<wl_seat::WlSeat>,
    rmlvo: Option<RMLVO>,
    callback: F,
) -> Result<wl_keyboard::WlKeyboard, Error>
where
    F: FnMut(Event<'_>, wl_keyboard::WlKeyboard, wayland_client::DispatchData<'_>) + 'static,
{
    let has_kbd = super::with_seat_data(seat, |data| data.has_keyboard).unwrap_or(false);
    let keyboard = if has_kbd {
        seat.get_keyboard()
    } else {
        return Err(Error::NoKeyboard);
    };

    let state = Rc::new(RefCell::new(rmlvo.map(KbState::from_rmlvo).unwrap_or_else(KbState::new)?));

    let callback = Rc::new(RefCell::new(callback));

    // prepare the handler
    let mut kbd_handler = KbdHandler {
        callback,
        state,
        #[cfg(feature = "calloop")]
        repeat: None,
    };

    keyboard.quick_assign(move |keyboard, event, data| {
        kbd_handler.event(keyboard.detach(), event, data)
    });

    Ok(keyboard.detach())
}

/// Implement a keyboard for keymap translation with key repetition
///
/// This requires you to provide a callback to receive the events after they
/// have been interpreted with the keymap.
///
/// The keymap will be loaded from the provided RMLVO rules, or from the compositor
/// provided keymap if `None`.
///
/// Returns an error if xkbcommon could not be initialized, the RMLVO specification
/// contained invalid values, or if the provided seat does not have keyboard capability.
///
/// **Note:** The keyboard repetition handling requires the `calloop` cargo feature.
#[cfg(feature = "calloop")]
pub fn map_keyboard_repeat<F, Data: 'static>(
    loop_handle: calloop::LoopHandle<Data>,
    seat: &Attached<wl_seat::WlSeat>,
    rmlvo: Option<RMLVO>,
    repeatkind: RepeatKind,
    callback: F,
) -> Result<(wl_keyboard::WlKeyboard, calloop::Source<RepeatSource>), Error>
where
    F: FnMut(Event<'_>, wl_keyboard::WlKeyboard, wayland_client::DispatchData<'_>) + 'static,
{
    let has_kbd = super::with_seat_data(seat, |data| data.has_keyboard).unwrap_or(false);
    let keyboard = if has_kbd {
        seat.get_keyboard()
    } else {
        return Err(Error::NoKeyboard);
    };

    let state = Rc::new(RefCell::new(rmlvo.map(KbState::from_rmlvo).unwrap_or_else(KbState::new)?));

    let callback = Rc::new(RefCell::new(callback));

    let repeat = match repeatkind {
        RepeatKind::System => RepeatDetails { locked: false, gap: None, delay: 200 },
        RepeatKind::Fixed { rate, delay } => {
            let gap = rate_to_gap(rate as i32);
            RepeatDetails { locked: true, gap, delay }
        }
    };

    // Prepare the repetition handling.
    let (mut kbd_handler, source) = {
        let current_repeat = Rc::new(RefCell::new(None));

        let source = RepeatSource {
            timer: calloop::timer::Timer::new().map_err(Error::TimerError)?,
            state: state.clone(),
            current_repeat: current_repeat.clone(),
        };

        let timer_handle = source.timer.handle();

        let handler = KbdHandler {
            callback: callback.clone(),
            state,
            repeat: Some(KbdRepeat { timer_handle, current_repeat, details: repeat }),
        };
        (handler, source)
    };

    let source = loop_handle
        .insert_source(source, move |event, kbd, ddata| {
            (&mut *callback.borrow_mut())(
                event,
                kbd.clone(),
                wayland_client::DispatchData::wrap(ddata),
            )
        })
        .map_err(|e| Error::TimerError(e.error))?;

    keyboard.quick_assign(move |keyboard, event, data| {
        kbd_handler.event(keyboard.detach(), event, data)
    });

    Ok((keyboard.detach(), source))
}

#[cfg(feature = "calloop")]
fn rate_to_gap(rate: i32) -> Option<NonZeroU32> {
    if rate <= 0 {
        None
    } else if MICROS_IN_SECOND < rate as u32 {
        NonZeroU32::new(1)
    } else {
        NonZeroU32::new(MICROS_IN_SECOND / rate as u32)
    }
}

/*
 * Classic handling
 */

type KbdCallback = dyn FnMut(Event<'_>, wl_keyboard::WlKeyboard, wayland_client::DispatchData<'_>);

#[cfg(feature = "calloop")]
struct RepeatDetails {
    locked: bool,
    /// Gap between key presses in microseconds.
    ///
    /// If the `gap` is `None`, it means that repeat is disabled.
    gap: Option<NonZeroU32>,
    /// Delay before starting key repeat in milliseconds.
    delay: u32,
}

struct KbdHandler {
    state: Rc<RefCell<KbState>>,
    callback: Rc<RefCell<KbdCallback>>,
    #[cfg(feature = "calloop")]
    repeat: Option<KbdRepeat>,
}

#[cfg(feature = "calloop")]
struct KbdRepeat {
    timer_handle: calloop::timer::TimerHandle<()>,
    current_repeat: Rc<RefCell<Option<RepeatData>>>,
    details: RepeatDetails,
}

#[cfg(feature = "calloop")]
impl KbdRepeat {
    fn start_repeat(&self, key: u32, keyboard: wl_keyboard::WlKeyboard, time: u32) {
        // Start a new repetition, overwriting the previous ones
        self.timer_handle.cancel_all_timeouts();

        // Handle disabled repeat rate.
        let gap = match self.details.gap {
            Some(gap) => gap.get() as u64,
            None => return,
        };

        *self.current_repeat.borrow_mut() = Some(RepeatData {
            keyboard,
            keycode: key,
            gap,
            time: (time + self.details.delay) as u64 * 1000,
        });
        self.timer_handle.add_timeout(Duration::from_micros(self.details.delay as u64 * 1000), ());
    }

    fn stop_repeat(&self, key: u32) {
        // only cancel if the released key is the currently repeating key
        let mut guard = self.current_repeat.borrow_mut();
        let stop = (*guard).as_ref().map(|d| d.keycode == key).unwrap_or(false);
        if stop {
            self.timer_handle.cancel_all_timeouts();
            *guard = None;
        }
    }

    fn stop_all_repeat(&self) {
        self.timer_handle.cancel_all_timeouts();
        *self.current_repeat.borrow_mut() = None;
    }
}

impl KbdHandler {
    fn event(
        &mut self,
        kbd: wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        dispatch_data: wayland_client::DispatchData,
    ) {
        use wl_keyboard::Event;

        match event {
            Event::Keymap { format, fd, size } => self.keymap(kbd, format, fd, size),
            Event::Enter { serial, surface, keys } => {
                self.enter(kbd, serial, surface, keys, dispatch_data)
            }
            Event::Leave { serial, surface } => self.leave(kbd, serial, surface, dispatch_data),
            Event::Key { serial, time, key, state } => {
                self.key(kbd, serial, time, key, state, dispatch_data)
            }
            Event::Modifiers { mods_depressed, mods_latched, mods_locked, group, .. } => {
                self.modifiers(kbd, mods_depressed, mods_latched, mods_locked, group, dispatch_data)
            }
            Event::RepeatInfo { rate, delay } => self.repeat_info(kbd, rate, delay),
            _ => {}
        }
    }

    fn keymap(
        &mut self,
        _: wl_keyboard::WlKeyboard,
        format: wl_keyboard::KeymapFormat,
        fd: RawFd,
        size: u32,
    ) {
        let fd = unsafe { File::from_raw_fd(fd) };
        let mut state = self.state.borrow_mut();
        if state.locked() {
            // state is locked, ignore keymap updates
            return;
        }
        if state.ready() {
            // new keymap, we first deinit to free resources
            unsafe {
                state.de_init();
            }
        }
        match format {
            wl_keyboard::KeymapFormat::XkbV1 => unsafe {
                state.init_with_fd(fd, size as usize);
            },
            wl_keyboard::KeymapFormat::NoKeymap => {
                // TODO: how to handle this (hopefully never occuring) case?
            }
            _ => unreachable!(),
        }
    }

    fn enter(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        serial: u32,
        surface: wl_surface::WlSurface,
        keys: Vec<u8>,
        dispatch_data: wayland_client::DispatchData,
    ) {
        let mut state = self.state.borrow_mut();
        let rawkeys = keys
            .chunks_exact(4)
            .map(|c| u32::from_ne_bytes(c.try_into().unwrap()))
            .collect::<Vec<_>>();
        let keys: Vec<u32> = rawkeys.iter().map(|k| state.get_one_sym_raw(*k)).collect();
        (&mut *self.callback.borrow_mut())(
            Event::Enter { serial, surface, rawkeys: &rawkeys, keysyms: &keys },
            object,
            dispatch_data,
        );
    }

    fn leave(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        serial: u32,
        surface: wl_surface::WlSurface,
        dispatch_data: wayland_client::DispatchData,
    ) {
        #[cfg(feature = "calloop")]
        {
            if let Some(ref mut repeat) = self.repeat {
                repeat.stop_all_repeat();
            }
        }
        (&mut *self.callback.borrow_mut())(Event::Leave { serial, surface }, object, dispatch_data);
    }

    #[cfg_attr(not(feature = "calloop"), allow(unused_variables))]
    fn key(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        serial: u32,
        time: u32,
        key: u32,
        key_state: wl_keyboard::KeyState,
        dispatch_data: wayland_client::DispatchData,
    ) {
        let (sym, utf8, repeats) = {
            let mut state = self.state.borrow_mut();
            // Get the values to generate a key event
            let sym = state.get_one_sym_raw(key);
            let utf8 = if key_state == wl_keyboard::KeyState::Pressed {
                match state.compose_feed(sym) {
                    Some(ffi::xkb_compose_feed_result::XKB_COMPOSE_FEED_ACCEPTED) => {
                        if let Some(status) = state.compose_status() {
                            match status {
                                ffi::xkb_compose_status::XKB_COMPOSE_COMPOSED => {
                                    state.compose_get_utf8()
                                }
                                ffi::xkb_compose_status::XKB_COMPOSE_NOTHING => {
                                    state.get_utf8_raw(key)
                                }
                                _ => None,
                            }
                        } else {
                            state.get_utf8_raw(key)
                        }
                    }
                    Some(_) => {
                        // XKB_COMPOSE_FEED_IGNORED
                        None
                    }
                    None => {
                        // XKB COMPOSE is not initialized
                        state.get_utf8_raw(key)
                    }
                }
            } else {
                None
            };
            let repeats = unsafe { state.key_repeats(key + 8) };
            (sym, utf8, repeats)
        };

        #[cfg(feature = "calloop")]
        {
            if let Some(ref mut repeat_handle) = self.repeat {
                if repeats {
                    if key_state == wl_keyboard::KeyState::Pressed {
                        repeat_handle.start_repeat(key, object.clone(), time);
                    } else {
                        repeat_handle.stop_repeat(key);
                    }
                }
            }
        }

        (&mut *self.callback.borrow_mut())(
            Event::Key { serial, time, rawkey: key, keysym: sym, state: key_state, utf8 },
            object,
            dispatch_data,
        );
    }

    fn modifiers(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
        dispatch_data: wayland_client::DispatchData,
    ) {
        {
            let mut state = self.state.borrow_mut();
            state.update_modifiers(mods_depressed, mods_latched, mods_locked, group);
            (&mut *self.callback.borrow_mut())(
                Event::Modifiers { modifiers: state.mods_state() },
                object,
                dispatch_data,
            );
        }
    }

    #[cfg_attr(not(feature = "calloop"), allow(unused_variables))]
    fn repeat_info(&mut self, _: wl_keyboard::WlKeyboard, rate: i32, delay: i32) {
        #[cfg(feature = "calloop")]
        {
            if let Some(ref mut repeat_handle) = self.repeat {
                if !repeat_handle.details.locked {
                    repeat_handle.details.gap = rate_to_gap(rate);
                    repeat_handle.details.delay = delay as u32;
                }
            }
        }
    }
}

/*
 * Repeat handling
 */

#[cfg(feature = "calloop")]
struct RepeatData {
    keyboard: wl_keyboard::WlKeyboard,
    keycode: u32,
    /// Gap between key presses in microseconds.
    gap: u64,
    /// Time of the last event in microseconds.
    time: u64,
}

/// An event source managing the key repetition of a keyboard
///
/// It is given to you from [`map_keyboard`](fn.map_keyboard.html), and you need to
/// insert it in your calloop event loop if you want to have functionning key repetition.
///
/// If don't want key repetition you can just drop it.
///
/// This source will not directly generate calloop events, and the callback provided to
/// `EventLoopHandle::insert_source()` will be ignored. Instead it triggers the
/// callback you provided to [`map_keyboard`](fn.map_keyboard.html).
#[cfg(feature = "calloop")]
pub struct RepeatSource {
    timer: calloop::timer::Timer<()>,
    state: Rc<RefCell<KbState>>,
    current_repeat: Rc<RefCell<Option<RepeatData>>>,
}

#[cfg(feature = "calloop")]
impl calloop::EventSource for RepeatSource {
    type Event = Event<'static>;
    type Metadata = wl_keyboard::WlKeyboard;
    type Ret = ();

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> std::io::Result<()>
    where
        F: FnMut(Event<'static>, &mut wl_keyboard::WlKeyboard),
    {
        let current_repeat = &self.current_repeat;
        let state = &self.state;
        self.timer.process_events(readiness, token, |(), timer_handle| {
            if let Some(ref mut data) = *current_repeat.borrow_mut() {
                // there is something to repeat
                let mut state = state.borrow_mut();
                let keysym = state.get_one_sym_raw(data.keycode);
                let utf8 = state.get_utf8_raw(data.keycode);
                let new_time = data.gap + data.time;
                // Notify the callback.
                callback(
                    Event::Repeat {
                        time: (new_time / 1000) as u32,
                        rawkey: data.keycode,
                        keysym,
                        utf8,
                    },
                    &mut data.keyboard,
                );
                // Update the time of last event.
                data.time = new_time;
                // Schedule the next timeout.
                timer_handle.add_timeout(Duration::from_micros(data.gap), ());
            }
        })
    }

    fn register(&mut self, poll: &mut calloop::Poll, token: calloop::Token) -> std::io::Result<()> {
        self.timer.register(poll, token)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token: calloop::Token,
    ) -> std::io::Result<()> {
        self.timer.reregister(poll, token)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> std::io::Result<()> {
        self.timer.unregister(poll)
    }
}
