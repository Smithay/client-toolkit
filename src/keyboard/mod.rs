//! Utilities for keymap interpretation of keyboard input
//! This module provides an implementation for `wl_keyboard`
//! objects using `libxkbcommon` to interpret the keyboard input
//! given the user keymap.
//!
//! You simply need to provide an implementation to receive the
//! intepreted events, as described by the `Event` enum of this modules.
//!
//! Implementation of your `NewProxy<WlKeyboard>` can be done with the
//! `map_keyboard_auto` or the `map_keyboard_rmlvo` functions depending
//! on whether you wish to use the keymap provided by the server or a
//! specific one.

use std::env;
use std::ffi::CString;
use std::fs::File;
use std::os::raw::c_char;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::{FromRawFd, RawFd};
use std::ptr;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use memmap::MmapOptions;

pub use wayland_client::protocol::wl_keyboard::KeyState;
use wayland_client::protocol::{wl_keyboard, wl_seat, wl_surface};

use self::ffi::xkb_state_component;
use self::ffi::XKBCOMMON_HANDLE as XKBH;

mod ffi;
pub mod keysyms;

struct KbState {
    xkb_context: *mut ffi::xkb_context,
    xkb_keymap: *mut ffi::xkb_keymap,
    xkb_state: *mut ffi::xkb_state,
    xkb_compose_table: *mut ffi::xkb_compose_table,
    xkb_compose_state: *mut ffi::xkb_compose_state,
    mods_state: ModifiersState,
    locked: bool,
}

/// Represents the current state of the keyboard modifiers
///
/// Each field of this struct represents a modifier and is `true` if this modifier is active.
///
/// For some modifiers, this means that the key is currently pressed, others are toggled
/// (like caps lock).
#[derive(Copy, Clone, Debug, Default)]
pub struct ModifiersState {
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
    /// Also known as the "windows" key on most keyboards
    pub logo: bool,
    /// The "Num lock" key
    pub num_lock: bool,
}

impl ModifiersState {
    fn new() -> ModifiersState {
        ModifiersState::default()
    }

    fn update_with(&mut self, state: *mut ffi::xkb_state) {
        self.ctrl = unsafe {
            (XKBH.xkb_state_mod_name_is_active)(
                state,
                ffi::XKB_MOD_NAME_CTRL.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE,
            ) > 0
        };
        self.alt = unsafe {
            (XKBH.xkb_state_mod_name_is_active)(
                state,
                ffi::XKB_MOD_NAME_ALT.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE,
            ) > 0
        };
        self.shift = unsafe {
            (XKBH.xkb_state_mod_name_is_active)(
                state,
                ffi::XKB_MOD_NAME_SHIFT.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE,
            ) > 0
        };
        self.caps_lock = unsafe {
            (XKBH.xkb_state_mod_name_is_active)(
                state,
                ffi::XKB_MOD_NAME_CAPS.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE,
            ) > 0
        };
        self.logo = unsafe {
            (XKBH.xkb_state_mod_name_is_active)(
                state,
                ffi::XKB_MOD_NAME_LOGO.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE,
            ) > 0
        };
        self.num_lock = unsafe {
            (XKBH.xkb_state_mod_name_is_active)(
                state,
                ffi::XKB_MOD_NAME_NUM.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE,
            ) > 0
        };
    }
}

unsafe impl Send for KbState {}

impl KbState {
    fn update_modifiers(
        &mut self,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    ) {
        if !self.ready() {
            return;
        }
        let mask = unsafe {
            (XKBH.xkb_state_update_mask)(
                self.xkb_state,
                mods_depressed,
                mods_latched,
                mods_locked,
                0,
                0,
                group,
            )
        };
        if mask.contains(xkb_state_component::XKB_STATE_MODS_EFFECTIVE) {
            // effective value of mods have changed, we need to update our state
            self.mods_state.update_with(self.xkb_state);
        }
    }

    fn get_one_sym_raw(&mut self, keycode: u32) -> u32 {
        if !self.ready() {
            return 0;
        }
        unsafe { (XKBH.xkb_state_key_get_one_sym)(self.xkb_state, keycode + 8) }
    }

    fn get_utf8_raw(&mut self, keycode: u32) -> Option<String> {
        if !self.ready() {
            return None;
        }
        let size = unsafe {
            (XKBH.xkb_state_key_get_utf8)(self.xkb_state, keycode + 8, ptr::null_mut(), 0)
        } + 1;
        if size <= 1 {
            return None;
        };
        let mut buffer = Vec::with_capacity(size as usize);
        unsafe {
            buffer.set_len(size as usize);
            (XKBH.xkb_state_key_get_utf8)(
                self.xkb_state,
                keycode + 8,
                buffer.as_mut_ptr() as *mut _,
                size as usize,
            );
        };
        // remove the final `\0`
        buffer.pop();
        // libxkbcommon will always provide valid UTF8
        Some(unsafe { String::from_utf8_unchecked(buffer) })
    }

    fn compose_feed(&mut self, keysym: u32) -> Option<ffi::xkb_compose_feed_result> {
        if !self.ready() || self.xkb_compose_state.is_null() {
            return None;
        }
        Some(unsafe { (XKBH.xkb_compose_state_feed)(self.xkb_compose_state, keysym) })
    }

    fn compose_status(&mut self) -> Option<ffi::xkb_compose_status> {
        if !self.ready() || self.xkb_compose_state.is_null() {
            return None;
        }
        Some(unsafe { (XKBH.xkb_compose_state_get_status)(self.xkb_compose_state) })
    }

    fn compose_get_utf8(&mut self) -> Option<String> {
        if !self.ready() || self.xkb_compose_state.is_null() {
            return None;
        }
        let size = unsafe {
            (XKBH.xkb_compose_state_get_utf8)(self.xkb_compose_state, ptr::null_mut(), 0)
        } + 1;
        if size <= 1 {
            return None;
        };
        let mut buffer = Vec::with_capacity(size as usize);
        unsafe {
            buffer.set_len(size as usize);
            (XKBH.xkb_compose_state_get_utf8)(
                self.xkb_compose_state,
                buffer.as_mut_ptr() as *mut _,
                size as usize,
            );
        };
        // remove the final `\0`
        buffer.pop();
        // libxkbcommon will always provide valid UTF8
        Some(unsafe { String::from_utf8_unchecked(buffer) })
    }

    fn new() -> Result<KbState, Error> {
        let xkbh = match ffi::XKBCOMMON_OPTION.as_ref() {
            Some(h) => h,
            None => return Err(Error::XKBNotFound),
        };
        let xkb_context =
            unsafe { (xkbh.xkb_context_new)(ffi::xkb_context_flags::XKB_CONTEXT_NO_FLAGS) };
        if xkb_context.is_null() {
            return Err(Error::XKBNotFound);
        }

        let mut me = KbState {
            xkb_context,
            xkb_keymap: ptr::null_mut(),
            xkb_state: ptr::null_mut(),
            xkb_compose_table: ptr::null_mut(),
            xkb_compose_state: ptr::null_mut(),
            mods_state: ModifiersState::new(),
            locked: false,
        };

        unsafe {
            me.init_compose();
        }

        Ok(me)
    }

    unsafe fn init_compose(&mut self) {
        let locale = env::var_os("LC_ALL")
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LC_CTYPE"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LANG"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .unwrap_or_else(|| "C".into());
        let locale = CString::new(locale.into_vec()).unwrap();

        let compose_table = (XKBH.xkb_compose_table_new_from_locale)(
            self.xkb_context,
            locale.as_ptr(),
            ffi::xkb_compose_compile_flags::XKB_COMPOSE_COMPILE_NO_FLAGS,
        );

        if compose_table.is_null() {
            // init of compose table failed, continue without compose
            return;
        }

        let compose_state = (XKBH.xkb_compose_state_new)(
            compose_table,
            ffi::xkb_compose_state_flags::XKB_COMPOSE_STATE_NO_FLAGS,
        );

        if compose_state.is_null() {
            // init of compose state failed, continue without compose
            (XKBH.xkb_compose_table_unref)(compose_table);
            return;
        }

        self.xkb_compose_table = compose_table;
        self.xkb_compose_state = compose_state;
    }

    unsafe fn post_init(&mut self, xkb_keymap: *mut ffi::xkb_keymap) {
        let xkb_state = (XKBH.xkb_state_new)(xkb_keymap);
        self.xkb_keymap = xkb_keymap;
        self.xkb_state = xkb_state;
        self.mods_state.update_with(xkb_state);
    }

    unsafe fn de_init(&mut self) {
        (XKBH.xkb_state_unref)(self.xkb_state);
        self.xkb_state = ptr::null_mut();
        (XKBH.xkb_keymap_unref)(self.xkb_keymap);
        self.xkb_keymap = ptr::null_mut();
    }

    unsafe fn init_with_fd(&mut self, fd: RawFd, size: usize) {
        let map = MmapOptions::new()
            .len(size)
            .map(&File::from_raw_fd(fd))
            .unwrap();

        let xkb_keymap = (XKBH.xkb_keymap_new_from_string)(
            self.xkb_context,
            map.as_ptr() as *const _,
            ffi::xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
            ffi::xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
        );

        if xkb_keymap.is_null() {
            panic!("Received invalid keymap from compositor.");
        }

        self.post_init(xkb_keymap);
    }

    unsafe fn init_with_rmlvo(&mut self, names: ffi::xkb_rule_names) -> Result<(), Error> {
        let xkb_keymap = (XKBH.xkb_keymap_new_from_names)(
            self.xkb_context,
            &names,
            ffi::xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
        );

        if xkb_keymap.is_null() {
            return Err(Error::BadNames);
        }

        self.post_init(xkb_keymap);

        Ok(())
    }

    unsafe fn key_repeats(&mut self, xkb_keycode_t: ffi::xkb_keycode_t) -> bool {
        (XKBH.xkb_keymap_key_repeats)(self.xkb_keymap, xkb_keycode_t) == 1
    }

    #[inline]
    fn ready(&self) -> bool {
        !self.xkb_state.is_null()
    }
}

impl Drop for KbState {
    fn drop(&mut self) {
        unsafe {
            (XKBH.xkb_compose_state_unref)(self.xkb_compose_state);
            (XKBH.xkb_compose_table_unref)(self.xkb_compose_table);
            (XKBH.xkb_state_unref)(self.xkb_state);
            (XKBH.xkb_keymap_unref)(self.xkb_keymap);
            (XKBH.xkb_context_unref)(self.xkb_context);
        }
    }
}

/// Determines the behavior of key repetition
#[derive(PartialEq)]
pub enum KeyRepeatKind {
    /// keys will be repeated at a set rate and delay
    Fixed {
        /// the number of repetitions per second that should occur
        rate: u64,
        /// delay (in milliseconds) between a key press and the start of repetition
        delay: u64,
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
}

/// The RMLVO description of a keymap
///
/// All fields are optional, and the system default
/// will be used if set to `None`.
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
    /// Repetition information advertising
    RepeatInfo {
        /// rate (in millisecond) at which the repetition should occur
        rate: i32,
        /// delay (in millisecond) between a key press and the start of repetition
        delay: i32,
    },
    /// The key modifiers have changed state
    Modifiers {
        /// current state of the modifiers
        modifiers: ModifiersState,
    },
}

/// An event sent at repeated intervals for certain keys determined by xkb_keymap_key_repeats
pub struct KeyRepeatEvent {
    /// time at which the keypress occurred
    pub time: u32,
    /// raw value of the key
    pub rawkey: u32,
    /// interpreted symbol of the key
    pub keysym: u32,
    /// utf8 interpretation of the entered text
    pub utf8: Option<String>,
}

/// Implement a keyboard to automatically detect the keymap
///
/// This requires you to provide an implementation to receive the events after they
/// have been interpreted with the keymap.
///
/// The keymap information will be loaded from the events sent by the compositor,
/// as such you need to call this method as soon as you have created the keyboard
/// to make sure this event does not get lost.
///
/// Returns an error if xkbcommon could not be initialized.
pub fn map_keyboard_auto<Impl>(
    seat: &wl_seat::WlSeat,
    implementation: Impl,
) -> Result<wl_keyboard::WlKeyboard, Error>
where
    for<'a> Impl: FnMut(Event<'a>, wl_keyboard::WlKeyboard) + 'static,
{
    let state = match KbState::new() {
        Ok(s) => s,
        Err(e) => return Err(e),
    };
    Ok(implement_kbd(
        seat,
        state,
        implementation,
        None::<(_, fn(_, _))>,
    ))
}

/// Implement a keyboard for a predefined keymap
///
/// This requires you to provide an implementation to receive the events after they
/// have been interpreted with the keymap.
///
/// The keymap will be loaded from the provided RMLVO rules. Any keymap provided
/// by the compositor will be ignored.
///
/// Returns an error if xkbcommon could not be initialized or the RMLVO specification
/// contained invalid values.
pub fn map_keyboard_rmlvo<Impl>(
    seat: &wl_seat::WlSeat,
    rmlvo: RMLVO,
    implementation: Impl,
) -> Result<wl_keyboard::WlKeyboard, Error>
where
    for<'a> Impl: FnMut(Event<'a>, wl_keyboard::WlKeyboard) + 'static,
{
    fn to_cstring(s: Option<String>) -> Result<Option<CString>, Error> {
        s.map_or(Ok(None), |s| CString::new(s).map(Option::Some))
            .map_err(|_| Error::BadNames)
    }

    fn init_state(rmlvo: RMLVO) -> Result<KbState, Error> {
        let mut state = KbState::new()?;

        let rules = to_cstring(rmlvo.rules)?;
        let model = to_cstring(rmlvo.model)?;
        let layout = to_cstring(rmlvo.layout)?;
        let variant = to_cstring(rmlvo.variant)?;
        let options = to_cstring(rmlvo.options)?;

        let xkb_names = ffi::xkb_rule_names {
            rules: rules.map_or(ptr::null(), |s| s.as_ptr()),
            model: model.map_or(ptr::null(), |s| s.as_ptr()),
            layout: layout.map_or(ptr::null(), |s| s.as_ptr()),
            variant: variant.map_or(ptr::null(), |s| s.as_ptr()),
            options: options.map_or(ptr::null(), |s| s.as_ptr()),
        };

        unsafe {
            state.init_with_rmlvo(xkb_names)?;
        }

        state.locked = true;
        Ok(state)
    }

    match init_state(rmlvo) {
        Ok(state) => Ok(implement_kbd(
            seat,
            state,
            implementation,
            None::<(_, fn(_, _))>,
        )),
        Err(error) => Err(error),
    }
}

fn implement_kbd<Impl, RepeatImpl>(
    seat: &wl_seat::WlSeat,
    state: KbState,
    implementation: Impl,
    repeat: Option<(KeyRepeatKind, RepeatImpl)>,
) -> wl_keyboard::WlKeyboard
where
    for<'a> Impl: FnMut(Event<'a>, wl_keyboard::WlKeyboard) + 'static,
    RepeatImpl: FnMut(KeyRepeatEvent, wl_keyboard::WlKeyboard) + Send + 'static,
{
    let state = Arc::new(Mutex::new(state));
    let repeat = repeat.map(|(kind, implem)| RepeatHandler {
        implementation: Arc::new(Mutex::new(implem)),
        state: state.clone(),
        kind,
        ongoing: None,
        rate: 5,
        delay: 300,
    });

    seat.get_keyboard(|kbd| {
        kbd.implement(
            KbdHandler {
                state,
                repeat,
                implementation,
            },
            (),
        )
    })
    .unwrap()
}

/// Implement a keyboard to automatically detect the keymap and send KeyRepeatEvents
/// at set intervals
///
/// This requires you to provide an implementation to receive the events after they
/// have been interpreted with the keymap. You must also provide an implementation to be called
/// when KeyRepeatEvents are sent at intervals set by the KeyRepeatKind argument, this
/// implementation can be called at anytime, independent of the dispatching of wayland events.
/// The dispatching of KeyRepeatEvents is handled with the spawning of threads.
///
/// The keymap information will be loaded from the events sent by the compositor,
/// as such you need to call this method as soon as you have created the keyboard
/// to make sure this event does not get lost.
///
/// Returns an error if xkbcommon could not be initialized.
pub fn map_keyboard_auto_with_repeat<Impl, RepeatImpl>(
    seat: &wl_seat::WlSeat,
    key_repeat_kind: KeyRepeatKind,
    implementation: Impl,
    repeat_implementation: RepeatImpl,
) -> Result<wl_keyboard::WlKeyboard, Error>
where
    for<'a> Impl: FnMut(Event<'a>, wl_keyboard::WlKeyboard) + 'static,
    RepeatImpl: FnMut(KeyRepeatEvent, wl_keyboard::WlKeyboard) + Send + 'static,
{
    let state = match KbState::new() {
        Ok(s) => s,
        Err(e) => return Err(e),
    };
    Ok(implement_kbd(
        seat,
        state,
        implementation,
        Some((key_repeat_kind, repeat_implementation)),
    ))
}

/// Implement a keyboard for a predefined keymap and send KeyRepeatEvents at set
/// intervals
///
/// This requires you to provide an implementation to receive the events after they
/// have been interpreted with the keymap. You must also provide an implementation to be called
/// when KeyRepeatEvents are sent at intervals set by the KeyRepeatKind argument, this
/// implementation can be called at anytime, independent of the dispatching of wayland events.
/// The dispatching of KeyRepeatEvents is handled with the spawning of threads.
///
/// The keymap will be loaded from the provided RMLVO rules. Any keymap provided
/// by the compositor will be ignored.
///
/// Returns an error if xkbcommon could not be initialized or the RMLVO specification
/// contained invalid values.
pub fn map_keyboard_rmlvo_with_repeat<Impl, RepeatImpl>(
    seat: &wl_seat::WlSeat,
    rmlvo: RMLVO,
    key_repeat_kind: KeyRepeatKind,
    implementation: Impl,
    repeat_implementation: RepeatImpl,
) -> Result<wl_keyboard::WlKeyboard, Error>
where
    for<'a> Impl: FnMut(Event<'a>, wl_keyboard::WlKeyboard) + 'static,
    RepeatImpl: FnMut(KeyRepeatEvent, wl_keyboard::WlKeyboard) + Send + 'static,
{
    fn to_cstring(s: Option<String>) -> Result<Option<CString>, Error> {
        s.map_or(Ok(None), |s| CString::new(s).map(Option::Some))
            .map_err(|_| Error::BadNames)
    }

    fn init_state(rmlvo: RMLVO) -> Result<KbState, Error> {
        let mut state = KbState::new()?;

        let rules = to_cstring(rmlvo.rules)?;
        let model = to_cstring(rmlvo.model)?;
        let layout = to_cstring(rmlvo.layout)?;
        let variant = to_cstring(rmlvo.variant)?;
        let options = to_cstring(rmlvo.options)?;

        let xkb_names = ffi::xkb_rule_names {
            rules: rules.map_or(ptr::null(), |s| s.as_ptr()),
            model: model.map_or(ptr::null(), |s| s.as_ptr()),
            layout: layout.map_or(ptr::null(), |s| s.as_ptr()),
            variant: variant.map_or(ptr::null(), |s| s.as_ptr()),
            options: options.map_or(ptr::null(), |s| s.as_ptr()),
        };

        unsafe {
            state.init_with_rmlvo(xkb_names)?;
        }

        state.locked = true;
        Ok(state)
    }

    match init_state(rmlvo) {
        Ok(state) => Ok(implement_kbd(
            seat,
            state,
            implementation,
            Some((key_repeat_kind, repeat_implementation)),
        )),
        Err(error) => Err(error),
    }
}

struct KbdHandler<Impl, RepeatImpl> {
    implementation: Impl,
    state: Arc<Mutex<KbState>>,
    repeat: Option<RepeatHandler<RepeatImpl>>,
}

impl<Impl, RepeatImpl> KbdHandler<Impl, RepeatImpl>
where
    for<'a> Impl: FnMut(Event<'a>, wl_keyboard::WlKeyboard) + 'static,
    RepeatImpl: FnMut(KeyRepeatEvent, wl_keyboard::WlKeyboard) + Send + 'static,
{
    fn start_repeat(&mut self, key: u32, object: wl_keyboard::WlKeyboard, time: u32) {
        if let Some(ref mut repeat) = self.repeat {
            repeat.start(key, object, time);
        }
    }

    fn stop_repeat(&mut self, key: Option<u32>) {
        if let Some(ref mut repeat) = self.repeat {
            repeat.stop(key);
        }
    }

    fn set_repeat_timing(&mut self, rate: i32, delay: i32) {
        if let Some(ref mut repeat) = self.repeat {
            repeat.rate = rate;
            repeat.delay = delay;
        }
    }

    fn repeat_state_changed(&mut self) {
        if let Some(ref mut repeat) = self.repeat {
            repeat.state_changed();
        }
    }
}

struct RepeatHandler<RepeatImpl> {
    implementation: Arc<Mutex<RepeatImpl>>,
    state: Arc<Mutex<KbState>>,
    kind: KeyRepeatKind,
    ongoing: Option<(u32, mpsc::Sender<()>)>,
    rate: i32,
    delay: i32,
}

impl<RepeatImpl> RepeatHandler<RepeatImpl>
where
    RepeatImpl: FnMut(KeyRepeatEvent, wl_keyboard::WlKeyboard) + Send + 'static,
{
    fn start(&mut self, key: u32, object: wl_keyboard::WlKeyboard, time: u32) {
        // replace any previously repeating key
        let (sender, receiver) = mpsc::channel();
        self.ongoing = Some((key, sender));

        let thread_impl = self.implementation.clone();
        let thread_state = self.state.clone();
        let repeat_timing = match self.kind {
            KeyRepeatKind::Fixed { rate, delay } => (rate, delay),
            KeyRepeatKind::System => (self.rate as u64, self.delay as u64),
        };
        // Start thread to send key events
        thread::spawn(move || {
            let time_tracker = Instant::now();
            // Delay
            thread::sleep(Duration::from_millis(repeat_timing.1));
            let (mut sym, mut utf8) = {
                let mut state = thread_state.lock().unwrap();
                (state.get_one_sym_raw(key), state.get_utf8_raw(key))
            };

            loop {
                // Drain channel
                let mut need_update = false;
                loop {
                    match receiver.try_recv() {
                        Ok(()) => need_update = true,
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => return,
                    }
                }
                if need_update {
                    // Update state
                    let mut state = thread_state.lock().unwrap();
                    sym = state.get_one_sym_raw(key);
                    utf8 = state.get_utf8_raw(key);
                }

                let elapsed_time = time_tracker.elapsed();
                (&mut *thread_impl.lock().unwrap())(
                    KeyRepeatEvent {
                        time: time
                            + elapsed_time.as_secs() as u32 * 1000
                            + elapsed_time.subsec_nanos() / 1_000_000,
                        rawkey: key,
                        keysym: sym,
                        utf8: utf8.clone(),
                    },
                    object.clone(),
                );
                // Rate
                thread::sleep(Duration::from_secs(1) / repeat_timing.0 as u32);
            }
        });
    }

    fn stop(&mut self, key: Option<u32>) {
        if let Some((current_key, sender)) = self.ongoing.take() {
            if key.is_some() && Some(current_key) != key {
                self.ongoing = Some((current_key, sender))
            }
        }
    }

    fn state_changed(&mut self) {
        if let Some((_, ref chan)) = self.ongoing {
            chan.send(()).unwrap();
        }
    }
}

impl<Impl, RepeatImpl> wl_keyboard::EventHandler for KbdHandler<Impl, RepeatImpl>
where
    for<'a> Impl: FnMut(Event<'a>, wl_keyboard::WlKeyboard) + 'static,
    RepeatImpl: FnMut(KeyRepeatEvent, wl_keyboard::WlKeyboard) + Send + 'static,
{
    fn keymap(
        &mut self,
        _: wl_keyboard::WlKeyboard,
        format: wl_keyboard::KeymapFormat,
        fd: RawFd,
        size: u32,
    ) {
        let mut state = self.state.lock().unwrap();
        if state.locked {
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
    ) {
        let mut state = self.state.lock().unwrap();
        let rawkeys: &[u32] =
            unsafe { ::std::slice::from_raw_parts(keys.as_ptr() as *const u32, keys.len() / 4) };
        let keys: Vec<u32> = rawkeys.iter().map(|k| state.get_one_sym_raw(*k)).collect();
        (self.implementation)(
            Event::Enter {
                serial,
                surface,
                rawkeys,
                keysyms: &keys,
            },
            object,
        );
    }

    fn leave(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        serial: u32,
        surface: wl_surface::WlSurface,
    ) {
        self.stop_repeat(None);
        (self.implementation)(Event::Leave { serial, surface }, object);
    }

    fn key(
        &mut self,
        object: wl_keyboard::WlKeyboard,
        serial: u32,
        time: u32,
        key: u32,
        key_state: wl_keyboard::KeyState,
    ) {
        let (sym, utf8, repeats) = {
            let mut state = self.state.lock().unwrap();
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
            (self.implementation)(
                Event::Key {
                    serial,
                    time,
                    rawkey: key,
                    keysym: sym,
                    state: key_state,
                    utf8: utf8.clone(),
                },
                object.clone(),
            );
            if repeats {
                self.start_repeat(key, object, time);
            }
        } else {
            self.stop_repeat(Some(key));
            (self.implementation)(
                Event::Key {
                    serial,
                    time,
                    rawkey: key,
                    keysym: sym,
                    state: key_state,
                    utf8: utf8.clone(),
                },
                object,
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
    ) {
        {
            let mut state = self.state.lock().unwrap();
            state.update_modifiers(mods_depressed, mods_latched, mods_locked, group);
            (self.implementation)(
                Event::Modifiers {
                    modifiers: state.mods_state,
                },
                object,
            );
        }
        self.repeat_state_changed();
    }

    fn repeat_info(&mut self, _: wl_keyboard::WlKeyboard, rate: i32, delay: i32) {
        self.set_repeat_timing(rate, delay);
    }
}
