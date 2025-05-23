/*! This implements support for the experimental xx-input-method-v2 protocol.
 * That protocol will hopefully become -v3 without changing the API at some point.
 */

use crate::compositor::Surface;
use crate::globals::GlobalData;

use log::{debug, warn};

use std::collections::HashMap;
use std::num::Wrapping;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use wayland_client::globals::{BindError, GlobalList};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_surface;
use wayland_client::WEnum;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::{
    ChangeCause, ContentHint, ContentPurpose,
};

use wayland_protocols_experimental::input_method::v1::client as protocol;

pub use protocol::xx_input_method_v1::XxInputMethodV1;
pub use protocol::xx_input_popup_positioner_v1::XxInputPopupPositionerV1;
pub use protocol::xx_input_popup_surface_v2::XxInputPopupSurfaceV2;

use protocol::{
    xx_input_method_manager_v2::{self, XxInputMethodManagerV2},
    xx_input_method_v1, xx_input_popup_positioner_v1, xx_input_popup_surface_v2,
};

pub use xx_input_popup_positioner_v1::{Anchor, Gravity};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct InputMethodManager {
    manager: XxInputMethodManagerV2,
}

impl InputMethodManager {
    /// Bind the input_method global, if it exists
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Result<Self, BindError>
    where
        D: Dispatch<XxInputMethodManagerV2, GlobalData> + 'static,
    {
        let manager = globals.bind(qh, 2..=2, GlobalData)?;
        Ok(Self { manager })
    }

    /// Request a new input_method object associated with a given
    /// seat.
    pub fn get_input_method<State>(&self, qh: &QueueHandle<State>, seat: &WlSeat) -> InputMethod
    where
        State: Dispatch<XxInputMethodV1, InputMethodData, State> + 'static,
    {
        InputMethod {
            input_method: self.manager.get_input_method(
                seat,
                qh,
                InputMethodData::new(seat.clone()),
            ),
        }
    }

    pub fn get_positioner<State>(&self, qh: &QueueHandle<State>) -> PopupPositioner
    where
        State: Dispatch<XxInputPopupPositionerV1, PositionerData, State> + 'static,
    {
        PopupPositioner(self.manager.get_positioner(qh, PositionerData))
    }
}

impl<D> Dispatch<xx_input_method_manager_v2::XxInputMethodManagerV2, GlobalData, D>
    for InputMethodManager
where
    D: Dispatch<xx_input_method_manager_v2::XxInputMethodManagerV2, GlobalData>
        + InputMethodHandler,
{
    fn event(
        _data: &mut D,
        _manager: &xx_input_method_manager_v2::XxInputMethodManagerV2,
        _event: xx_input_method_manager_v2::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

/// A trivial wrapper for an [`XxInputPopupPositionerV1`].
///
/// This wrapper calls [`destroy`][XxInputPopupPositionerV1::destroy] on the contained
/// positioner when it is dropped.
#[derive(Debug)]
pub struct PopupPositioner(XxInputPopupPositionerV1);

impl Deref for PopupPositioner {
    type Target = XxInputPopupPositionerV1;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for PopupPositioner {
    fn drop(&mut self) {
        self.0.destroy()
    }
}

impl<D> Dispatch<XxInputPopupPositionerV1, PositionerData, D> for PopupPositioner
where
    D: Dispatch<XxInputPopupPositionerV1, PositionerData> + InputMethodHandler,
{
    fn event(
        _data: &mut D,
        _manager: &XxInputPopupPositionerV1,
        _event: xx_input_popup_positioner_v1::Event,
        _: &PositionerData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!("Positioner has no events")
    }
}

#[derive(Debug)]
pub struct PositionerData;

#[derive(Debug)]
pub struct InputMethod {
    input_method: XxInputMethodV1,
}

/// Can't set the preedit string due to cursor index not on UTF-8 code point boundary
#[derive(Debug)]
pub enum InvalidIndex {
    /// Only the start index is not on the boundary
    Start,
    /// Only the end index is not on the boundary
    End,
    /// Both the start and end indices are not on boundaries
    Both,
}

impl InputMethod {
    pub fn input_method(&self) -> &XxInputMethodV1 {
        &self.input_method
    }

    pub fn set_preedit_string(
        &self,
        text: String,
        cursor: CursorPosition,
    ) -> Result<(), InvalidIndex> {
        let (start, end) = match cursor {
            CursorPosition::Hidden => (-1, -1),
            CursorPosition::Visible { start, end } => {
                match (text.is_char_boundary(start), text.is_char_boundary(end)) {
                    (true, true) => (
                        // This happens only for cursor values in the upper usize range.
                        // Such values are most likely bugs already,
                        // so it's not a problem if one of the cursors weirdly lands at 0 sometimes.
                        start.try_into().unwrap_or(0),
                        end.try_into().unwrap_or(0),
                    ),
                    (true, false) => {
                        return Err(InvalidIndex::End);
                    }
                    (false, true) => {
                        return Err(InvalidIndex::Start);
                    }
                    (false, false) => {
                        return Err(InvalidIndex::Both);
                    }
                }
            }
        };
        self.input_method.set_preedit_string(text, start, end);
        Ok(())
    }

    pub fn commit_string(&self, text: String) {
        self.input_method.commit_string(text)
    }

    pub fn delete_surrounding_text(&self, before_length: u32, after_length: u32) {
        // TODO: this has 2 separate behaviours:
        // one when surrounding text is supported,
        // and a completely different one when it is not supported
        // and the input method doesn't know what bytes it deletes.
        // Not sure how or whether this should be reflected here.
        self.input_method.delete_surrounding_text(before_length, after_length)
    }

    pub fn commit(&self) {
        let data = self.input_method.data::<InputMethodData>().unwrap();
        let inner = &data.inner.lock().unwrap();
        self.input_method.commit(inner.serial.0)
    }

    pub fn get_input_popup_surface<D>(
        &self,
        qh: &QueueHandle<D>,
        surface: impl Into<Surface>,
        positioner: &PopupPositioner,
    ) -> Popup
    where
        D: Dispatch<XxInputPopupSurfaceV2, PopupData> + 'static,
    {
        let data = self.input_method.data::<InputMethodData>().unwrap();
        let surface = surface.into();
        Popup {
            input_method: self.input_method.clone(),
            popup: self.input_method.get_input_popup_surface(
                surface.wl_surface(),
                &positioner.0,
                qh,
                PopupData { inner: Mutex::new(PopupDataInner::new(Arc::downgrade(&data.inner))) },
            ),
            surface,
        }
    }
}

#[derive(Debug)]
pub struct InputMethodData {
    seat: WlSeat,

    inner: Arc<Mutex<InputMethodDataInner>>,
}

impl InputMethodData {
    /// Create the new touch data associated with the given seat.
    pub fn new(seat: WlSeat) -> Self {
        Self {
            seat,
            inner: Arc::new(Mutex::new(InputMethodDataInner {
                pending_state: Default::default(),
                current_state: Default::default(),
                serial: Wrapping(0),
            })),
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
    pub popups: HashMap<XxInputPopupSurfaceV2, PopupState>,
}

impl Default for InputMethodEventState {
    fn default() -> Self {
        Self {
            surrounding: SurroundingText::default(),
            content_hint: ContentHint::empty(),
            content_purpose: ContentPurpose::Normal,
            text_change_cause: ChangeCause::InputMethod,
            active: Active::default(),
            popups: Default::default(),
        }
    }
}

/// Server-provided popup state
#[derive(Clone, Debug, PartialEq)]
pub struct PopupState {
    /// The position of the anchor relative to top-left corner of the popup
    pub anchor: Rectangle,
    pub size: Size,
    /// serial == None means there is no configure sequence open and attempts to change state must be ignored.
    pub serial: Option<u32>,
    /// The repositioned token from the last sequence
    pub repositioned: Option<u32>,
}

impl PopupState {
    /// Creates an uninitialized copy ready to fill in
    fn new_uninit() -> Self {
        Self {
            // The protocol doesn't allow reading size or anchor before writing, so the values don't matter
            anchor: Rectangle { x: 0, y: 0, width: 0, height: 0 },
            size: Size { width: 0, height: 0 },
            serial: None,
            repositioned: None,
        }
    }

    /// Returns a copy after resetting the fields as required by the protocol on input_method.done.
    fn reset_on_done(&self) -> Self {
        Self { serial: None, repositioned: None, ..self.clone() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CursorPosition {
    Hidden,
    // Bytes relative to the beginning of the text. Must fall on code point boundaries.
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

#[derive(Debug)]
pub struct Popup {
    /// A weak reference to the input method to which this applies
    input_method: XxInputMethodV1,
    popup: XxInputPopupSurfaceV2,
    surface: Surface,
}

impl Popup {
    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        self.surface.wl_surface()
    }

    pub fn input_method(&self) -> &XxInputMethodV1 {
        &self.input_method
    }

    pub fn popup(&self) -> &XxInputPopupSurfaceV2 {
        &self.popup
    }

    pub fn reposition(&self, positioner: &PopupPositioner) {
        let data = self.popup.data::<PopupData>().unwrap();
        let mut inner: MutexGuard<'_, PopupDataInner> = data.inner.lock().unwrap();
        let token = inner.next_token;
        inner.next_token = inner.next_token.wrapping_add(1);
        inner.outstanding_reposition_token = Some(token);
        self.popup.reposition(positioner, token);
    }
}

impl<D> Dispatch<XxInputPopupSurfaceV2, PopupData, D> for Popup
where
    D: Dispatch<XxInputPopupSurfaceV2, PopupData> + InputMethodHandler,
{
    fn event(
        _data: &mut D,
        popup: &XxInputPopupSurfaceV2,
        event: xx_input_popup_surface_v2::Event,
        udata: &PopupData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        let inner: MutexGuard<'_, PopupDataInner> = udata.inner.lock().unwrap();
        if let Some(im) = inner.im.upgrade() {
            let mut im = im.lock().unwrap();

            use xx_input_popup_surface_v2::Event;
            match event {
                Event::Repositioned { token } => {
                    let state = im
                        .pending_state
                        .popups
                        .entry(popup.clone())
                        .or_insert(PopupState::new_uninit());
                    if state.serial.is_some() {
                        state.repositioned = Some(token);
                    } else {
                        warn!(
                            "Repositioned received after im.done but before popup.start_configure"
                        );
                    }
                }
                Event::StartConfigure {
                    width,
                    height,
                    anchor_x,
                    anchor_y,
                    anchor_width,
                    anchor_height,
                    serial,
                } => {
                    let uninit = PopupState::new_uninit();
                    let prev_state = im.pending_state.popups.get(popup).unwrap_or(&uninit);
                    let anchor = Rectangle {
                        x: anchor_x,
                        y: anchor_y,
                        width: anchor_width,
                        height: anchor_height,
                    };
                    let popup_state = PopupState {
                        anchor,
                        serial: Some(serial),
                        size: Size { width, height },
                        ..prev_state.clone()
                    };
                    im.pending_state.popups.insert(popup.clone(), popup_state);
                }
                _ => unreachable!(),
            };
        } else {
            warn!("received event for an input method that already disappeared");
        }
    }
}

/// Data reachable from XxInputPopupSurfaceV2
#[derive(Debug)]
pub struct PopupData {
    // For mutability. Data is immutable.
    inner: Mutex<PopupDataInner>,
}

/// Mutable data reachable from XxInputPopupSurfaceV2
#[derive(Debug)]
struct PopupDataInner {
    im: Weak<Mutex<InputMethodDataInner>>,
    next_token: u32,
    outstanding_reposition_token: Option<u32>,
}

impl PopupDataInner {
    /// Creates a new, uninitialized state
    fn new(im: Weak<Mutex<InputMethodDataInner>>) -> Self {
        Self { im, next_token: 0, outstanding_reposition_token: None }
    }

    /// Returns the newly received token if it's plausibly valid.
    fn update_repositioned(&mut self, state: &PopupState) -> Option<u32> {
        match (state.repositioned, self.outstanding_reposition_token) {
            (Some(_), None) => {
                warn!("Received a repositioned token even though all were already processed. Did one arrive out of order?");
                None
            }
            (None, _) => None,
            (received, Some(outstanding)) => {
                if received == Some(outstanding) {
                    self.outstanding_reposition_token = None
                } else {
                    debug!(
                        "Received a reposition token that is not the most recently requested one."
                    )
                };
                received
            }
        }
    }
}

#[macro_export]
macro_rules! delegate_input_method_v3 {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_experimental::input_method::v1::client::xx_input_method_manager_v2::XxInputMethodManagerV2: $crate::globals::GlobalData
        ] => $crate::seat::input_method_v3::InputMethodManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_experimental::input_method::v1::client::xx_input_method_v1::XxInputMethodV1: $crate::seat::input_method_v3::InputMethodData
        ] => $crate::seat::input_method_v3::InputMethod);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_experimental::input_method::v1::client::xx_input_popup_surface_v2::XxInputPopupSurfaceV2: $crate::seat::input_method_v3::PopupData
        ] => $crate::seat::input_method_v3::Popup);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_experimental::input_method::v1::client::xx_input_popup_positioner_v1::XxInputPopupPositionerV1: $crate::seat::input_method_v3::PositionerData
        ] => $crate::seat::input_method_v3::PopupPositioner);
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
        &mut self,
        qh: &QueueHandle<Self>,
        input_method: &XxInputMethodV1,
        state: &InputMethodEventState,
    );
    /*fn handle_popup_configure(
        &self,
        connection: &Connection,
        qh: &QueueHandle<Self>,
        input_method: &XxInputPopupSurfaceV2,
        state: PopupConfigure,
    );*/
    fn handle_unavailable(&mut self, qh: &QueueHandle<Self>, input_method: &XxInputMethodV1);
}

impl<D, U> Dispatch<XxInputMethodV1, U, D> for InputMethod
where
    D: Dispatch<XxInputMethodV1, U> + InputMethodHandler,
    U: InputMethodDataExt,
{
    fn event(
        data: &mut D,
        input_method: &XxInputMethodV1,
        event: xx_input_method_v1::Event,
        udata: &U,
        _conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let mut imdata: MutexGuard<'_, InputMethodDataInner> =
            udata.input_method_data().inner.lock().unwrap();

        use xx_input_method_v1::Event;

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
                for (popup, state) in imdata.pending_state.popups.iter_mut() {
                    if let Some(serial) = state.serial {
                        popup.ack_configure(serial);
                    }
                    let data = popup.data::<PopupData>().unwrap();
                    {
                        let mut inner: MutexGuard<'_, PopupDataInner> = data.inner.lock().unwrap();
                        inner.update_repositioned(state);
                    }
                    *state = state.clone().reset_on_done();
                }
                imdata.current_state = imdata.pending_state.clone();
                imdata.serial += 1;
                data.handle_done(qh, input_method, &imdata.current_state)
            }
            Event::Unavailable => data.handle_unavailable(qh, input_method),
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
            &mut self,
            _qh: &QueueHandle<Self>,
            _input_method: &XxInputMethodV1,
            _state: &InputMethodEventState,
        ) {
        }

        fn handle_unavailable(&mut self, _qh: &QueueHandle<Self>, _input_method: &XxInputMethodV1) {
        }
    }

    delegate_input_method_v3!(Handler);

    fn assert_is_manager_delegate<T>()
    where
        T: wayland_client::Dispatch<
            protocol::xx_input_method_manager_v2::XxInputMethodManagerV2,
            crate::globals::GlobalData,
        >,
    {
    }

    fn assert_is_delegate<T>()
    where
        T: wayland_client::Dispatch<protocol::xx_input_method_v1::XxInputMethodV1, InputMethodData>,
    {
    }

    fn assert_is_popup_delegate<T>()
    where
        T: wayland_client::Dispatch<
            protocol::xx_input_popup_surface_v2::XxInputPopupSurfaceV2,
            PopupData,
        >,
    {
    }

    fn assert_is_positioner_delegate<T>()
    where
        T: wayland_client::Dispatch<
            protocol::xx_input_popup_positioner_v1::XxInputPopupPositionerV1,
            PositionerData,
        >,
    {
    }

    #[test]
    fn test_valid_assignment() {
        assert_is_manager_delegate::<Handler>();
        assert_is_delegate::<Handler>();
        assert_is_popup_delegate::<Handler>();
        assert_is_positioner_delegate::<Handler>();
    }
}
