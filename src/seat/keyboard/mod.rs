#[rustfmt::skip]
pub mod keysyms;

use std::{
    convert::TryInto,
    env,
    fmt::Debug,
    num::NonZeroU32,
    os::unix::io::AsRawFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

use wayland_client::{
    protocol::{wl_keyboard, wl_seat, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};
use xkbcommon::xkb;

use super::{Capability, SeatError, SeatHandler, SeatState};

#[cfg(feature = "calloop")]
pub mod repeat;
#[cfg(feature = "calloop")]
use repeat::RepeatMessage;

/// Error when creating a keyboard.
#[derive(Debug, thiserror::Error)]
pub enum KeyboardError {
    /// Seat error.
    #[error(transparent)]
    Seat(#[from] SeatError),

    /// The specified keymap (RMLVO) is not valid.
    #[error("invalid keymap was specified")]
    InvalidKeymap,
}

impl SeatState {
    /// Creates a keyboard from a seat.
    ///
    /// This keyboard implementation uses libxkbcommon for the keymap.
    ///
    /// Typically the compositor will provide a keymap, but you may specify your own keymap using the `rmlvo`
    /// field.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a keyboard.
    pub fn get_keyboard<D>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        rmlvo: Option<RMLVO>,
    ) -> Result<wl_keyboard::WlKeyboard, KeyboardError>
    where
        D: Dispatch<wl_keyboard::WlKeyboard, KeyboardData>
            + SeatHandler
            + KeyboardHandler
            + 'static,
    {
        let udata = match rmlvo {
            Some(rmlvo) => KeyboardData::from_rmlvo(rmlvo)?,
            None => KeyboardData::default(),
        };

        self.get_keyboard_with_data(qh, seat, udata)
    }

    /// Creates a keyboard from a seat.
    ///
    /// This keyboard implementation uses libxkbcommon for the keymap.
    ///
    /// Typically the compositor will provide a keymap, but you may specify your own keymap using the `rmlvo`
    /// field.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a keyboard.
    pub fn get_keyboard_with_data<D, U>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        udata: U,
    ) -> Result<wl_keyboard::WlKeyboard, KeyboardError>
    where
        D: Dispatch<wl_keyboard::WlKeyboard, U> + SeatHandler + KeyboardHandler + 'static,
        U: KeyboardDataExt + 'static,
    {
        let inner =
            self.seats.iter().find(|inner| &inner.seat == seat).ok_or(SeatError::DeadObject)?;

        if !inner.data.has_keyboard.load(Ordering::SeqCst) {
            return Err(SeatError::UnsupportedCapability(Capability::Keyboard).into());
        }

        Ok(seat.get_keyboard(qh, udata))
    }
}

/// Wrapper around a libxkbcommon keymap
#[allow(missing_debug_implementations)]
pub struct Keymap<'a>(&'a xkb::Keymap);

impl<'a> Keymap<'a> {
    /// Get keymap as string in text format. The keymap should always be valid.
    pub fn as_string(&self) -> String {
        self.0.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1)
    }
}

/// Handler trait for keyboard input.
///
/// The functions defined in this trait are called as keyboard events are received from the compositor.
pub trait KeyboardHandler: Sized {
    /// The keyboard has entered a surface.
    ///
    /// When called, you may assume the specified surface has keyboard focus.
    ///
    /// When a keyboard enters a surface, the `raw` and `keysym` fields indicate which keys are currently
    /// pressed.
    #[allow(clippy::too_many_arguments)]
    fn enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        serial: u32,
        raw: &[u32],
        keysyms: &[u32],
    );

    /// The keyboard has left a surface.
    ///
    /// When called, keyboard focus leaves the specified surface.
    ///
    /// All currently held down keys are released when this event occurs.
    fn leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        serial: u32,
    );

    /// A key has been pressed on the keyboard.
    ///
    /// The key will repeat if there is no other press event afterwards or the key is released.
    fn press_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        serial: u32,
        event: KeyEvent,
    );

    /// A key has been released.
    ///
    /// This stops the key from being repeated if the key is the last key which was pressed.
    fn release_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        serial: u32,
        event: KeyEvent,
    );

    /// Keyboard modifiers have been updated.
    ///
    /// This happens when one of the modifier keys, such as "Shift", "Control" or "Alt" is pressed or
    /// released.
    fn update_modifiers(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        serial: u32,
        modifiers: Modifiers,
    );

    /// The keyboard has updated the rate and delay between repeating key inputs.
    ///
    /// This function does nothing by default but is provided if a repeat mechanism outside of calloop is\
    /// used.
    fn update_repeat_info(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _info: RepeatInfo,
    ) {
    }

    /// Keyboard keymap has been updated.
    ///
    /// `keymap.as_string()` can be used get the keymap as a string. It cannot be exposed directly
    /// as an `xkbcommon::xkb::Keymap` due to the fact xkbcommon uses non-thread-safe reference
    /// counting. But can be used to create an independent `Keymap`.
    ///
    /// This is called after the default handler for keymap changes and does nothing by default.
    fn update_keymap<'a>(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _keymap: Keymap<'a>,
    ) {
    }
}

/// The rate at which a pressed key is repeated.
#[derive(Debug, Clone, Copy)]
pub enum RepeatInfo {
    /// Keys will be repeated at the specified rate and delay.
    Repeat {
        /// The number of repetitions per second that should occur.
        rate: NonZeroU32,

        /// Delay (in milliseconds) between a key press and the start of repetition.
        delay: u32,
    },

    /// Keys should not be repeated.
    Disable,
}

/// Data associated with a key press or release event.
#[derive(Debug, Clone)]
pub struct KeyEvent {
    /// Time at which the keypress occurred.
    pub time: u32,

    /// The raw value of the key.
    pub raw_code: u32,

    /// The interpreted symbol of the key.
    ///
    /// This corresponds to one of the values in the [`keysyms`] module.
    pub keysym: u32,

    /// UTF-8 interpretation of the entered text.
    ///
    /// This will always be [`None`] on release events.
    pub utf8: Option<String>,
}

/// The state of keyboard modifiers
///
/// Each field of this indicates whether a specified modifier is active.
///
/// Depending on the modifier, the modifier key may currently be pressed or toggled.
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    /// The "control" key
    pub ctrl: bool,

    /// The "alt" key
    pub alt: bool,

    /// The "shift" key
    pub shift: bool,

    /// The "Caps lock" key
    pub caps_lock: bool,

    /// The "logo" key
    ///
    /// Also known as the "windows" or "super" key on a keyboard.
    #[doc(alias = "windows")]
    #[doc(alias = "super")]
    pub logo: bool,

    /// The "Num lock" key
    pub num_lock: bool,
}

/// The RMLVO description of a keymap
///
/// All fields are optional, and the system default
/// will be used if set to `None`.
#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct RMLVO {
    /// The rules file to use
    pub rules: Option<String>,

    /// The keyboard model by which to interpret keycodes and LEDs
    pub model: Option<String>,

    /// A comma separated list of layouts (languages) to include in the keymap
    pub layout: Option<String>,

    /// A comma separated list of variants, one per layout, which may modify or
    /// augment the respective layout in various ways
    pub variant: Option<String>,

    /// A comma separated list of options, through which the user specifies
    /// non-layout related preferences, like which key combinations are
    /// used for switching layouts, or which key is the Compose key.
    pub options: Option<String>,
}

pub struct KeyboardData {
    first_event: AtomicBool,
    xkb_context: Mutex<xkb::Context>,
    /// If the user manually specified the RMLVO to use.
    user_specified_rmlvo: bool,
    xkb_state: Mutex<Option<xkb::State>>,
    xkb_compose: Mutex<Option<xkb::compose::State>>,
    #[cfg(feature = "calloop")]
    repeat_sender: Option<calloop::channel::Sender<RepeatMessage>>,
    current_repeat: Mutex<Option<KeyEvent>>,
}

impl Debug for KeyboardData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyboardData").finish_non_exhaustive()
    }
}

#[macro_export]
macro_rules! delegate_keyboard {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_keyboard::WlKeyboard: $crate::seat::keyboard::KeyboardData
            ] => $crate::seat::SeatState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, keyboard: [$($udata:ty),* $(,)?]) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $(
                    $crate::reexports::client::protocol::wl_keyboard::WlKeyboard: $udata,
                )*
            ] => $crate::seat::SeatState
        );
    };
}

// SAFETY: The state does not share state with any other rust types.
unsafe impl Send for KeyboardData {}
// SAFETY: The state is guarded by a mutex since libxkbcommon has no internal synchronization.
unsafe impl Sync for KeyboardData {}

impl Default for KeyboardData {
    fn default() -> Self {
        let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let udata = KeyboardData {
            first_event: AtomicBool::new(false),
            xkb_context: Mutex::new(xkb_context),
            xkb_state: Mutex::new(None),
            user_specified_rmlvo: false,
            xkb_compose: Mutex::new(None),
            repeat_sender: None,
            current_repeat: Mutex::new(None),
        };

        udata.init_compose();

        udata
    }
}

impl KeyboardData {
    pub fn from_rmlvo(rmlvo: RMLVO) -> Result<Self, KeyboardError> {
        let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &xkb_context,
            &rmlvo.rules.unwrap_or_default(),
            &rmlvo.model.unwrap_or_default(),
            &rmlvo.layout.unwrap_or_default(),
            &rmlvo.variant.unwrap_or_default(),
            rmlvo.options,
            xkb::COMPILE_NO_FLAGS,
        );

        if keymap.is_none() {
            return Err(KeyboardError::InvalidKeymap);
        }

        let xkb_state = Some(xkb::State::new(&keymap.unwrap()));

        let udata = KeyboardData {
            first_event: AtomicBool::new(false),
            xkb_context: Mutex::new(xkb_context),
            xkb_state: Mutex::new(xkb_state),
            user_specified_rmlvo: true,
            xkb_compose: Mutex::new(None),
            repeat_sender: None,
            current_repeat: Mutex::new(None),
        };

        udata.init_compose();

        Ok(udata)
    }

    fn init_compose(&self) {
        let xkb_context = self.xkb_context.lock().unwrap();

        if let Some(locale) = env::var_os("LC_ALL")
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LC_CTYPE"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LANG"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .unwrap_or_else(|| "C".into())
            .to_str()
        {
            // TODO: Pending new release of xkbcommon to use new_from_locale with OsStr
            if let Ok(table) = xkb::compose::Table::new_from_locale(
                &xkb_context,
                locale.as_ref(),
                xkb::compose::COMPILE_NO_FLAGS,
            ) {
                let compose_state =
                    xkb::compose::State::new(&table, xkb::compose::COMPILE_NO_FLAGS);
                *self.xkb_compose.lock().unwrap() = Some(compose_state);
            }
        }
    }

    fn update_modifiers(&self) -> Modifiers {
        let guard = self.xkb_state.lock().unwrap();
        let state = guard.as_ref().unwrap();

        Modifiers {
            ctrl: state.mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE),
            alt: state.mod_name_is_active(xkb::MOD_NAME_ALT, xkb::STATE_MODS_EFFECTIVE),
            shift: state.mod_name_is_active(xkb::MOD_NAME_SHIFT, xkb::STATE_MODS_EFFECTIVE),
            caps_lock: state.mod_name_is_active(xkb::MOD_NAME_CAPS, xkb::STATE_MODS_EFFECTIVE),
            logo: state.mod_name_is_active(xkb::MOD_NAME_LOGO, xkb::STATE_MODS_EFFECTIVE),
            num_lock: state.mod_name_is_active(xkb::MOD_NAME_NUM, xkb::STATE_MODS_EFFECTIVE),
        }
    }
}

pub trait KeyboardDataExt: Send + Sync {
    fn keyboard_data(&self) -> &KeyboardData;
    fn keyboard_data_mut(&mut self) -> &mut KeyboardData;
}

impl KeyboardDataExt for KeyboardData {
    fn keyboard_data(&self) -> &KeyboardData {
        self
    }

    fn keyboard_data_mut(&mut self) -> &mut KeyboardData {
        self
    }
}

impl<D, U> Dispatch<wl_keyboard::WlKeyboard, U, D> for SeatState
where
    D: Dispatch<wl_keyboard::WlKeyboard, U> + KeyboardHandler,
    U: KeyboardDataExt,
{
    fn event(
        data: &mut D,
        keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let udata = udata.keyboard_data();

        // The compositor has no way to tell clients if the seat is not version 4 or above.
        // In this case, send a synthetic repeat info event using the default repeat values used by the X
        // server.
        if keyboard.version() < 4 && udata.first_event.load(Ordering::SeqCst) {
            udata.first_event.store(true, Ordering::SeqCst);

            data.update_repeat_info(
                conn,
                qh,
                keyboard,
                RepeatInfo::Repeat { rate: NonZeroU32::new(200).unwrap(), delay: 200 },
            );
        }

        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => {
                match format {
                    WEnum::Value(format) => match format {
                        wl_keyboard::KeymapFormat::NoKeymap => {
                            log::warn!(target: "sctk", "non-xkb compatible keymap");
                        }

                        wl_keyboard::KeymapFormat::XkbV1 => {
                            if udata.user_specified_rmlvo {
                                // state is locked, ignore keymap updates
                                return;
                            }

                            let context = udata.xkb_context.lock().unwrap();

                            // 0.5.0-beta.0 does not mark this function as unsafe but upstream rightly makes
                            // this function unsafe.
                            //
                            // Version 7 of wl_keyboard requires the file descriptor to be mapped using
                            // MAP_PRIVATE. xkbcommon-rs does mmap the file descriptor properly.
                            //
                            // SAFETY:
                            // - wayland-client guarantees we have received a valid file descriptor.
                            #[allow(unused_unsafe)] // Upstream release will change this
                            match unsafe {
                                xkb::Keymap::new_from_fd(
                                    &context,
                                    fd.as_raw_fd(),
                                    size as usize,
                                    xkb::KEYMAP_FORMAT_TEXT_V1,
                                    xkb::COMPILE_NO_FLAGS,
                                )
                            } {
                                Ok(Some(keymap)) => {
                                    let state = xkb::State::new(&keymap);
                                    {
                                        let mut state_guard = udata.xkb_state.lock().unwrap();
                                        *state_guard = Some(state);
                                    }
                                    data.update_keymap(conn, qh, keyboard, Keymap(&keymap));
                                }

                                Ok(None) => {
                                    log::error!(target: "sctk", "invalid keymap");
                                }

                                Err(err) => {
                                    log::error!(target: "sctk", "{}", err);
                                }
                            }
                        }

                        _ => unreachable!(),
                    },

                    WEnum::Unknown(value) => {
                        log::warn!(target: "sctk", "unknown keymap format 0x{:x}", value)
                    }
                }
            }

            wl_keyboard::Event::Enter { serial, surface, keys } => {
                let state_guard = udata.xkb_state.lock().unwrap();

                if let Some(guard) = state_guard.as_ref() {
                    // Keysyms are encoded as an array of u32
                    let raw = keys
                        .chunks_exact(4)
                        .flat_map(TryInto::<[u8; 4]>::try_into)
                        .map(u32::from_le_bytes)
                        .collect::<Vec<_>>();

                    let keysyms = raw
                        .iter()
                        .copied()
                        // We must add 8 to the keycode for any functions we pass the raw keycode into per
                        // wl_keyboard protocol.
                        .map(|raw| guard.key_get_one_sym(raw + 8))
                        .collect::<Vec<_>>();

                    // Drop guard before calling user code.
                    drop(state_guard);

                    data.enter(conn, qh, keyboard, &surface, serial, &raw, &keysyms);
                }
            }

            wl_keyboard::Event::Leave { serial, surface } => {
                // We can send this event without any other checks in the protocol will guarantee a leave is
                // sent before entering a new surface.
                #[cfg(feature = "calloop")]
                {
                    if let Some(repeat_sender) = &udata.repeat_sender {
                        let _ = repeat_sender.send(RepeatMessage::StopRepeat);
                    }
                }

                data.leave(conn, qh, keyboard, &surface, serial);
            }

            wl_keyboard::Event::Key { serial, time, key, state } => match state {
                WEnum::Value(state) => {
                    let state_guard = udata.xkb_state.lock().unwrap();

                    if let Some(guard) = state_guard.as_ref() {
                        // We must add 8 to the keycode for any functions we pass the raw keycode into per
                        // wl_keyboard protocol.
                        let keysym = guard.key_get_one_sym(key + 8);
                        let utf8 = if state == wl_keyboard::KeyState::Pressed {
                            let mut compose = udata.xkb_compose.lock().unwrap();

                            match compose.as_mut() {
                                Some(compose) => match compose.feed(keysym) {
                                    xkb::FeedResult::Ignored => None,
                                    xkb::FeedResult::Accepted => match compose.status() {
                                        xkb::Status::Composed => compose.utf8(),
                                        xkb::Status::Nothing => Some(guard.key_get_utf8(key + 8)),
                                        _ => None,
                                    },
                                },

                                // No compose
                                None => Some(guard.key_get_utf8(key + 8)),
                            }
                        } else {
                            None
                        };

                        // Drop guard before calling user code.
                        drop(state_guard);

                        let event = KeyEvent { time, raw_code: key, keysym, utf8 };

                        match state {
                            wl_keyboard::KeyState::Released => {
                                #[cfg(feature = "calloop")]
                                {
                                    if let Some(repeat_sender) = &udata.repeat_sender {
                                        let mut current_repeat =
                                            udata.current_repeat.lock().unwrap();
                                        if Some(event.raw_code)
                                            == current_repeat.as_ref().map(|r| r.raw_code)
                                        {
                                            current_repeat.take();
                                            let _ = repeat_sender.send(RepeatMessage::StopRepeat);
                                        }
                                    }
                                }
                                data.release_key(conn, qh, keyboard, serial, event);
                            }

                            wl_keyboard::KeyState::Pressed => {
                                #[cfg(feature = "calloop")]
                                {
                                    if let Some(repeat_sender) = &udata.repeat_sender {
                                        let state_guard = udata.xkb_state.lock().unwrap();
                                        let key_repeats = state_guard
                                            .as_ref()
                                            .map(|guard| {
                                                guard.get_keymap().key_repeats(event.raw_code + 8)
                                            })
                                            .unwrap_or_default();
                                        if key_repeats {
                                            udata
                                                .current_repeat
                                                .lock()
                                                .unwrap()
                                                .replace(event.clone());
                                            let _ = repeat_sender
                                                .send(RepeatMessage::StartRepeat(event.clone()));
                                        }
                                    }
                                }
                                data.press_key(conn, qh, keyboard, serial, event);
                            }

                            _ => unreachable!(),
                        }
                    };
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sctk", "{}: compositor sends invalid key state: {:x}", keyboard.id(), unknown);
                }
            },

            wl_keyboard::Event::Modifiers {
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => {
                let mut guard = udata.xkb_state.lock().unwrap();

                let mask = match guard.as_mut() {
                    Some(state) => {
                        let mask = state.update_mask(
                            mods_depressed,
                            mods_latched,
                            mods_locked,
                            0,
                            0,
                            group,
                        );

                        // update current repeating key
                        let mut current_event = udata.current_repeat.lock().unwrap();
                        if let Some(mut event) = current_event.take() {
                            if let Some(repeat_sender) = &udata.repeat_sender {
                                // apply new modifiers to get new utf8
                                let utf8 = {
                                    let mut compose = udata.xkb_compose.lock().unwrap();

                                    match compose.as_mut() {
                                        Some(compose) => match compose.feed(event.keysym) {
                                            xkb::FeedResult::Ignored => None,
                                            xkb::FeedResult::Accepted => match compose.status() {
                                                xkb::Status::Composed => compose.utf8(),
                                                xkb::Status::Nothing => {
                                                    Some(state.key_get_utf8(event.raw_code + 8))
                                                }
                                                _ => None,
                                            },
                                        },

                                        // No compose
                                        None => Some(state.key_get_utf8(event.raw_code + 8)),
                                    }
                                };
                                event.utf8 = utf8;

                                current_event.replace(event.clone());
                                let _ = repeat_sender.send(RepeatMessage::StartRepeat(event));
                            }
                        }
                        mask
                    }
                    None => return,
                };

                // Drop guard before calling user code.
                drop(guard);

                if mask & xkb::STATE_MODS_EFFECTIVE != 0 {
                    let modifiers = udata.update_modifiers();
                    data.update_modifiers(conn, qh, keyboard, serial, modifiers);
                }
            }

            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                let info = if rate != 0 {
                    RepeatInfo::Repeat {
                        rate: NonZeroU32::new(rate as u32).unwrap(),
                        delay: delay as u32,
                    }
                } else {
                    RepeatInfo::Disable
                };

                #[cfg(feature = "calloop")]
                {
                    if let Some(repeat_sender) = &udata.repeat_sender {
                        let _ = repeat_sender.send(RepeatMessage::RepeatInfo(info));
                    }
                }

                data.update_repeat_info(conn, qh, keyboard, info);
            }

            _ => unreachable!(),
        }
    }
}
