use std::sync::Arc;
use wayland_client::backend::ObjectData;
use wayland_client::{Connection, Proxy, QueueHandle};

pub trait Dispatch2<I: Proxy, State> {
    fn event(
        &self,
        _: &mut State,
        _: &I,
        _: <I as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    );

    fn event_created_child(opcode: u16, _qh: &QueueHandle<State>) -> Arc<dyn ObjectData> {
        panic!(
            "Missing event_created_child specialization for event opcode {} of {}",
            opcode,
            I::interface().name
        );
    }
}

#[macro_export]
macro_rules! delegate_dispatch2 {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        impl<$( $( $lt $( : $clt $(+ $dlt )* )? ),+, )? I, UserData> $crate::reexports::client::Dispatch<I, UserData> for $ty
        where
            I: $crate::reexports::client::Proxy,
            UserData: $crate::dispatch2::Dispatch2<I, $ty> {
            fn event(
                state: &mut $ty,
                proxy: &I,
                event: <I as $crate::reexports::client::Proxy>::Event,
                data: &UserData,
                conn: &$crate::reexports::client::Connection,
                qh: &$crate::reexports::client::QueueHandle<$ty>,
            ) {
                data.event(state, proxy, event, conn, qh);
            }

            fn event_created_child(opcode: u16, qh: &$crate::reexports::client::QueueHandle<$ty>) -> ::std::sync::Arc<dyn $crate::reexports::client::backend::ObjectData> {
                UserData::event_created_child(opcode, qh)
            }
        }
    };
}
