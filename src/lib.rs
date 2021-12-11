#![warn(
//    missing_docs, // Commented out for now so the project isn't all yellow.
    missing_debug_implementations
)]
#![allow(clippy::new_without_default)]

/// Re-exports of some crates, for convenience.
pub mod reexports {
    #[cfg(feature = "calloop")]
    pub use calloop;
    pub use wayland_client as client;
    pub use wayland_protocols as protocols;
}

pub mod output;

/// TODO: Replace this wil wayland-rs delegate_dispatch when it supports fields.
#[macro_export]
macro_rules! delegate_dispatch {
    ($dispatch_from: ty: [$($interface: ty),*] => $dispatch_to: ty ; |$dispatcher: ident| $closure: block) => {
        $(
            impl $crate::Dispatch<$interface> for $dispatch_from {
                type UserData = <$dispatch_to as $crate::DelegateDispatchBase<$interface>>::UserData;

                fn event(
                    &mut self,
                    proxy: &$interface,
                    event: <$interface as $crate::Proxy>::Event,
                    data: &Self::UserData,
                    cxhandle: &mut $crate::ConnectionHandle,
                    qhandle: &$crate::QueueHandle<Self>,
                    init: &mut $crate::DataInit<'_>,
                ) {
                    let $dispatcher = self; // We need to do this so the closure can see the dispatcher field.
                    let delegate: &mut $dispatch_to = { $closure };
                    delegate.event(proxy, event, data, cxhandle, qhandle, init)
                }
            }
        )*
    };
}
