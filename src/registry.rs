//! Utilities for binding globals with [`wl_registry`] in delegates.
//!
//! This module is based around the [`RegistryHandler`] trait and [`RegistryHandle`].
//!
//! [`RegistryHandle`] provides an interface to bind globals regularly, creating an object with each new
//! instantiation or caching bound globals to prevent duplicate object instances from being created. Binding
//! a global regularly is accomplished through [`RegistryHandle::bind_once`]. For caching a bound global use
//! [`RegistryHandle::bind_cached`].
//!
//! [`RegistryHandle`] is dispatched in an event queue using [`RegistryDispatch`]. The [`delegate_registry`]
//! macro is used to delegate [`wl_registry`] to [`RegistryDispatch`].
//!
//! ## Sample implementation of [`RegistryHandler`]
//!
//! ```
//! use smithay_client_toolkit::reexports::client::{
//!     ConnectionHandle,
//!     Dispatch,
//!     QueueHandle,
//!     delegate_dispatch,
//!     protocol::wl_compositor,
//! };
//!
//! use smithay_client_toolkit::registry::{RegistryHandle, RegistryHandler};
//!
//! struct ExampleApp {
//!     /// The registry handle is needed to use the global abstractions.
//!     registry_handle: RegistryHandle,
//!     /// This is a type we want to delegate global handling to.
//!     delegate_that_wants_registry: Delegate,
//! }
//!
//! /// The delegate a global should be provided to.
//! struct Delegate;
//!
//! // When implementing RegistryHandler, you must be able to dispatch any type you could bind using the registry handle.
//! impl<D> RegistryHandler<D> for Delegate
//! where
//!     D: Dispatch<wl_compositor::WlCompositor, UserData = ()> + 'static,
//! {
//!     // When a global is advertised, this function is called to let handlers see the new global.
//!     fn new_global(
//!         &mut self,
//!         cx: &mut ConnectionHandle,
//!         qh: &QueueHandle<D>,
//!         name: u32,
//!         interface: &str,
//!         version: u32,
//!         handle: &mut RegistryHandle,
//!     ) {
//!         if interface == "wl_compositor" {
//!             // You can bind a global like normal.
//!             let _compositor = handle.bind_once::<wl_compositor::WlCompositor, _, _>(
//!                 cx,
//!                 qh,
//!                 name,
//!                 1, // we want to bind version 1 of the global.
//!                 (), // and we provide the user data for the wl_compositor being created.
//!             ).unwrap();
//!
//!             // Or you can cache the bound global if it will be bound by multiple delegates.
//!             let _cached_compositor = handle.bind_cached::<wl_compositor::WlCompositor, _, _, _>(
//!                 cx,
//!                 qh,
//!                 name,
//!                 || {
//!                     // If the global is bound for the first time, this closure is invoked to provide the
//!                     // version of the global to bind and user data.
//!                     (1, ())
//!                 }
//!             ).unwrap();
//!         }
//!     }
//!
//!     // When a global is no longer advertised, this function is called to let handlers clean up.
//!     fn remove_global(&mut self, _cx: &mut ConnectionHandle, _name: u32) {
//!         // Do nothing since the compositor is a capability. Peripherals should implement this to avoid
//!         // keeping around dead objects.
//!     }
//! }
//! ```

use std::collections::{hash_map::Entry, HashMap};

use wayland_client::{
    backend::{InvalidId, ObjectId},
    protocol::wl_registry,
    ConnectionHandle, Dispatch, Proxy, QueueHandle,
};

/// A trait implemented by modular parts of a smithay's client toolkit and protocol delegates that may be used
/// to receive notification of a global being created or destroyed.
///
/// Delegates that choose to implement this trait may be used in [`delegate_registry`] which automatically
/// notifies delegates about the creation and destruction of globals, with the choice to bind the global.
pub trait RegistryHandler<D> {
    /// Called when a new global has been advertised by the compositor.
    ///
    /// The provided registry handle may be used to bind the global.
    fn new_global(
        &mut self,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
        handle: &mut RegistryHandle,
    );

    /// Called when a global has been destroyed by the compositor.
    fn remove_global(&mut self, cx: &mut ConnectionHandle, name: u32);
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

    /// The cached global being bound has not been created with correct interface.
    #[error("the cached global being bound has not been created with correct interface")]
    IncorrectInterface,
}

/// State object associated with the registry handling for smithay's client toolkit.
///
/// This object provides utilities to cache bound globals that are needed by multiple modules.
#[derive(Debug)]
pub struct RegistryHandle {
    registry: wl_registry::WlRegistry,
    cached_globals: HashMap<u32, CachedGlobal>,
}

impl RegistryHandle {
    /// Creates a new registry handle.
    ///
    /// This type may be used to bind globals as they are advertised.
    pub fn new(registry: wl_registry::WlRegistry) -> RegistryHandle {
        RegistryHandle { registry, cached_globals: HashMap::new() }
    }

    /// Binds a global, returning a new object associated with the global.
    ///
    /// This function may be used for any global, but should be avoided if the global being bound may be used
    /// by multiple modules of smithay's client toolkit. If multiple modules need a global, use
    /// [`RegistryHandle::bind_cached`] instead.
    ///
    /// A protocol error will be risen if the global has not yet been advertised.
    pub fn bind_once<I, D, U>(
        &mut self,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        version: u32,
        udata: U,
    ) -> Result<I, BindError>
    where
        D: Dispatch<I, UserData = U> + 'static,
        I: Proxy + 'static,
        U: Send + Sync + 'static,
    {
        if let Entry::Occupied(entry) = self.cached_globals.entry(name) {
            let cached = entry.get();

            log::warn!(
                target: "sctk",
                "RegistryHandle::bind_once used to bind cached global {} (name: {})",
                cached.interface,
                cached.name
            );
        }

        let global = self.registry.bind::<I, _>(cx, name, version, qh, udata)?;

        log::debug!(target: "sctk", "Bound new global [{}] {} v{}", name, I::interface().name, version);

        Ok(global)
    }

    /// Binds a global, caching the bound global for other modules of smithay's client toolkit to use.
    ///
    /// This function is primarily intended for globals which multiple modules may need to access, such as a
    /// `wl_output`.
    ///
    /// The closure passed into the function will be invoked to obtain the version of the global to bind and
    /// the user data associated with the global if the global has not been bound yet.
    ///
    /// A protocol error will be risen if the global has not yet been advertised.
    pub fn bind_cached<I, D, F, U>(
        &mut self,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        f: F,
    ) -> Result<I, BindError>
    where
        D: Dispatch<I, UserData = U> + 'static,
        I: Proxy + 'static,
        U: Send + Sync + 'static,
        F: FnOnce() -> (u32, U),
    {
        match self.cached_globals.get(&name) {
            Some(cached) => {
                // Ensure the requested interface is the same.
                if I::interface().name == cached.interface {
                    // Create a new handle for the existing global.
                    Ok(I::from_id(cx, cached.id.clone())?)
                } else {
                    Err(BindError::IncorrectInterface)
                }
            }

            // First bind of a global.
            None => {
                let (version, udata) = f();
                let global = self.registry.bind::<I, _>(cx, name, version, qh, udata)?;

                log::debug!(target: "sctk", "Bound new cached global [{}] {} v{}", name, I::interface().name, version);

                let removed = self.cached_globals.insert(
                    name,
                    CachedGlobal {
                        name,
                        _version: version,
                        interface: I::interface().name,
                        id: global.id(),
                    },
                );

                assert!(removed.is_none(), "Global was cached twice?");

                Ok(global)
            }
        }
    }
}

/// Delegates the handling of [`wl_registry`].
///
/// This macro takes a closure to obtain the [`RegistryHandle`] and a list of closures to obtain handlers for
/// registry events.
///
/// Anything which implements [`RegistryHandler`] may be used in the delegate.
///
/// ## Usage
///
/// ```
/// use smithay_client_toolkit::{delegate_registry, shm::ShmState, registry::RegistryHandle};
///
/// struct ExampleApp {
///     registry_handle: RegistryHandle,
///     shm_state: ShmState,
/// }
/// # // Implement wl_shm so the example compiles
/// # smithay_client_toolkit::delegate_shm!(ExampleApp: |app| { &mut app.shm_state });
///
/// // The delegate_registry macro implements `Dispatch<wl_registry::WlRegistry>` and sends events to the
/// // handlers.
/// delegate_registry!(ExampleApp:
///     // First we need to provide the instance of the registry handle to dispatch with.
///     |app| {
///         &mut app.registry_handle
///     },
///     // Delegates to need to be provided using a closure syntax.
///     // The named parameter of the closure above may be used in the lower closures.
///     handlers = [
///         { &mut app.shm_state }
///         // And more delegates that want to bind globals, all of these are comma separated.
///     ]
/// );
/// ```
#[macro_export]
macro_rules! delegate_registry {
    (
        $ty: ty:
        |$dispatcher: ident| $closure: block,
        handlers = [
            $($get_handler: block),*
        ]
    ) => {
        impl
            $crate::reexports::client::Dispatch<
                $crate::reexports::client::protocol::wl_registry::WlRegistry,
            > for $ty
        {
            type UserData = ();

            fn event(
                &mut self,
                registry: &$crate::reexports::client::protocol::wl_registry::WlRegistry,
                event: $crate::reexports::client::protocol::wl_registry::Event,
                _: &(),
                cx: &mut $crate::reexports::client::ConnectionHandle<'_>,
                qh: &$crate::reexports::client::QueueHandle<Self>,
            ) {
                use $crate::registry::RegistryHandler;

                type Event = $crate::reexports::client::protocol::wl_registry::Event;

                let $dispatcher = self;
                let handle = { $closure };

                match event {
                    Event::Global { name, interface, version } => {
                        $(
                            let handler: &mut dyn RegistryHandler<Self> = { $get_handler };
                            handler.new_global(cx, qh, name, &interface[..], version, handle);
                        )*
                    }

                    Event::GlobalRemove { name } => {
                        $(
                            let handler: &mut dyn RegistryHandler<Self> = { $get_handler };
                            handler.remove_global(cx, name);
                        )*

                        handle.remove_cached_global(&name);
                    }

                    _ => unreachable!("wl_registry is frozen"),
                }
            }
        }
    };
}

#[derive(Debug)]
struct CachedGlobal {
    name: u32,
    _version: u32,
    interface: &'static str,
    id: ObjectId,
}

impl RegistryHandle {
    /// Smithay client toolkit implementation detail.
    ///
    /// Library users should not invoke this function
    ///
    /// There are no stability guarantees for this function.
    #[doc(hidden)]
    pub fn remove_cached_global(&mut self, name: &u32) {
        self.cached_globals.remove(name);
    }
}
