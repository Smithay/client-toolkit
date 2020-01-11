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

use std::{cell::RefCell, os::unix::io::RawFd, rc::Rc, sync::Arc, time::Duration};

pub use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_client::{
    protocol::{wl_keyboard, wl_seat, wl_surface},
    Attached,
};

mod ffi;
mod state;
pub mod keysyms;

use self::state::KbState;
pub use self::state::{ModifiersState, RMLVO};

/// Possible kinds of key repetition
pub enum RepeatKind {
    /// keys will be repeated at a set rate and delay
    Fixed {
        /// the number of repetitions per second that should occur
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

/// Implement a keyboard for keymap translation
///
/// This requires you to provide a callback to receive the events after they
/// have been interpreted with the keymap.
///
/// The keymap will be loaded from the provided RMLVO rules, or from the compositor
/// provided keymap if `None`.
///
/// Returns an error if xkbcommon could not be initialized, the RMLVO specification
/// contained invalid values, or if the provided seat does not have keyboard capability.
pub fn map_keyboard<F>(
    seat: &Attached<wl_seat::WlSeat>,
    rmlvo: Option<RMLVO>,
    repeatkind: RepeatKind,
    callback: F,
) -> Result<(wl_keyboard::WlKeyboard, RepeatSource), Error>
where
    F: FnMut(Event<'_>, wl_keyboard::WlKeyboard, wayland_client::DispatchData<'_>) + 'static,
{
    let has_kbd = super::with_seat_data(seat, |data| data.has_keyboard).unwrap_or(false);
    let keyboard = if has_kbd {
        seat.get_keyboard()
    } else {
        return Err(Error::NoKeyboard);
    };

    let state = Rc::new(RefCell::new(
        rmlvo
            .map(KbState::from_rmlvo)
            .unwrap_or_else(KbState::new)?,
    ));

    let current_repeat = Rc::new(RefCell::new(None));
    let callback = Rc::new(RefCell::new(callback));

    let source = RepeatSource {
        timer: calloop::timer::Timer::new(),
        state: state.clone(),
        current_repeat: current_repeat.clone(),
        callback: callback.clone(),
    };

    let timer_handle = source.timer.handle();

    let repeat = match repeatkind {
        RepeatKind::System => RepeatDetails {
            locked: false,
            rate: 100,
            delay: 300,
        },
        RepeatKind::Fixed { rate, delay } => RepeatDetails {
            locked: true,
            rate,
            delay,
        },
    };

    let mut kbd_handler = KbdHandler {
        callback,
        timer_handle,
        current_repeat,
        state,
        repeat,
    };

    keyboard.quick_assign(move |keyboard, event, data| {
        kbd_handler.event((**keyboard).clone(), event, data)
    });

    Ok(((**keyboard).clone(), source))
}

/*
 * Classic handling
 */

struct RepeatDetails {
    locked: bool,
    rate: u32,
    delay: u32,
}

struct KbdHandler {
    timer_handle: calloop::timer::TimerHandle<()>,
    state: Rc<RefCell<KbState>>,
    current_repeat: Rc<RefCell<Option<RepeatData>>>,
    callback: Rc<
        RefCell<dyn FnMut(Event<'_>, wl_keyboard::WlKeyboard, wayland_client::DispatchData<'_>)>,
    >,
    repeat: RepeatDetails,
}

impl KbdHandler {
    fn start_repeat(&self, key: u32, keyboard: wl_keyboard::WlKeyboard, time: u32) {
        // start a new repetition, overwriting the previous ones
        self.timer_handle.cancel_all_timeouts();
        *self.current_repeat.borrow_mut() = Some(RepeatData {
            keyboard,
            keycode: key,
            rate: self.repeat.rate,
            time: time + self.repeat.delay,
        });
        self.timer_handle
            .add_timeout(Duration::from_millis(self.repeat.delay as u64), ());
    }

    fn stop_repeat(&self) {
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
            Event::Enter {
                serial,
                surface,
                keys,
            } => self.enter(kbd, serial, surface, keys, dispatch_data),
            Event::Leave { serial, surface } => self.leave(kbd, serial, surface, dispatch_data),
            Event::Key {
                serial,
                time,
                key,
                state,
            } => self.key(kbd, serial, time, key, state, dispatch_data),
            Event::Modifiers {
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => self.modifiers(
                kbd,
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                dispatch_data,
            ),
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
        let rawkeys: &[u32] =
            unsafe { ::std::slice::from_raw_parts(keys.as_ptr() as *const u32, keys.len() / 4) };
        let keys: Vec<u32> = rawkeys.iter().map(|k| state.get_one_sym_raw(*k)).collect();
        (&mut *self.callback.borrow_mut())(
            Event::Enter {
                serial,
                surface,
                rawkeys,
                keysyms: &keys,
            },
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
        self.stop_repeat();
        (&mut *self.callback.borrow_mut())(Event::Leave { serial, surface }, object, dispatch_data);
    }

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

        if key_state == wl_keyboard::KeyState::Pressed {
            (&mut *self.callback.borrow_mut())(
                Event::Key {
                    serial,
                    time,
                    rawkey: key,
                    keysym: sym,
                    state: key_state,
                    utf8: utf8.clone(),
                },
                object.clone(),
                dispatch_data,
            );
            if repeats {
                self.start_repeat(key, object, time);
            }
        } else {
            self.stop_repeat();
            (&mut *self.callback.borrow_mut())(
                Event::Key {
                    serial,
                    time,
                    rawkey: key,
                    keysym: sym,
                    state: key_state,
                    utf8: utf8.clone(),
                },
                object,
                dispatch_data,
            );
        }
    }

    fn modifiers(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        _: u32,
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
                Event::Modifiers {
                    modifiers: state.mods_state(),
                },
                object,
                dispatch_data,
            );
        }
    }

    fn repeat_info(&mut self, _: wl_keyboard::WlKeyboard, rate: i32, delay: i32) {
        if !self.repeat.locked {
            self.repeat.rate = rate as u32;
            self.repeat.delay = delay as u32;
        }
    }
}

/*
 * Repeat handling
 */

struct RepeatData {
    keyboard: wl_keyboard::WlKeyboard,
    keycode: u32,
    // repeat rate, in ms
    rate: u32,
    // time of the last event, in ms
    time: u32,
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
pub struct RepeatSource {
    timer: calloop::timer::Timer<()>,
    state: Rc<RefCell<KbState>>,
    current_repeat: Rc<RefCell<Option<RepeatData>>>,
    callback: Rc<
        RefCell<dyn FnMut(Event<'_>, wl_keyboard::WlKeyboard, wayland_client::DispatchData<'_>)>,
    >,
}

impl calloop::EventSource for RepeatSource {
    type Event = ();

    fn interest(&self) -> calloop::mio::Interest {
        calloop::EventSource::interest(&self.timer)
    }

    fn as_mio_source(&mut self) -> Option<&mut dyn calloop::mio::event::Source> {
        calloop::EventSource::as_mio_source(&mut self.timer)
    }

    fn make_dispatcher<Data: 'static, F: FnMut(Self::Event, &mut Data) + 'static>(
        &mut self,
        _callback: F,
        waker: &Arc<calloop::mio::Waker>,
    ) -> Rc<RefCell<dyn calloop::EventDispatcher<Data>>> {
        let state = self.state.clone();
        let current_repeat = self.current_repeat.clone();
        let callback = self.callback.clone();
        calloop::EventSource::make_dispatcher(
            &mut self.timer,
            move |((), timer_handle), dispatch_data| {
                if let Some(ref mut data) = *current_repeat.borrow_mut() {
                    // there is something to repeat
                    let mut state = state.borrow_mut();
                    let keysym = state.get_one_sym_raw(data.keycode);
                    let utf8 = state.get_utf8_raw(data.keycode);
                    let new_time = data.rate + data.time;
                    // notify the callback
                    (&mut *callback.borrow_mut())(
                        Event::Repeat {
                            time: new_time,
                            rawkey: data.keycode,
                            keysym,
                            utf8,
                        },
                        data.keyboard.clone(),
                        wayland_client::DispatchData::wrap(dispatch_data),
                    );
                    // update the time of last event
                    data.time = new_time;
                    // schedule the next timeout
                    timer_handle.add_timeout(Duration::from_millis(data.rate as u64), ());
                }
            },
            waker,
        )
    }
}
