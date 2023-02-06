use std::time::Duration;

use calloop::{
    channel::{self, Channel},
    timer::{TimeoutAction, Timer},
    EventSource, Poll, PostAction, Readiness, Token, TokenFactory,
};
use wayland_client::{
    protocol::{wl_keyboard, wl_seat},
    Dispatch, QueueHandle,
};

use super::{
    KeyEvent, KeyboardData, KeyboardDataExt, KeyboardError, KeyboardHandler, RepeatInfo, RMLVO,
};
use crate::seat::SeatState;

/// Internal repeat message sent to the repeating mechanism.
#[derive(Debug)]
pub(crate) enum RepeatMessage {
    /// Stop the key repeat.
    StopRepeat,

    /// The key event should not have any time added, the repeating mechanism is responsible
    /// for that instead.
    StartRepeat(KeyEvent),

    /// Key has changed during the repeat, but the repeat shouldn't stop.
    KeyChanged(KeyEvent),

    /// The repeat info has changed.
    RepeatInfo(RepeatInfo),
}

/// [`EventSource`] used to emit key repeat events.
#[derive(Debug)]
pub struct KeyRepeatSource {
    channel: Channel<RepeatMessage>,
    timer: Timer,
    gap: Duration,
    delay: Duration,
    disabled: bool,
    key: Option<KeyEvent>,
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
    pub fn get_keyboard_with_repeat<D>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        rmlvo: Option<RMLVO>,
    ) -> Result<(wl_keyboard::WlKeyboard, KeyRepeatSource), KeyboardError>
    where
        D: Dispatch<wl_keyboard::WlKeyboard, KeyboardData> + KeyboardHandler + 'static,
    {
        let udata = match rmlvo {
            Some(rmlvo) => KeyboardData::from_rmlvo(seat.clone(), rmlvo)?,
            None => KeyboardData::new(seat.clone()),
        };

        self.get_keyboard_with_repeat_with_data(qh, seat, udata)
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
    pub fn get_keyboard_with_repeat_with_data<D, U>(
        &mut self,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        mut udata: U,
    ) -> Result<(wl_keyboard::WlKeyboard, KeyRepeatSource), KeyboardError>
    where
        D: Dispatch<wl_keyboard::WlKeyboard, U> + KeyboardHandler + 'static,
        U: KeyboardDataExt + 'static,
    {
        let (repeat_sender, channel) = channel::channel();

        let kbd_data = udata.keyboard_data_mut();
        kbd_data.repeat_sender.replace(repeat_sender);
        kbd_data.init_compose();

        let repeat = KeyRepeatSource {
            channel,
            timer: Timer::immediate(),
            gap: Duration::ZERO,
            delay: Duration::ZERO,
            key: None,
            disabled: true,
        };

        Ok((seat.get_keyboard(qh, udata), repeat))
    }
}

impl EventSource for KeyRepeatSource {
    type Event = KeyEvent;
    type Metadata = ();
    type Ret = ();
    type Error = calloop::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> calloop::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        let mut removed = false;

        let timer = &mut self.timer;
        let gap = &mut self.gap;
        let delay_mut = &mut self.delay;
        let key = &mut self.key;
        let disabled = &mut self.disabled;

        let mut reregister = false;

        // Check if the key repeat should stop
        let channel_pa = self
            .channel
            .process_events(readiness, token, |event, _| {
                match event {
                    channel::Event::Msg(message) => {
                        match message {
                            RepeatMessage::StopRepeat => {
                                key.take();
                            }
                            RepeatMessage::StartRepeat(mut event) => {
                                // Update time for next event, the timestamps are in ms.
                                event.time += delay_mut.as_millis() as u32;
                                key.replace(event);
                                reregister = true;

                                // Schedule a new press event in the timer.
                                timer.set_duration(*delay_mut);
                            }
                            RepeatMessage::KeyChanged(new_event) => {
                                key.replace(new_event);
                            }
                            RepeatMessage::RepeatInfo(info) => {
                                match info {
                                    // Store the repeat time, using it for the next repeat sequence
                                    RepeatInfo::Repeat { rate, delay } => {
                                        *gap = Duration::from_micros(1_000_000 / rate.get() as u64);
                                        *delay_mut = Duration::from_millis(delay as u64);
                                        *disabled = false;
                                        timer.set_duration(*delay_mut);
                                    }

                                    RepeatInfo::Disable => {
                                        // Compositor will send repeat events manually, cancel all repeating events
                                        key.take();
                                        *disabled = true;
                                    }
                                }
                            }
                        }
                    }

                    channel::Event::Closed => {
                        removed = true;
                    }
                }
            })
            .map_err(|err| calloop::Error::OtherError(Box::new(err)))?;

        // Keyboard was destroyed
        if removed {
            return Ok(PostAction::Remove);
        }

        // Re-register the timer to start it again
        if reregister {
            return Ok(PostAction::Reregister);
        }

        let timer_pa = timer.process_events(readiness, token, |mut event, _| {
            if self.disabled || key.is_none() {
                return TimeoutAction::Drop;
            }

            // Invoke the event
            callback(key.clone().unwrap(), &mut ());

            // Update time for next event
            event += *gap;

            // Schedule the next key press
            TimeoutAction::ToDuration(*gap)
        })?;

        // Only disable or remove if both want to, otherwise continue or re-register
        Ok(match (timer_pa, channel_pa) {
            (PostAction::Disable, PostAction::Disable) => PostAction::Disable,
            (PostAction::Remove, PostAction::Remove) => PostAction::Remove,
            (PostAction::Reregister, _) | (_, PostAction::Reregister) => PostAction::Reregister,
            _ => PostAction::Continue,
        })
    }

    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        self.channel.register(poll, token_factory)?;
        self.timer.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        self.channel.reregister(poll, token_factory)?;
        self.timer.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        self.channel.unregister(poll)?;
        self.timer.unregister(poll)
    }
}
