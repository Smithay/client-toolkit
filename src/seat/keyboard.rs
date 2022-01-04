use wayland_client::{
    protocol::{wl_keyboard, wl_surface},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy, QueueHandle, WEnum,
};

use super::{SeatData, SeatHandler, SeatState};

pub trait KeyboardHandler: SeatHandler + Sized {
    /// The keyboard focus is set to a surface.
    fn keyboard_focus(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
    );

    /// The keyboard focus is removed from a surface.
    fn keyboard_release_focus(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
    );

    fn keyboard_press_key(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        time: u32,
        key: u32,
    );

    fn keyboard_release_key(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        time: u32,
        key: u32,
    );

    fn keyboard_update_modifiers(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        // TODO: Other params
    );

    /// The keyboard has updated the rate and delay between repeating key inputs.
    fn keyboard_update_repeat_info(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        rate: u32,
        delay: u32,
    );
}

#[macro_export]
macro_rules! delegate_keyboard {
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty:
            [
                $crate::reexports::client::protocol::wl_keyboard::WlKeyboard
            ] => $crate::seat::SeatState
        );
    };
}

impl DelegateDispatchBase<wl_keyboard::WlKeyboard> for SeatState {
    type UserData = SeatData;
}

impl<D> DelegateDispatch<wl_keyboard::WlKeyboard, D> for SeatState
where
    D: Dispatch<wl_keyboard::WlKeyboard, UserData = Self::UserData> + KeyboardHandler,
{
    fn event(
        state: &mut D,
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

                        wl_keyboard::KeymapFormat::XkbV1 => {
                            // TODO: Load keymap
                        }

                        _ => unreachable!(),
                    },

                    WEnum::Unknown(value) => {
                        log::warn!(target: "sctk", "Unknown keymap format {:x}", value)
                    }
                }
            }

            wl_keyboard::Event::Enter { serial: _, surface, keys: _ } => {
                // Notify of focus.
                state.keyboard_focus(conn, qh, keyboard, &surface);

                // TODO: Send events to notify of keys being pressed in this event
            }

            wl_keyboard::Event::Leave { serial: _, surface } => {
                // We can send this event without any other checks in the protocol will guarantee a leave is\
                // sent before entering a new surface.
                state.keyboard_release_focus(conn, qh, keyboard, &surface);
            }

            wl_keyboard::Event::Key { serial: _, time, key, state: key_state } => match key_state {
                WEnum::Value(key_state) => match key_state {
                    wl_keyboard::KeyState::Released => {
                        state.keyboard_release_key(conn, qh, keyboard, time, key);
                    }

                    wl_keyboard::KeyState::Pressed => {
                        state.keyboard_press_key(conn, qh, keyboard, time, key);
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
                state.keyboard_update_repeat_info(conn, qh, keyboard, rate as u32, delay as u32);
            }

            _ => unreachable!(),
        }
    }
}
