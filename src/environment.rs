//! Environment management utilities
//!
//! This module provide the tools to automatically bind the wayland global objects you need in your program.
//!
//! At the heart of this is the `environment!` macro, which allows you to signal the globals you need
//! and a struct to manage them as they are signaled in the registry.
//!
//! ## Global handlers
//!
//! Wayland globals are split in two kinds, that we will call here "single" globals and "multi" globals.
//!
//! - "single" globals represent a capability of the server. They are generally signaled in the registry
//!   from the start and never removed. They are signaled a single time. Examples of these globals are
//!   `wl_compositor`, `wl_shm` or `xdg_wm_base`.
//! - "multi" globals represent a resource that the server gives you access to. These globals can be
//!   created or removed during the run of the program, and may exist as more than one instance, each
//!   representing a different physical resource. Examples of such globals are `wl_output` or `wl_seat`.
//!
//! The objects you need to handle these globals must implement one the two traits
//! [`GlobalHandler<I>`](trait.GlobalHandler.html) or [`MultiGlobalHandler<I>`](trait.MultiGlobalHandler.html),
//! depending on the kind of globals it will handle. These objects are responsible for binding the globals
//! from the registry, and assigning them to filters to receive their events as necessary.
//!
//! This module provides a generic implementation of the [`GlobalHandler<I>`](trait.GlobalHandler.html) trait
//! as [`SimpleGlobal<I>`](struct.SimpleGlobal.html). It can manage "single" globals that do not generate
//! events, and thus require no filter.
//!
//! ## the  `environment!` macro
//!
//! This macro is at the core of this module. See its documentation for details about how to
//! use it: [`environment!`](../macro.environment.html). You can alternatively use the
//! [`default_environment!`](../macro.default_environment.html) macro to quickly setup things and bring
//! in all SCTK modules.

use std::cell::RefCell;
use std::io::Result;
use std::rc::Rc;

use wayland_client::{
    protocol::{wl_display, wl_registry},
    Attached, DispatchData, EventQueue, GlobalEvent, GlobalManager, Interface, Proxy,
};

/*
 * Traits definitions
 */

/// Required trait for implementing a handler for "single" globals
pub trait GlobalHandler<I: Interface> {
    /// This global was created and signaled in the registry with given id and version
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        ddata: DispatchData,
    );
    /// Access the global if it was signaled
    fn get(&self) -> Option<Attached<I>>;
}

/// Required trait for implementing a handler for "multi" globals
pub trait MultiGlobalHandler<I: Interface> {
    /// A new instance of this global was created with given id and version
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        ddata: DispatchData,
    );
    /// The instance with given id was removed
    fn removed(&mut self, id: u32, ddata: DispatchData);
    /// Access all the currently existing instances
    fn get_all(&self) -> Vec<Attached<I>>;
}

/*
 * General Environment<E>
 */

/// A Wayland Environment
///
/// This struct is generated by the `environment!` macro, see module-level documentation
/// for more details about this.
///
/// This is the central point for accessing globals for your Wayland app. Any global that has
/// previously been declared in the `environment!` macro can be access from this type via the
/// `get_global`, `required_global` and `get_all_globals` methods.
///
/// This `Environment` is a handle that can be cloned.
pub struct Environment<E> {
    /// The underlying `GlobalManager`, if you need to do manual interaction with the
    /// registry. See `wayland-client` documentation for details.
    pub manager: GlobalManager,
    inner: Rc<RefCell<E>>,
}

impl<E: InnerEnv + 'static> Environment<E> {
    /// Create new `Environment`
    ///
    /// This requires access to a `wl_display` attached to the `event_queue`.
    /// You also need to provide an instance of the inner environment type declared
    /// using the [`environment!`](../macro.environment.html) macro.
    ///
    /// If you instead used the [`default_environment!`](../macro.default_environment.html), then
    /// you need to initialize your `Environment` using the
    /// [`new_default_environment!`](../macro.new_default_environment.html) macro.
    ///
    /// `std::io::Error` could be returned if initial roundtrips to the server failed.
    ///
    /// If this call indefinitely blocks when doing initial roundtrips this can only be
    /// caused by server bugs.
    pub fn new(
        display: &Attached<wl_display::WlDisplay>,
        queue: &mut EventQueue,
        env: E,
    ) -> Result<Environment<E>> {
        let environment = Self::new_pending(display, env);

        // Fully initialize the environment.
        queue.sync_roundtrip(&mut (), |event, _, _| {
            panic!(
                "Encountered unhandled event during initial roundtrip ({}::{})",
                event.interface, event.name
            );
        })?;
        queue.sync_roundtrip(&mut (), |event, _, _| {
            panic!(
                "Encountered unhandled event during initial roundtrip ({}::{})",
                event.interface, event.name
            );
        })?;

        Ok(environment)
    }

    /// Create new pending `Environment`
    ///
    /// This requires access to a `wl_display` attached to an event queue (on which the main SCTK logic
    /// will be attached). You also need to provide an instance of the inner environment type declared
    /// using the [`environment!`](../macro.environment.html) macro.
    ///
    /// If you instead used the [`default_environment!`](../macro.default_environment.html), then you need
    /// to initialize your `Environment` using the
    /// [`new_default_environment!`](../macro.new_default_environment.html) macro.
    ///
    /// You should prefer to use `Environment::new`, unless you want to control initialization
    /// manually or you create additional environment meaning that the initialization may be fine
    /// with just `dispatch_pending` of the event queue, instead of two roundtrips to
    /// fully initialize environment. If you manually initialize your environment two sync
    /// roundtrips are required.
    pub fn new_pending(display: &Attached<wl_display::WlDisplay>, env: E) -> Environment<E> {
        let inner = Rc::new(RefCell::new(env));

        let my_inner = inner.clone();
        let my_cb = move |event, registry, ddata: DispatchData| {
            let mut inner = my_inner.borrow_mut();
            inner.process_event(event, registry, ddata);
        };

        let manager = GlobalManager::new_with_cb(display, my_cb);

        Self { manager, inner }
    }
}

impl<E> Environment<E> {
    /// Access a "single" global
    ///
    /// This method allows you to access any "single" global that has previously
    /// been declared in the `environment!` macro. It is forwarded to the `get()`
    /// method of the appropriate `GlobalHandler`.
    ///
    /// It returns `None` if the global has not (yet) been signaled by the registry.
    pub fn get_global<I: Interface>(&self) -> Option<Attached<I>>
    where
        E: GlobalHandler<I>,
    {
        self.inner.borrow().get()
    }

    /// Access a "single" global or panic
    ///
    /// This method is similar to `get_global`, but will panic with a detailed error
    /// message if the requested global was not advertized by the server.
    pub fn require_global<I: Interface>(&self) -> Attached<I>
    where
        E: GlobalHandler<I>,
    {
        match self.inner.borrow().get() {
            Some(g) => g,
            None => panic!("[SCTK] A missing global was required: {}", I::NAME),
        }
    }

    /// Access all instances of a "multi" global
    ///
    /// This will return a `Vec` containing all currently existing instances of the
    /// requested "multi" global that has been previously declared in the `environment!`
    /// macro. It is forwarded to the `get_all()` method of the appropriate
    /// `MultiGlobalHandler`.
    pub fn get_all_globals<I: Interface>(&self) -> Vec<Attached<I>>
    where
        E: MultiGlobalHandler<I>,
    {
        self.inner.borrow().get_all()
    }

    /// Access the inner environment
    ///
    /// This gives your access, via a closure, to the inner type you declared
    /// via the [`environment!`](../macro.environment.html) or
    /// [`default_environment!`](../macro.default_environment.html) macro.
    ///
    /// This method returns the return value of your closure.
    pub fn with_inner<T, F: FnOnce(&mut E) -> T>(&self, f: F) -> T {
        let mut inner = self.inner.borrow_mut();
        f(&mut *inner)
    }
}

impl<E> Clone for Environment<E> {
    fn clone(&self) -> Environment<E> {
        Environment { manager: self.manager.clone(), inner: self.inner.clone() }
    }
}

/// Internal trait for the `Environment` logic
///
/// This trait is automatically implemented by the [`environment!`](../macro.environment.html)
/// macro, you should not implement it manually unless you seriously want to.
pub trait InnerEnv {
    /// Process a `GlobalEvent`
    fn process_event(
        &mut self,
        event: GlobalEvent,
        registry: Attached<wl_registry::WlRegistry>,
        data: DispatchData,
    );
}

/*
 * Simple handlers
 */

/// A minimalist global handler for "single" globals
///
/// This handler will simply register the global as soon as the registry signals
/// it, and do nothing more.
///
/// It is appropriate for globals that never generate events, like `wl_compositor`
/// or `wl_data_device_manager`.
pub struct SimpleGlobal<I: Interface> {
    global: Option<Attached<I>>,
}

impl<I: Interface> SimpleGlobal<I> {
    /// Create a new handler
    pub fn new() -> SimpleGlobal<I> {
        SimpleGlobal { global: None }
    }
}

impl<I: Interface + Clone + From<Proxy<I>> + AsRef<Proxy<I>>> GlobalHandler<I> for SimpleGlobal<I> {
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        _: DispatchData,
    ) {
        self.global = Some((*registry.bind::<I>(version, id)).clone())
    }
    fn get(&self) -> Option<Attached<I>> {
        self.global.clone()
    }
}

/*
 * environment! macro
 */

/// Macro for declaring an environment
///
/// It needs to be used in conjunction with a a `struct` you declared, which will serve as the inner
/// environment and hold the handlers for your globals.
///
/// The macro is invoked as such:
///
/// ```no_run
/// # extern crate smithay_client_toolkit as sctk;
/// # use sctk::reexports::client::protocol::{wl_compositor::WlCompositor, wl_subcompositor::WlSubcompositor, wl_output::WlOutput};
/// # use sctk::environment::SimpleGlobal;
/// # use sctk::environment;
/// # use sctk::output::OutputHandler;
/// struct MyEnv {
///     compositor: SimpleGlobal<WlCompositor>,
///     subcompositor: SimpleGlobal<WlSubcompositor>,
///     outputs: OutputHandler
/// }
///
/// environment!(MyEnv,
///     singles = [
///         WlCompositor => compositor,
///         WlSubcompositor => subcompositor,
///     ],
///     multis = [
///         WlOutput => outputs,
///     ]
/// );
/// ```
///
/// This will define how your `MyEnv` struct is able to manage the `WlCompositor`, `WlSubcompositor` and
/// `WlOutput` globals. For each global, you need to provide a pattern
/// `$type => $name` where:
///
/// - `$type` is the type (implementing the `Interface` trait from `wayland-client`) representing a global
/// - `$name` is the name of the field of `MyEnv` that is in charge of managing this global, implementing the
///   appropriate `GlobalHandler` or `MultiGlobalHandler` trait
///
/// It is possible to route several globals to the same field as long as it implements all the appropriate traits.
#[macro_export]
macro_rules! environment {
    ($env_name:ident,
        singles = [$($sty:ty => $sname:ident),* $(,)?],
        multis = [$($mty:ty => $mname:ident),* $(,)?]$(,)?
    ) => {
        impl $crate::environment::InnerEnv for $env_name {
            fn process_event(
                &mut self,
                event: $crate::reexports::client::GlobalEvent,
                registry: $crate::reexports::client::Attached<$crate::reexports::client::protocol::wl_registry::WlRegistry>,
                ddata: $crate::reexports::client::DispatchData,
            ) {
                match event {
                    $crate::reexports::client::GlobalEvent::New { id, interface, version } => match &interface[..] {
                        $(
                            <$sty as $crate::reexports::client::Interface>::NAME => $crate::environment::GlobalHandler::<$sty>::created(&mut self.$sname, registry, id, version, ddata),
                        )*
                        $(
                            <$mty as $crate::reexports::client::Interface>::NAME => $crate::environment::MultiGlobalHandler::<$mty>::created(&mut self.$mname, registry, id, version, ddata),
                        )*
                        _ => { /* ignore unkown globals */ }
                    },
                    $crate::reexports::client::GlobalEvent::Removed { id, interface } => match &interface[..] {
                        $(
                            <$mty as $crate::reexports::client::Interface>::NAME => $crate::environment::MultiGlobalHandler::<$mty>::removed(&mut self.$mname, id, ddata),
                        )*
                        _ => { /* ignore unknown globals */ }
                    }
                }
            }
        }

        $(
            impl $crate::environment::GlobalHandler<$sty> for $env_name {
                fn created(&mut self, registry: $crate::reexports::client::Attached<$crate::reexports::client::protocol::wl_registry::WlRegistry>, id: u32, version: u32, ddata: $crate::reexports::client::DispatchData) {
                    $crate::environment::GlobalHandler::<$sty>::created(&mut self.$sname, registry, id, version, ddata)
                }
                fn get(&self) -> Option<$crate::reexports::client::Attached<$sty>> {
                    $crate::environment::GlobalHandler::<$sty>::get(&self.$sname)
                }
            }
        )*

        $(
            impl $crate::environment::MultiGlobalHandler<$mty> for $env_name {
                fn created(&mut self, registry: $crate::reexports::client::Attached<$crate::reexports::client::protocol::wl_registry::WlRegistry>, id: u32, version: u32, ddata: $crate::reexports::client::DispatchData) {
                    $crate::environment::MultiGlobalHandler::<$mty>::created(&mut self.$mname, registry, id, version, ddata)
                }
                fn removed(&mut self, id: u32, ddata: $crate::reexports::client::DispatchData) {
                    $crate::environment::MultiGlobalHandler::<$mty>::removed(&mut self.$mname, id, ddata)
                }
                fn get_all(&self) -> Vec<$crate::reexports::client::Attached<$mty>> {
                    $crate::environment::MultiGlobalHandler::<$mty>::get_all(&self.$mname)
                }
            }
        )*
    };
}
