use wayland_client::{
    globals::GlobalList,
    protocol::{wl_seat, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::{
    zwp_keyboard_shortcuts_inhibit_manager_v1, zwp_keyboard_shortcuts_inhibitor_v1,
};

use crate::{error::GlobalError, globals::GlobalData, registry::GlobalProxy};

#[derive(Debug)]
pub struct ShortcutsInhibitState {
    shortcuts_inhibit_manager: GlobalProxy<
        zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
    >,
}

impl ShortcutsInhibitState {
    /// Bind `zwp_keyboard_shortcuts_inhibit_manager_v1` global, if it exists
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<
                zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
                GlobalData,
            > + 'static,
    {
        let shortcuts_inhibit_manager = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { shortcuts_inhibit_manager }
    }

    /// Request that keyboard shortcuts are inhibited for surface on given seat.
    ///
    /// Raises protocol error if a shortcut inhibitor already exists for the seat and surface.
    pub fn inhibit_shortcuts<D>(
        &self,
        surface: &wl_surface::WlSurface,
        seat: &wl_seat::WlSeat,
        qh: &QueueHandle<D>,
    ) -> Result<zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1, GlobalError>
    where
        D: Dispatch<
                zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1,
                GlobalData,
            > + 'static,
    {
        Ok(self.shortcuts_inhibit_manager.get()?.inhibit_shortcuts(surface, seat, qh, GlobalData))
    }
}

pub trait ShortcutsInhibitHandler: Sized {
    fn active(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        shortcuts_inhibitor: &zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1,
    );

    fn inactive(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        shortcuts_inhibitor: &zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1,
    );
}

impl<D>
    Dispatch<
        zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
        GlobalData,
        D,
    > for ShortcutsInhibitState
where
    D: Dispatch<
        zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
        GlobalData,
    >,
{
    fn event(
        _data: &mut D,
        _manager: &zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
        _event: zwp_keyboard_shortcuts_inhibit_manager_v1::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1, GlobalData, D>
    for ShortcutsInhibitState
where
    D: Dispatch<zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1, GlobalData>
        + ShortcutsInhibitHandler,
{
    fn event(
        data: &mut D,
        inhibitor: &zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1,
        event: zwp_keyboard_shortcuts_inhibitor_v1::Event,
        _: &GlobalData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zwp_keyboard_shortcuts_inhibitor_v1::Event::Active => data.active(conn, qh, inhibitor),
            zwp_keyboard_shortcuts_inhibitor_v1::Event::Inactive => {
                data.inactive(conn, qh, inhibitor)
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_shortcuts_inhibit {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1: $crate::globals::GlobalData
        ] => $crate::seat::shortcuts_inhibit::ShortcutsInhibitState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1: $crate::globals::GlobalData
        ] => $crate::seat::shortcuts_inhibit::ShortcutsInhibitState);
    };
}
