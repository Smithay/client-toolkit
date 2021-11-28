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

/// TODO: Replace this wil wayland-rs delegate_dispatch when it supports fields.
#[macro_export]
macro_rules! delegate_dispatch_2 {
    // Delegate implementation to another type using a conversion function from the from type.
    ($dispatch_from:ty => $dispatch_to:ty ; [$($interface:ty),*] => $convert:ident) => {
        $(
            impl wayland_client::Dispatch<$interface> for $dispatch_from {
                type UserData = <$dispatch_to as $crate::DelegateDispatchBase<$interface>>::UserData;
                fn event(
                    &mut self,
                    proxy: &$interface,
                    event: <$interface as wayland_client::Proxy>::Event,
                    data: &Self::UserData,
                    cxhandle: &mut wayland_client::ConnectionHandle,
                    qhandle: &wayland_client::QueueHandle<Self>,
                    init: &mut wayland_client::DataInit<'_>,
                ) {
                    <$dispatch_to as wayland_client::DelegateDispatch<$interface, Self>>::event(&mut self.$convert(), proxy, event, data, cxhandle, qhandle, init)
                }
            }
        )*
    };

    // Delegate implementation to another type using a field owned by the from type.
    ($dispatch_from:ty => $dispatch_to:ty ; [$($interface:ty),*] => self.$field:ident) => {
        $(
            impl wayland_client::Dispatch<$interface> for $dispatch_from {
                type UserData = <$dispatch_to as wayland_client::DelegateDispatchBase<$interface>>::UserData;

                fn event(
                    &mut self,
                    proxy: &$interface,
                    event: <$interface as wayland_client::Proxy>::Event,
                    data: &Self::UserData,
                    cxhandle: &mut wayland_client::ConnectionHandle,
                    qhandle: &wayland_client::QueueHandle<Self>,
                    init: &mut wayland_client::DataInit<'_>,
                ) {
                    <$dispatch_to as wayland_client::DelegateDispatch<$interface, Self>>::event(&mut self.$field, proxy, event, data, cxhandle, qhandle, init)
                }
            }
        )*
    };
}
