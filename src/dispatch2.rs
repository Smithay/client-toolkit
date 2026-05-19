use std::sync::Arc;
use wayland_client::backend::ObjectData;
use wayland_client::{Connection, Proxy, QueueHandle};

pub use wayland_client::Dispatch as Dispatch2;

#[macro_export]
macro_rules! delegate_dispatch2 {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {};
}
