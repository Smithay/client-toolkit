use std::sync::atomic::Ordering;

use calloop::{LoopHandle, RegistrationToken};
use wayland_client::{
    protocol::{
        wl_keyboard::{self, WlKeyboard},
        wl_seat, wl_surface,
    },
    Dispatch, QueueHandle,
};

use super::{
    Capability, KeyEvent, KeyboardData, KeyboardDataExt, KeyboardError, KeyboardHandler,
    RepeatInfo, SeatError, RMLVO,
};
use crate::seat::SeatState;

pub(crate) struct RepeatedKey {
    pub(crate) key: KeyEvent,
    /// Whether this is the first event of the repeat sequence.
    pub(crate) is_first: bool,
    pub(crate) surface: wl_surface::WlSurface,
}

pub type RepeatCallback<T> = Box<dyn FnMut(&mut T, &WlKeyboard, KeyEvent) + 'static>;

pub(crate) struct RepeatData<T> {
    pub(crate) current_repeat: Option<RepeatedKey>,
    pub(crate) repeat_info: RepeatInfo,
    pub(crate) loop_handle: LoopHandle<'static, T>,
    pub(crate) callback: RepeatCallback<T>,
    pub(crate) repeat_token: Option<RegistrationToken>,
}

impl<T> Drop for RepeatData<T> {
    fn drop(&mut self) {
        if let Some(token) = self.repeat_token.take() {
            self.loop_handle.remove(token);
        }
    }
}

impl SeatState {
    /// Creates a keyboard from a seat.
    ///
    /// This function returns an [`EventSource`] that indicates when a key press is going to repeat.
    ///
    /// This keyboard implementation uses libxkbcommon for the keymap.
    ///
    /// Typically the compositor will provide a keymap, but you may specify your own keymap using the `rmlvo`
    /// field.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a keyboard.
    ///
    /// [`EventSource`]: calloop::EventSource
    pub fn get_keyboard_with_repeat<D, T>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        rmlvo: Option<RMLVO>,
        loop_handle: LoopHandle<'static, T>,
        callback: RepeatCallback<T>,
    ) -> Result<wl_keyboard::WlKeyboard, KeyboardError>
    where
        D: Dispatch<wl_keyboard::WlKeyboard, KeyboardData<T>> + KeyboardHandler + 'static,
        T: 'static,
    {
        let udata = match rmlvo {
            Some(rmlvo) => KeyboardData::from_rmlvo(seat.clone(), rmlvo)?,
            None => KeyboardData::new(seat.clone()),
        };

        self.get_keyboard_with_repeat_with_data(qh, seat, udata, loop_handle, callback)
    }

    /// Creates a keyboard from a seat.
    ///
    /// This function returns an [`EventSource`] that indicates when a key press is going to repeat.
    ///
    /// This keyboard implementation uses libxkbcommon for the keymap.
    ///
    /// Typically the compositor will provide a keymap, but you may specify your own keymap using the `rmlvo`
    /// field.
    ///
    /// ## Errors
    ///
    /// This will return [`SeatError::UnsupportedCapability`] if the seat does not support a keyboard.
    ///
    /// [`EventSource`]: calloop::EventSource
    pub fn get_keyboard_with_repeat_with_data<D, U>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        mut udata: U,
        loop_handle: LoopHandle<'static, <U as KeyboardDataExt>::State>,
        callback: RepeatCallback<<U as KeyboardDataExt>::State>,
    ) -> Result<wl_keyboard::WlKeyboard, KeyboardError>
    where
        D: Dispatch<wl_keyboard::WlKeyboard, U> + KeyboardHandler + 'static,
        U: KeyboardDataExt + 'static,
    {
        let inner =
            self.seats.iter().find(|inner| &inner.seat == seat).ok_or(SeatError::DeadObject)?;

        if !inner.data.has_keyboard.load(Ordering::SeqCst) {
            return Err(SeatError::UnsupportedCapability(Capability::Keyboard).into());
        }

        let kbd_data = udata.keyboard_data_mut();
        kbd_data.repeat_data.lock().unwrap().replace(RepeatData {
            current_repeat: None,
            repeat_info: RepeatInfo::Disable,
            loop_handle: loop_handle.clone(),
            callback,
            repeat_token: None,
        });
        kbd_data.init_compose();

        Ok(seat.get_keyboard(qh, udata))
    }
}
