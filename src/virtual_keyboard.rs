use std::fmt::Debug;
use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::wl_seat::WlSeat,
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

use crate::globals::GlobalData;

#[derive(Debug)]
pub struct VirtualKeyboardManager {
    manager: ZwpVirtualKeyboardManagerV1,
}

#[derive(Debug)]
pub struct VirtualKeyboard {}

impl VirtualKeyboardManager {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Self, BindError>
    where
        State: Dispatch<ZwpVirtualKeyboardManagerV1, GlobalData, State> + 'static,
    {
        let manager = globals.bind(qh, 1..=1, GlobalData)?;
        Ok(Self { manager })
    }

    pub fn get_virtual_keyboard<State>(
        &self,
        seat: &WlSeat,
        queue_handle: &QueueHandle<State>,
    ) -> ZwpVirtualKeyboardV1
    where
        State: Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboard, State> + 'static,
    {
        self.manager.create_virtual_keyboard(seat, queue_handle, VirtualKeyboard {})
    }
}

impl<D> Dispatch<ZwpVirtualKeyboardManagerV1, GlobalData, D> for VirtualKeyboardManager
where
    D: Dispatch<ZwpVirtualKeyboardManagerV1, GlobalData>,
{
    fn event(
        _: &mut D,
        _: &ZwpVirtualKeyboardManagerV1,
        _: <ZwpVirtualKeyboardManagerV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zwp_virtual_keyboard_manager had no events")
    }
}

impl<D> Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboard, D> for VirtualKeyboardManager
where
    D: Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboard>,
{
    fn event(
        _: &mut D,
        _: &ZwpVirtualKeyboardV1,
        _: <ZwpVirtualKeyboardV1 as Proxy>::Event,
        _: &VirtualKeyboard,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zwp_virtual_keyboard had no events")
    }
}

#[macro_export]
macro_rules! delegate_virtual_keyboard {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1: $crate::globals::GlobalData
            ] => $crate::virtual_keyboard::VirtualKeyboardManager
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1: $crate::virtual_keyboard::VirtualKeyboard
            ] => $crate::virtual_keyboard::VirtualKeyboardManager
        );
    };
}
