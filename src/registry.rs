use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Formatter},
};

use wayland_client::{
    backend::{InvalidId, ObjectId},
    protocol::wl_registry,
    ConnectionHandle, DataInit, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy,
    QueueHandle,
};

/// A trait implemented by modular parts of a smithay's client toolkit and protocol delegates that may be used
/// to receive notification of a global being created or destroyed.
///
///
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

    /// The cached global being bound is not the correct interface.
    #[error("the cached global being bound is not the correct interface")]
    IncorrectInterface,
}

/// State object associated with the registry handling for smithay's client toolkit.
#[derive(Debug)]
pub struct RegistryHandle {
    registry: wl_registry::WlRegistry,
    cached_globals: HashMap<u32, CachedGlobal>,
}

impl RegistryHandle {
    /// Creates a new registry handle.
    pub fn new(registry: wl_registry::WlRegistry) -> RegistryHandle {
        RegistryHandle { registry, cached_globals: HashMap::new() }
    }

    /// Binds a global, returning a new object associated with the global.
    ///
    /// This function may be used for any global, but should be avoided if the global being bound may be used
    /// by multiple modules of smithay's client toolkit. If multiple modules need a global, use
    /// [`RegistryHandle::bind_cached`] instead.
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

pub struct RegistryDispatch<'s, D>(
    pub &'s mut RegistryHandle,
    pub Vec<&'s mut dyn RegistryHandler<D>>,
);

impl<D> fmt::Debug for RegistryDispatch<'_, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegistryDispatch").field("handle", &self.0).finish_non_exhaustive()
    }
}

impl<D> DelegateDispatchBase<wl_registry::WlRegistry> for RegistryDispatch<'_, D> {
    type UserData = ();
}

impl<D> DelegateDispatch<wl_registry::WlRegistry, D> for RegistryDispatch<'_, D>
where
    D: Dispatch<wl_registry::WlRegistry, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        _: &mut DataInit<'_>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                for handler in self.1.iter_mut() {
                    handler.new_global(cx, qh, name, &interface[..], version, self.0);
                }
            }

            wl_registry::Event::GlobalRemove { name } => {
                for handler in self.1.iter_mut() {
                    handler.remove_global(cx, name);
                }
            }

            _ => unreachable!("wl_registry is frozen"),
        }
    }
}

#[derive(Debug)]
struct CachedGlobal {
    name: u32,
    _version: u32,
    interface: &'static str,
    id: ObjectId,
}
