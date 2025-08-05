/*! This implements support for the experimental xx-keyboard-filter-v1 protocol.
 */

pub use protocol::xx_keyboard_filter_manager_v1::XxKeyboardFilterManagerV1;
pub use protocol::xx_keyboard_filter_v1::XxKeyboardFilterV1;
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_experimental::keyboard_filter::v3::client::{
    self as protocol, xx_keyboard_filter_manager_v1, xx_keyboard_filter_v1,
};

use crate::globals::GlobalData;

#[derive(Debug)]
pub struct KeyboardFilterManager {
    manager: XxKeyboardFilterManagerV1,
}

impl KeyboardFilterManager {
    /// Bind the input_method global, if it exists
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Result<Self, BindError>
    where
        D: Dispatch<XxKeyboardFilterManagerV1, GlobalData> + 'static,
    {
        let manager = globals.bind(qh, 1..=1, GlobalData)?;
        Ok(Self { manager })
    }

    /// Request a new keyboard_filter object associated with a given
    /// keyboard, input method, and surface.
    ///
    /// Surface can be any surface, even a dummy one.
    ///
    /// May cause a protocol error if there's a bound keyboard already.
    pub fn bind_to_input_method<D>(
        &self,
        qh: &QueueHandle<D>,
        keyboard: &WlKeyboard,
        input_method: &super::input_method_v3::XxInputMethodV1,
        surface: &WlSurface,
    ) -> KeyboardFilter
    where
        D: Dispatch<XxKeyboardFilterV1, ()> + 'static,
    {
        KeyboardFilter(self.manager.bind_to_input_method(keyboard, input_method, surface, qh, ()))
    }
}

impl<D> Dispatch<XxKeyboardFilterManagerV1, GlobalData, D> for KeyboardFilterManager
where
    D: Dispatch<XxKeyboardFilterManagerV1, GlobalData>,
{
    fn event(
        _data: &mut D,
        _manager: &XxKeyboardFilterManagerV1,
        _event: xx_keyboard_filter_manager_v1::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!("Filter manager receives no events")
    }
}

#[derive(Debug)]
pub struct KeyboardFilter(XxKeyboardFilterV1);

impl KeyboardFilter {
    /// May cause a protocol error if there's no bound keyboard.
    pub fn unbind(&self) {
        self.0.unbind();
    }

    /// May cause a protocol error on invalid serial.
    pub fn filter(&self, serial: u32, action: xx_keyboard_filter_v1::FilterAction) {
        self.0.filter(serial, action);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyboardVersion(pub u32);

impl<D> Dispatch<XxKeyboardFilterV1, (), D> for KeyboardFilter
where
    D: Dispatch<XxKeyboardFilterV1, ()>,
{
    fn event(
        _data: &mut D,
        _keyboard: &XxKeyboardFilterV1,
        _event: xx_keyboard_filter_v1::Event,
        _: &(),
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!("Filter receives no events")
    }
}

#[macro_export]
macro_rules! delegate_keyboard_filter_v1 {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_experimental::keyboard_filter::v3::client::xx_keyboard_filter_manager_v1::XxKeyboardFilterManagerV1: $crate::globals::GlobalData
        ] => $crate::seat::keyboard_filter::KeyboardFilterManager);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols_experimental::keyboard_filter::v3::client::xx_keyboard_filter_v1::XxKeyboardFilterV1: ()
        ] => $crate::seat::keyboard_filter::KeyboardFilter);
    };
}

#[cfg(test)]
mod test {
    use super::*;

    struct Handler {}

    delegate_keyboard_filter_v1!(Handler);

    fn assert_is_manager_delegate<T>()
    where
        T: wayland_client::Dispatch<
            protocol::xx_keyboard_filter_manager_v1::XxKeyboardFilterManagerV1,
            crate::globals::GlobalData,
        >,
    {
    }

    fn assert_is_delegate<T>()
    where
        T: wayland_client::Dispatch<protocol::xx_keyboard_filter_v1::XxKeyboardFilterV1, ()>,
    {
    }

    #[test]
    fn test_valid_assignment() {
        assert_is_manager_delegate::<Handler>();
        assert_is_delegate::<Handler>();
    }
}
