//! Utilities for binding globals with [`wl_registry`] in delegates.
//!
//! This module is based around the [`RegistryHandler`] trait and [`RegistryState`].
//!
//! [`RegistryState`] provides an interface to bind globals regularly, creating an object with each new
//! instantiation or caching bound globals to prevent duplicate object instances from being created. Binding
//! a global regularly is accomplished through [`RegistryState::bind_once`]. For caching a bound global use
//! [`RegistryState::bind_cached`].
//!
//! The [`delegate_registry`] macro is used to implement handling for [`wl_registry`].
//!
//! ## Sample implementation of [`RegistryHandler`]
//!
//! ```
//! use smithay_client_toolkit::reexports::client::{
//!     Connection, Dispatch, QueueHandle,
//!     delegate_dispatch,
//!     protocol::wl_compositor,
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
//!     // You usually want to cache the bound global so you can use it later
//!     compositor: GlobalProxy<wl_compositor::WlCompositor>,
//! }
//!
//! // When implementing RegistryHandler, you must be able to dispatch any type you could bind using the registry state.
//! impl<D> RegistryHandler<D> for Delegate
//! where
//!     // In order to bind a global, you must statically assert the global may be handled with the data type.
//!     D: Dispatch<wl_compositor::WlCompositor, ()>
//!         // ProvidesRegistryState provides a function to access the RegistryState within the impl.
//!         + ProvidesRegistryState
//!         // We need some way to access our part of the application's state.  This uses AsMut,
//!         // but you may prefer to create your own trait to avoid making .as_mut() ambiguous.
//!         + AsMut<Delegate>
//!         + 'static,
//! {
//!     // When all globals have been enumerated, this is called.
//!     fn ready(
//!         data: &mut D,
//!         conn: &Connection,
//!         qh: &QueueHandle<D>,
//!     ) {
//!         // Bind the global and store it in our state.
//!         data.as_mut().compositor = data.registry().bind_one(
//!             qh,
//!             1..=2, // we want to bind version 1 or 2 of the global.
//!             (), // and we provide the user data for the wl_compositor being created.
//!         ).into();
//!
//!         // You could either handle errors here or when attempting to use the interface.  Most
//!         // Wayland protocols are optional, so if your application can function without a
//!         // protocol it should try to do so; the From impl of GlobalProxy is written to make
//!         // this straightforward.
//!     }
//! }
//! ```

use crate::error::GlobalError;
use wayland_client::{
    backend::InvalidId,
    protocol::{wl_callback, wl_registry},
    Connection, DelegateDispatch, Dispatch, Proxy, QueueHandle,
};

/// A trait implemented by modular parts of a smithay's client toolkit and protocol delegates that may be used
/// to receive notification of a global being created or destroyed.
///
/// Delegates that choose to implement this trait may be used in [`registry_handlers`] which
/// automatically notifies delegates about the creation and destruction of globals.
///
/// Note that in order to delegate registry handling to a type which implements this trait, your `D` data type
/// must implement [`ProvidesRegistryState`].
pub trait RegistryHandler<D>
where
    D: ProvidesRegistryState,
{
    /// Called when initial enumeration of globals has been completed.
    ///
    /// This should be used to bind capability globals.
    fn ready(data: &mut D, conn: &Connection, qh: &QueueHandle<D>);

    /// Called when a new global has been advertised by the compositor.
    ///
    /// The provided registry handle may be used to bind the global.  This is not called during
    /// initial enumeration of globals, only for globals added after the calls to
    /// [`Registryhandler::ready`].  It is primarily useful for multi-instance globals such as
    /// `wl_output` and `wl_seat`.
    ///
    /// The default implementation does nothing.
    fn new_global(
        data: &mut D,
        conn: &Connection,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        let _ = (data, conn, qh, name, interface, version);
    }

    /// Called when a global has been destroyed by the compositor.
    ///
    /// The default implementation does nothing.
    fn remove_global(
        data: &mut D,
        conn: &Connection,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
    ) {
        let _ = (data, conn, qh, name, interface);
    }
}

/// Trait which asserts a data type may provide a mutable reference to the registry state.
///
/// Typically this trait will be required by delegates or [`RegistryHandler`] implementations which need
/// to access the registry utilities provided by Smithay's client toolkit.
pub trait ProvidesRegistryState: Sized {
    /// Returns a mutable reference to the registry state.
    fn registry(&mut self) -> &mut RegistryState;

    /// Called when initial enumeration of globals has been completed.
    fn global_enumeration_finished(&mut self, conn: &Connection, qh: &QueueHandle<Self>);

    /// Called when a new global has been advertised by the compositor.
    ///
    /// This is not called during initial global enumeration.
    fn runtime_add_global(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        name: u32,
        interface: &str,
        version: u32,
    );

    /// Called when a global has been destroyed by the compositor.
    fn runtime_remove_global(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        name: u32,
        interface: &str,
    );
}

/// An error when binding a global.
#[derive(Debug, thiserror::Error)]
pub enum BindError {
    /// The requested version of the global is not supported.
    #[error("the requested version of the global is not supported")]
    UnsupportedVersion,

    /// Protocol error.
    #[error(transparent)]
    Protocol(#[from] InvalidId),

    /// The requested global was not found in the registry.
    #[error("the requested global was not found in the registry")]
    NotPresent,
}

/// State object associated with the registry handling for smithay's client toolkit.
///
/// This object provides utilities to cache bound globals that are needed by multiple modules.
#[derive(Debug)]
pub struct RegistryState {
    registry: wl_registry::WlRegistry,
    globals: Vec<Global>,
    ready: bool,
}

#[derive(Debug)]
struct Global {
    interface: String,
    version: u32,
    name: u32,
}

impl RegistryState {
    /// Creates a new registry handle.
    ///
    /// This type may be used to bind globals as they are advertised.
    pub fn new<D>(conn: &Connection, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<wl_registry::WlRegistry, ()>
            + Dispatch<wl_callback::WlCallback, RegistryReady>
            + ProvidesRegistryState
            + 'static,
    {
        let display = conn.display();
        let registry = display.get_registry(qh, ()).unwrap();
        display.sync(qh, RegistryReady).unwrap();
        RegistryState { registry, globals: Vec::new(), ready: false }
    }

    /// Uses an existing WlRegistry for handling registry state.
    ///
    /// Note: prefer using [Self::new] unless you need access to the registry for other reasons.
    ///
    /// You will need to ensure the RegistryReady signal is sent to this object after initial
    /// enumeration of the registry is complete.
    pub fn from_registry(registry: wl_registry::WlRegistry) -> Self {
        RegistryState { registry, globals: Vec::new(), ready: false }
    }

    pub fn registry(&self) -> &wl_registry::WlRegistry {
        &self.registry
    }

    /// Returns true if the registry has completed the initial enumeration of globals and is ready
    /// to serve bind requests.
    pub fn ready(&self) -> bool {
        self.ready
    }

    /// Binds a global, returning a new object associated with the global.
    ///
    /// This should not be used to bind globals that have multiple instances such as `wl_output`;
    /// use [Self::bind_all] instead.
    pub fn bind_one<I, D, U>(
        &self,
        qh: &QueueHandle<D>,
        version: std::ops::RangeInclusive<u32>,
        udata: U,
    ) -> Result<I, BindError>
    where
        D: Dispatch<I, U> + 'static,
        I: Proxy + 'static,
        U: Send + Sync + 'static,
    {
        let iface = I::interface();
        if *version.end() > iface.version {
            // This is a panic because it's a compile-time programmer error, not a runtime error.
            panic!("Maximum version ({}) was higher than the proxy's maximum version ({}); outdated wayland XML files?",
                version.end(), iface.version);
        }
        for global in &self.globals {
            if global.interface != iface.name {
                continue;
            }
            if global.version < *version.start() {
                return Err(BindError::UnsupportedVersion);
            }
            let version = global.version.min(*version.end());
            let proxy = self.registry.bind(global.name, version, qh, udata)?;
            log::debug!(target: "sctk", "Bound new global [{}] {} v{}", global.name, iface.name, version);

            return Ok(proxy);
        }
        Err(BindError::NotPresent)
    }

    /// Binds a global, returning a new object associated with the global.
    ///
    /// This binds a specific object by its name as provided by the [RegistryHandler::new_global]
    /// callback.
    pub fn bind_specific<I, D, U>(
        &self,
        qh: &QueueHandle<D>,
        name: u32,
        version: std::ops::RangeInclusive<u32>,
        udata: U,
    ) -> Result<I, BindError>
    where
        D: Dispatch<I, U> + 'static,
        I: Proxy + 'static,
        U: Send + Sync + 'static,
    {
        let iface = I::interface();
        if *version.end() > iface.version {
            // This is a panic because it's a compile-time programmer error, not a runtime error.
            panic!("Maximum version ({}) was higher than the proxy's maximum version ({}); outdated wayland XML files?",
                version.end(), iface.version);
        }
        // Optimize for runtime_add_global which will use the last entry
        for global in self.globals.iter().rev() {
            if global.name != name || global.interface != iface.name {
                continue;
            }
            if global.version < *version.start() {
                return Err(BindError::UnsupportedVersion);
            }
            let version = global.version.min(*version.end());
            let proxy = self.registry.bind(global.name, version, qh, udata)?;
            log::debug!(target: "sctk", "Bound new global [{}] {} v{}", global.name, iface.name, version);

            return Ok(proxy);
        }
        Err(BindError::NotPresent)
    }

    /// Binds all globals with a given interface.
    pub fn bind_all<I, D, U, F>(
        &self,
        qh: &QueueHandle<D>,
        version: std::ops::RangeInclusive<u32>,
        mut make_udata: F,
    ) -> Result<Vec<I>, BindError>
    where
        D: Dispatch<I, U> + 'static,
        I: Proxy + 'static,
        F: FnMut(u32) -> U,
        U: Send + Sync + 'static,
    {
        let iface = I::interface();
        if *version.end() > iface.version {
            // This is a panic because it's a compile-time programmer error, not a runtime error.
            panic!("Maximum version ({}) was higher than the proxy's maximum version ({}); outdated wayland XML files?",
                version.end(), iface.version);
        }
        let mut rv = Vec::new();
        for global in &self.globals {
            if global.interface != iface.name {
                continue;
            }
            if global.version < *version.start() {
                return Err(BindError::UnsupportedVersion);
            }
            let version = global.version.min(*version.end());
            let udata = make_udata(global.name);
            let proxy = self.registry.bind(global.name, version, qh, udata)?;
            log::debug!(target: "sctk", "Bound new global [{}] {} v{}", global.name, iface.name, version);

            rv.push(proxy);
        }
        Ok(rv)
    }
}

/// Delegates the handling of [`wl_registry`].
///
/// Anything which implements [`RegistryHandler`] may be used in the delegate.
///
/// ## Usage
///
/// ```
/// use smithay_client_toolkit::{
///     delegate_registry, delegate_shm, registry_handlers,
///     shm::{ShmHandler, ShmState},
///     registry::{RegistryState, ProvidesRegistryState}
/// };
///
/// struct ExampleApp {
///     registry_state: RegistryState,
///     shm_state: ShmState,
/// }
///
/// // In order to use the registry, we need to delegate handling of WlRegistry to it.
/// delegate_registry!(ExampleApp);
///
/// // In order to use the registry delegate, we need to provide a way to access the registry state
/// // from your data type and provide a list of types that will bind to the registry.
/// impl ProvidesRegistryState for ExampleApp {
///     fn registry(&mut self) -> &mut RegistryState {
///         &mut self.registry_state
///     }
///     // Here we specify the types of the delegates which should handle registry events.
///     registry_handlers!(ShmState);
/// }
///
/// // Here is the implementation of wl_shm to compile:
/// delegate_shm!(ExampleApp);
///
/// impl ShmHandler for ExampleApp {
///     fn shm_state(&mut self) -> &mut ShmState {
///         &mut self.shm_state
///     }
/// }
/// ```
#[macro_export]
macro_rules! delegate_registry {
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty:
            [
                $crate::reexports::client::protocol::wl_registry::WlRegistry: (),
                $crate::reexports::client::protocol::wl_callback::WlCallback: $crate::registry::RegistryReady,
            ]  => $crate::registry::RegistryState
        );
    };
}

impl<D> DelegateDispatch<wl_registry::WlRegistry, (), D> for RegistryState
where
    D: Dispatch<wl_registry::WlRegistry, ()> + ProvidesRegistryState,
{
    fn event(
        state: &mut D,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                let iface = interface.clone();
                state.registry().globals.push(Global { name, interface, version });
                if state.registry().ready {
                    state.runtime_add_global(conn, qh, name, &iface, version);
                }
            }

            wl_registry::Event::GlobalRemove { name } => {
                if let Some(i) = state.registry().globals.iter().position(|g| g.name == name) {
                    let global = state.registry().globals.swap_remove(i);
                    if state.registry().ready {
                        state.runtime_remove_global(conn, qh, name, &global.interface);
                    }
                }
            }

            _ => unreachable!("wl_registry is frozen"),
        }
    }
}

impl<D> DelegateDispatch<wl_callback::WlCallback, RegistryReady, D> for RegistryState
where
    D: Dispatch<wl_callback::WlCallback, RegistryReady> + ProvidesRegistryState,
{
    fn event(
        state: &mut D,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        _: &RegistryReady,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        state.registry().ready = true;
        state.global_enumeration_finished(conn, qh);
    }
}

/// A helper that sets [RegistryState::ready] when enumeration is finished.
#[derive(Debug)]
pub struct RegistryReady;

/// A helper for storing a bound global.
///
/// This helper is intended to simplify the implementation of [RegistryHandler] for state objects
/// that cache a bound global.
#[derive(Debug)]
pub enum GlobalProxy<I> {
    /// Initial state: registry has not yet finished enumeration or there is a missing
    /// [RegistryHandler] delegtation.
    NotReady,
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
    pub fn new() -> Self {
        GlobalProxy::NotReady
    }

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
            GlobalProxy::NotReady => Err(GlobalError::NotReady),
        }
    }
}

/// A helper macro for implementing [`ProvidesRegistryState`].
///
/// See [`delegate_registry`] for an example.
#[macro_export]
macro_rules! registry_handlers {
    ($($ty:ty),* $(,)?) => {
        fn global_enumeration_finished(
            &mut self,
            conn: &$crate::reexports::client::Connection,
            qh: &$crate::reexports::client::QueueHandle<Self>,
        ) {
            $(
                <$ty as $crate::registry::RegistryHandler<Self>>::ready(self, conn, qh);
            )*
        }

        fn runtime_add_global(
            &mut self,
            conn: &$crate::reexports::client::Connection,
            qh: &$crate::reexports::client::QueueHandle<Self>,
            name: u32,
            interface: &str,
            version: u32,
        ) {
            $(
                <$ty as $crate::registry::RegistryHandler<Self>>::new_global(self, conn, qh, name, interface, version);
            )*
        }

        fn runtime_remove_global(
            &mut self,
            conn: &$crate::reexports::client::Connection,
            qh: &$crate::reexports::client::QueueHandle<Self>,
            name: u32,
            interface: &str,
        ) {
            $(
                <$ty as $crate::registry::RegistryHandler<Self>>::remove_global(self, conn, qh, name, interface);
            )*
        }
    }
}
