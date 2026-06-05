//! Utilities for binding globals with [`wl_registry`] in delegates.
//!
//! This module is based around the [`RegistryHandler`] trait and [`RegistryState`].
//!
//! [`RegistryState`] provides an interface to bind globals regularly, creating an object with each new
//! instantiation or caching bound globals to prevent duplicate object instances from being created. Binding
//! a global regularly is accomplished through [`RegistryState::bind_one`].
//!
//! The [`delegate_registry`](crate::delegate_registry) macro is used to implement handling for [`wl_registry`].
//!
//! ## Sample implementation of [`RegistryHandler`]
//!
//! ```
//! use smithay_client_toolkit::reexports::client::{
//!     Connection, Dispatch, QueueHandle,
//!     delegate_dispatch,
//!     globals::GlobalList,
//!     protocol::wl_output,
//! };
//!
//! use smithay_client_toolkit::registry::{
//!     GlobalProxy, ProvidesRegistryState, RegistryHandler, RegistryState,
//! };
//!
//! struct ExampleApp {
//!     /// The registry state is needed to use the global abstractions.
//!     registry_state: RegistryState,
//!     /// This is a type we want to delegate global handling to.
//!     delegate_that_wants_registry: Delegate,
//! }
//!
//! /// The delegate a global should be provided to.
//! struct Delegate {
//!     outputs: Vec<wl_output::WlOutput>,
//! }
//!
//! // When implementing RegistryHandler, you must be able to dispatch any type you could bind using the registry state.
//! impl<D> RegistryHandler<D> for Delegate
//! where
//!     // In order to bind a global, you must statically assert the global may be handled with the data type.
//!     D: Dispatch<wl_output::WlOutput, ()>
//!         // ProvidesRegistryState provides a function to access the RegistryState within the impl.
//!         + ProvidesRegistryState
//!         // We need some way to access our part of the application's state.  This uses AsMut,
//!         // but you may prefer to create your own trait to avoid making .as_mut() ambiguous.
//!         + AsMut<Delegate>
//!         + 'static,
//! {
//!   /// New global added after initial enumeration.
//!    fn new_global(
//!        data: &mut D,
//!        conn: &Connection,
//!        qh: &QueueHandle<D>,
//!        name: u32,
//!        interface: &str,
//!        version: u32,
//!    ) {
//!         if interface == "wl_output" {
//!             // Bind `wl_output` with newest version from 1 to 4 the compositor supports
//!             let output = data.registry().bind_specific(qh, name, 1..=4, ()).unwrap();
//!             data.as_mut().outputs.push(output);
//!         }
//!
//!         // You could either handle errors here or when attempting to use the interface.  Most
//!         // Wayland protocols are optional, so if your application can function without a
//!         // protocol it should try to do so; the From impl of GlobalProxy is written to make
//!         // this straightforward.
//!     }
//! }
//! ```

use crate::{error::GlobalError, globals::ProvidesBoundGlobal};
use wayland_client::{
    globals::{BindError, Global, GlobalList, GlobalListHandler},
    protocol::wl_registry,
    Connection, Dispatch, Proxy, QueueHandle,
};

/// A trait implemented by modular parts of a smithay's client toolkit and protocol delegates that may be used
/// to receive notification of a global being created or destroyed.
///
/// Delegates that choose to implement this trait may be used in [`registry_handlers`] which
/// automatically notifies delegates about the creation and destruction of globals.
///
/// [`registry_handlers`]: crate::registry_handlers
///
/// Note that in order to delegate registry handling to a type which implements this trait, your `D` data type
/// must implement [`ProvidesRegistryState`].
pub trait RegistryHandler<D>
where
    D: GlobalListHandler,
{
    /// Called when a new global has been advertised by the compositor.
    ///
    /// The provided registry handle may be used to bind the global.  This is not called during
    /// initial enumeration of globals. It is primarily useful for multi-instance globals such as
    /// `wl_output` and `wl_seat`.
    ///
    /// The default implementation does nothing.
    fn new_global(
        data: &mut D,
        global_list: &GlobalList,
        conn: &Connection,
        qh: &QueueHandle<D>,
        global: &Global,
    ) {
        let _ = (data, conn, qh, global);
    }

    /// Called when a global has been destroyed by the compositor.
    ///
    /// The default implementation does nothing.
    fn remove_global(
        data: &mut D,
        global_list: &GlobalList,
        conn: &Connection,
        qh: &QueueHandle<D>,
        global: &Global,
    ) {
        let _ = (data, conn, qh, global);
    }
}

/// A helper for storing a bound global.
///
/// This helper is intended to simplify the implementation of [RegistryHandler] for state objects
/// that cache a bound global.
#[derive(Debug)]
pub enum GlobalProxy<I> {
    /// The requested global was not present after a complete enumeration.
    NotPresent,
    /// The cached global.
    Bound(I),
}

impl<I> From<Result<I, BindError>> for GlobalProxy<I> {
    fn from(r: Result<I, BindError>) -> Self {
        match r {
            Ok(proxy) => GlobalProxy::Bound(proxy),
            Err(_) => GlobalProxy::NotPresent,
        }
    }
}

impl<I: Proxy> GlobalProxy<I> {
    pub fn get(&self) -> Result<&I, GlobalError> {
        self.with_min_version(0)
    }

    pub fn with_min_version(&self, min_version: u32) -> Result<&I, GlobalError> {
        match self {
            GlobalProxy::Bound(proxy) => {
                if proxy.version() < min_version {
                    Err(GlobalError::InvalidVersion {
                        name: I::interface().name,
                        required: min_version,
                        available: proxy.version(),
                    })
                } else {
                    Ok(proxy)
                }
            }
            GlobalProxy::NotPresent => Err(GlobalError::MissingGlobal(I::interface().name)),
        }
    }
}

#[derive(Debug)]
pub struct SimpleGlobal<I, const MAX_VERSION: u32> {
    proxy: GlobalProxy<I>,
}

impl<I: Proxy + 'static, const MAX_VERSION: u32> SimpleGlobal<I, MAX_VERSION> {
    pub fn bind_singleton<State>(
        globals: &GlobalList,
        qh: &QueueHandle<State>,
    ) -> Result<Self, BindError>
    where
        State: 'static,
        (): Dispatch<I, State> + 'static,
    {
        let proxy = globals.bind_singleton(qh, 0..=MAX_VERSION, ())?;
        Ok(Self { proxy: GlobalProxy::Bound(proxy) })
    }

    pub fn get(&self) -> Result<&I, GlobalError> {
        self.proxy.get()
    }

    pub fn with_min_version(&self, min_version: u32) -> Result<&I, GlobalError> {
        self.proxy.with_min_version(min_version)
    }

    /// Construct an instance from an already bound proxy.
    ///
    /// Useful when a [`ProvidesBoundGlobal`] implementation is needed.
    pub fn from_bound(proxy: I) -> Self {
        Self { proxy: GlobalProxy::Bound(proxy) }
    }
}

impl<I: Proxy + Clone, const MAX_VERSION: u32> ProvidesBoundGlobal<I, MAX_VERSION>
    for SimpleGlobal<I, MAX_VERSION>
{
    fn bound_global(&self) -> Result<I, GlobalError> {
        self.proxy.get().cloned()
    }
}

/// Binds all globals with a given interface.
pub(crate) fn bind_all<I, D, U, F>(
    registry: &wl_registry::WlRegistry,
    globals: &[Global],
    qh: &QueueHandle<D>,
    version: std::ops::RangeInclusive<u32>,
    mut make_udata: F,
) -> Result<Vec<I>, BindError>
where
    D: 'static,
    I: Proxy + 'static,
    F: FnMut(u32) -> U,
    U: Dispatch<I, D> + Send + Sync + 'static,
{
    let iface = I::interface();
    if *version.end() > iface.version {
        // This is a panic because it's a compile-time programmer error, not a runtime error.
        panic!("Maximum version ({}) was higher than the proxy's maximum version ({}); outdated wayland XML files?",
            version.end(), iface.version);
    }
    let mut rv = Vec::new();
    for global in globals {
        if global.interface != iface.name {
            continue;
        }
        if global.version < *version.start() {
            return Err(BindError::UnsupportedVersion {
                interface: iface.name,
                requested: *version.start(),
                available: global.version,
            });
        }
        let version = global.version.min(*version.end());
        let udata = make_udata(global.name);
        let proxy = registry.bind(global.name, version, qh, udata);
        log::debug!(target: "sctk", "Bound new global [{}] {} v{}", global.name, iface.name, version);

        rv.push(proxy);
    }
    Ok(rv)
}

/// A helper macro for implementing [`GlobalListHandler`].
///
/// See [`delegate_registry`][crate::delegate_registry] for an example.
#[macro_export]
macro_rules! registry_handlers {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $($ty:ty),* $(,)?) => {
        fn runtime_add_global(
            &mut self,
            global_list: &$crate::reexports::client::globals::GlobalList,
            conn: &$crate::reexports::client::Connection,
            qh: &$crate::reexports::client::QueueHandle<Self>,
            global: &$crate::reexports::client::globals::Global,
        ) {
            $(
                <$ty as $crate::registry::RegistryHandler<Self>>::new_global(self, global_list, conn, qh, global);
            )*
        }

        fn runtime_remove_global(
            &mut self,
            global_list: &$crate::reexports::client::globals::GlobalList,
            conn: &$crate::reexports::client::Connection,
            qh: &$crate::reexports::client::QueueHandle<Self>,
            global: &$crate::reexports::client::globals::Global,
        ) {
            $(
                <$ty as $crate::registry::RegistryHandler<Self>>::remove_global(self, global_list, conn, qh, global);
            )*
        }
    }
}
