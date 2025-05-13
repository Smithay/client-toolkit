//! Implementation of the `input-method-unstable-v2` protocol.
//!
//! This protocol allows applications to act as input methods for compositors.
//!
//! ### Implementation status
//! Currently only the input-method object is supported. No keyboard grab, no popup surface.

use crate::globals::GlobalData;

use log::warn;

use std::num::Wrapping;
use std::sync::Mutex;

use wayland_client::globals::{BindError, GlobalList};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::WEnum;

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::{
    ChangeCause, ContentHint, ContentPurpose,
};
pub use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_v2::ZwpInputMethodV2;
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_manager_v2::{self, ZwpInputMethodManagerV2},
    zwp_input_method_v2,
};

#[derive(Debug)]
pub struct InputMethodManager {
    manager: ZwpInputMethodManagerV2,
}

impl InputMethodManager {
    /// Bind `zwp_input_method_v2` global, if it exists
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Result<Self, BindError>
    where
        D: Dispatch<ZwpInputMethodManagerV2, GlobalData> + 'static,
    {
        let manager = globals.bind(qh, 1..=1, GlobalData)?;
        Ok(Self { manager })
    }

    /// Request a new input zwp_input_method_v2 object associated with a given
    /// seat.
    pub fn get_input_method<State>(&self, qh: &QueueHandle<State>, seat: &WlSeat) -> InputMethod
    where
        State: Dispatch<ZwpInputMethodV2, InputMethodData, State> + 'static,
    {
        InputMethod {
            input_method: self.manager.get_input_method(
                seat,
                qh,
                InputMethodData::new(seat.clone()),
            ),
        }
    }
}

impl<D> Dispatch<zwp_input_method_manager_v2::ZwpInputMethodManagerV2, GlobalData, D>
    for InputMethodManager
where
    D: Dispatch<zwp_input_method_manager_v2::ZwpInputMethodManagerV2, GlobalData>
        + InputMethodHandler,
{
    fn event(
        _data: &mut D,
        _manager: &zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
        _event: zwp_input_method_manager_v2::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

#[derive(Debug)]
pub struct InputMethod {
    input_method: ZwpInputMethodV2,
}

impl InputMethod {
    pub fn set_preedit_string(&self, text: String, cursor: CursorPosition) {
        // TODO: should this enforce indices on codepoint boundaries?
        let (start, end) = match cursor {
            CursorPosition::Hidden => (-1, -1),
            CursorPosition::Visible { start, end } => (
                // This happens only for cursor values in the upper usize range.
                // Such values are most likely bugs already,
                // so it's not a problem if one of the cursors weirdly lands at 0 sometimes.
                start.try_into().unwrap_or(0),
                end.try_into().unwrap_or(0),
            ),
        };
        self.input_method.set_preedit_string(text, start, end)
    }

    pub fn commit_string(&self, text: String) {
        self.input_method.commit_string(text)
    }

    pub fn delete_surrounding_text(&self, before_length: u32, after_length: u32) {
        // TODO: this has 2 separate behaviours:
        // one when preedit text is supported,
        // and a completely different one when it is not supported
        // and the input method doesn't know what bytes it deletes.
        // Not sure how or whether this should be reflected here.
        self.input_method.delete_surrounding_text(before_length, after_length)
    }

    pub fn commit(&self) {
        let data = self.input_method.data::<InputMethodData>().unwrap();
        let inner = data.inner.lock().unwrap();
        self.input_method.commit(inner.serial.0)
    }
}

#[derive(Debug)]
pub struct InputMethodData {
    seat: WlSeat,

    inner: Mutex<InputMethodDataInner>,
}

impl InputMethodData {
    /// Create the new input method data associated with the given seat.
    pub fn new(seat: WlSeat) -> Self {
        Self {
            seat,
            inner: Mutex::new(InputMethodDataInner {
                pending_state: Default::default(),
                current_state: Default::default(),
                serial: Wrapping(0),
            }),
        }
    }

    /// Get the associated seat from the data.
    pub fn seat(&self) -> &WlSeat {
        &self.seat
    }
}

#[derive(Debug)]
struct InputMethodDataInner {
    pending_state: InputMethodEventState,
    current_state: InputMethodEventState,
    serial: Wrapping<u32>,
}

/// Stores incoming interface state.
#[derive(Debug, Clone, PartialEq)]
pub struct InputMethodEventState {
    pub surrounding: SurroundingText,
    pub content_purpose: ContentPurpose,
    pub content_hint: ContentHint,
    pub text_change_cause: ChangeCause,
    pub active: Active,
}

impl Default for InputMethodEventState {
    fn default() -> Self {
        Self {
            surrounding: SurroundingText::default(),
            content_hint: ContentHint::empty(),
            content_purpose: ContentPurpose::Normal,
            text_change_cause: ChangeCause::InputMethod,
            active: Active::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CursorPosition {
    Hidden,
    Visible { start: usize, end: usize },
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct SurroundingText {
    pub text: String,
    pub cursor: u32,
    pub anchor: u32,
}

/// State machine for determining the capabilities of a text input
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Active {
    Inactive,
    NegotiatingCapabilities { surrounding_text: bool, content_type: bool },
    Active { surrounding_text: bool, content_type: bool },
}

impl Default for Active {
    fn default() -> Self {
        Self::Inactive
    }
}

impl Active {
    fn with_active(self) -> Self {
        match self {
            Self::Inactive => {
                Self::NegotiatingCapabilities { content_type: false, surrounding_text: false }
            }
            other => other,
        }
    }

    fn with_surrounding_text(self) -> Self {
        match self {
            Self::Inactive => Self::Inactive,
            Self::NegotiatingCapabilities { content_type, .. } => {
                Self::NegotiatingCapabilities { content_type, surrounding_text: true }
            }
            active @ Self::Active { .. } => active,
        }
    }

    fn with_content_type(self) -> Self {
        match self {
            Self::Inactive => Self::Inactive,
            Self::NegotiatingCapabilities { surrounding_text, .. } => {
                Self::NegotiatingCapabilities { content_type: true, surrounding_text }
            }
            active @ Self::Active { .. } => active,
        }
    }

    fn with_done(self) -> Self {
        match self {
            Self::Inactive => Self::Inactive,
            Self::NegotiatingCapabilities { surrounding_text, content_type } => {
                Self::Active { content_type, surrounding_text }
            }
            active @ Self::Active { .. } => active,
        }
    }
}

#[macro_export]
macro_rules! delegate_input_method {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_misc::zwp_input_method_v2::client::zwp_input_method_manager_v2::ZwpInputMethodManagerV2: $crate::globals::GlobalData
        ] => $crate::seat::input_method::InputMethodManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_misc::zwp_input_method_v2::client::zwp_input_method_v2::ZwpInputMethodV2: $crate::seat::input_method::InputMethodData
        ] => $crate::seat::input_method::InputMethod);
    };
}

pub trait InputMethodDataExt: Send + Sync {
    fn input_method_data(&self) -> &InputMethodData;
}

impl InputMethodDataExt for InputMethodData {
    fn input_method_data(&self) -> &InputMethodData {
        self
    }
}

pub trait InputMethodHandler: Sized {
    fn handle_done(
        &self,
        connection: &Connection,
        qh: &QueueHandle<Self>,
        input_method: &ZwpInputMethodV2,
        state: &InputMethodEventState,
    );
    fn handle_unavailable(
        &self,
        connection: &Connection,
        qh: &QueueHandle<Self>,
        input_method: &ZwpInputMethodV2,
    );
}

impl<D, U> Dispatch<ZwpInputMethodV2, U, D> for InputMethod
where
    D: Dispatch<ZwpInputMethodV2, U> + InputMethodHandler,
    U: InputMethodDataExt,
{
    fn event(
        data: &mut D,
        input_method: &ZwpInputMethodV2,
        event: zwp_input_method_v2::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let mut imdata: std::sync::MutexGuard<'_, InputMethodDataInner> =
            udata.input_method_data().inner.lock().unwrap();

        use zwp_input_method_v2::Event;

        match event {
            Event::Activate => {
                imdata.pending_state = InputMethodEventState {
                    active: imdata.pending_state.active.with_active(),
                    ..Default::default()
                };
            }
            Event::Deactivate => {
                imdata.pending_state = Default::default();
            }
            Event::SurroundingText { text, cursor, anchor } => {
                imdata.pending_state = InputMethodEventState {
                    active: imdata.pending_state.active.with_surrounding_text(),
                    surrounding: SurroundingText { text, cursor, anchor },
                    ..imdata.pending_state.clone()
                }
            }
            Event::TextChangeCause { cause } => {
                imdata.pending_state = InputMethodEventState {
                    text_change_cause: match cause {
                        WEnum::Value(cause) => cause,
                        WEnum::Unknown(value) => {
                            warn!(
                                "Unknown `text_change_cause`: {}. Assuming not input method.",
                                value
                            );
                            ChangeCause::Other
                        }
                    },
                    ..imdata.pending_state.clone()
                }
            }
            Event::ContentType { hint, purpose } => {
                imdata.pending_state = InputMethodEventState {
                    active: imdata.pending_state.active.with_content_type(),
                    content_hint: match hint {
                        WEnum::Value(hint) => hint,
                        WEnum::Unknown(value) => {
                            warn!(
                                "Unknown content hints: 0b{:b}, ignoring.",
                                ContentHint::from_bits_retain(value)
                                    - ContentHint::from_bits_truncate(value)
                            );
                            ContentHint::from_bits_truncate(value)
                        }
                    },
                    content_purpose: match purpose {
                        WEnum::Value(v) => v,
                        WEnum::Unknown(value) => {
                            warn!("Unknown `content_purpose`: {}. Assuming `normal`.", value);
                            ContentPurpose::Normal
                        }
                    },
                    ..imdata.pending_state.clone()
                }
            }
            Event::Done => {
                imdata.pending_state = InputMethodEventState {
                    active: imdata.pending_state.active.with_done(),
                    ..imdata.pending_state.clone()
                };
                imdata.current_state = imdata.pending_state.clone();
                imdata.serial += 1;
                data.handle_done(conn, qh, input_method, &imdata.current_state)
            }
            Event::Unavailable => data.handle_unavailable(conn, qh, input_method),
            _ => unreachable!(),
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct Handler {}

    impl InputMethodHandler for Handler {
        fn handle_done(
            &self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _input_method: &ZwpInputMethodV2,
            _state: &InputMethodEventState,
        ) {
        }

        fn handle_unavailable(
            &self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _input_method: &ZwpInputMethodV2,
        ) {
        }
    }

    delegate_input_method!(Handler);

    fn assert_is_manager_delegate<T>()
    where
        T: wayland_client::Dispatch<ZwpInputMethodManagerV2, crate::globals::GlobalData>,
    {
    }

    fn assert_is_delegate<T>()
    where
        T: wayland_client::Dispatch<ZwpInputMethodV2, InputMethodData>,
    {
    }

    #[test]
    fn test_valid_assignment() {
        assert_is_manager_delegate::<Handler>();
        assert_is_delegate::<Handler>();
    }
}
