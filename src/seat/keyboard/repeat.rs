use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};

use calloop::{
    channel::{self, Channel},
    timer::Timer,
    EventSource, Poll, PostAction, Readiness, Token, TokenFactory,
};
use wayland_client::{
    protocol::{wl_keyboard, wl_seat},
    ConnectionHandle, Dispatch, QueueHandle,
};
use xkbcommon::xkb;

use crate::seat::{keyboard::RepeatInfo, Capability, SeatError, SeatState};

use super::{KeyEvent, KeyboardData, KeyboardError, KeyboardHandler, RepeatMessage, RMLVO};

/// [`EventSource`] used to emit key repeat events.
#[derive(Debug)]
pub struct KeyRepeatSource {
    channel: Channel<RepeatMessage>,
    timer: Timer<KeyEvent>,
    /// Gap in time to the next key event in milliseconds.
    gap: u64,
    delay: u64,
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
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        seat: &wl_seat::WlSeat,
        rmlvo: Option<RMLVO>,
    ) -> Result<(wl_keyboard::WlKeyboard, KeyRepeatSource), KeyboardError>
    where
        D: Dispatch<wl_keyboard::WlKeyboard, UserData = KeyboardData> + KeyboardHandler + 'static,
    {
        let inner =
            self.seats.iter().find(|inner| &inner.seat == seat).ok_or(SeatError::DeadObject)?;

        if !inner.data.has_keyboard.load(Ordering::SeqCst) {
            return Err(SeatError::UnsupportedCapability(Capability::Keyboard).into());
        }

        let xkb_context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let (user_specified_rmlvo, xkb_state) = if let Some(rmlvo) = rmlvo {
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

            let state = xkb::State::new(&keymap.unwrap());

            (true, Some(state))
        } else {
            (false, None)
        };

        let (repeat_sender, channel) = channel::channel();

        let udata = KeyboardData {
            first_event: AtomicBool::new(false),
            xkb_context: Mutex::new(xkb_context),
            xkb_state: Mutex::new(xkb_state),
            user_specified_rmlvo,
            xkb_compose: Mutex::new(None),
            repeat_sender: Some(repeat_sender),
        };

        udata.init_compose();

        let repeat = KeyRepeatSource { channel, timer: Timer::new()?, gap: 0, delay: 0 };

        Ok((seat.get_keyboard(conn, qh, udata).map_err(Into::<SeatError>::into)?, repeat))
    }
}

impl EventSource for KeyRepeatSource {
    type Event = KeyEvent;
    type Metadata = ();
    type Ret = ();

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        let mut removed = false;

        let timer = &mut self.timer;
        let gap = &mut self.gap;
        let delay_mut = &mut self.delay;

        // Check if the key repeat should stop
        self.channel.process_events(readiness, token, |event, _| {
            match event {
                channel::Event::Msg(message) => {
                    match message {
                        RepeatMessage::StopRepeat => {
                            timer.handle().cancel_all_timeouts();
                        }

                        RepeatMessage::StartRepeat(mut event) => {
                            // Update time for next event
                            event.time += *delay_mut as u32;
                            // Schedule a new press event in the timer.
                            timer.handle().add_timeout(Duration::from_millis(*delay_mut), event);
                        }

                        RepeatMessage::RepeatInfo(info) => {
                            match info {
                                // Store the repeat time, using it for the next repeat sequence.
                                RepeatInfo::Repeat { rate, delay } => {
                                    // Number of repetitions per second / 1000 ms
                                    *gap = (rate.get() / 1000) as u64;
                                    *delay_mut = delay as u64;
                                }

                                RepeatInfo::Disable => {
                                    // Compositor will send repeat events manually, cancel all repeating events
                                    timer.handle().cancel_all_timeouts();
                                }
                            }
                        }
                    }
                }

                channel::Event::Closed => {
                    removed = true;
                }
            }
        })?;

        // Keyboard was destroyed
        if removed {
            return Ok(PostAction::Remove);
        }

        timer.process_events(readiness, token, |mut event, timer_handle| {
            // Invoke the event
            callback(event.clone(), &mut ());

            // Update time for next event
            event.time += *gap as u32;
            // Schedule the next key press
            timer_handle.add_timeout(Duration::from_micros(*gap), event);
        })
    }

    fn register(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        self.channel.register(poll, token_factory)?;
        self.timer.register(poll, token_factory)
    }

    fn reregister(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        self.channel.reregister(poll, token_factory)?;
        self.timer.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> io::Result<()> {
        self.channel.unregister(poll)?;
        self.timer.unregister(poll)
    }
}
