use std::io;
use std::sync::{Arc, Mutex};

use wayland_client::commons::Implementation;
use wayland_client::protocol::{
    wl_compositor, wl_data_device_manager, wl_output, wl_registry, wl_shell, wl_shm,
    wl_subcompositor,
};
use wayland_client::{EventQueue, GlobalEvent, GlobalManager, NewProxy, Proxy};
use wayland_protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1;
use wayland_protocols::unstable::xdg_shell::v6::client::zxdg_shell_v6;
use wayland_protocols::xdg_shell::client::xdg_wm_base;

use wayland_client::protocol::wl_registry::RequestsTrait;

/// Possible shell globals
pub enum Shell {
    /// Using xdg_shell protocol, the standart
    Xdg(Proxy<xdg_wm_base::XdgWmBase>),
    /// Old version of xdg_shell, for compatiblity
    Zxdg(Proxy<zxdg_shell_v6::ZxdgShellV6>),
    /// Using wl_shell, deprecated, compatibility mode
    Wl(Proxy<wl_shell::WlShell>),
}

impl Shell {
    /// Check whether you need to wait for a configure before
    /// drawing to your surfaces
    ///
    /// This depend on the underlying shell protocol
    pub fn needs_configure(&self) -> bool {
        match *self {
            Shell::Xdg(_) => true,
            Shell::Zxdg(_) => true,
            Shell::Wl(_) => false,
        }
    }
}

/// A convenience for global management
///
/// This type provides convenience utilities for writing wayland
/// client apps, by auto-binding a large portion of the global
/// objects you'll likely need to write your app. This is mostly
/// provided as a mean to factor a consequent amount of dumb,
/// redundant code.
pub struct Environment {
    /// The underlying GlobalManager wrapping your registry
    pub manager: GlobalManager,
    /// The compositor global, used to create surfaces
    pub compositor: Proxy<wl_compositor::WlCompositor>,
    /// The subcompositor global, used to create subsurfaces
    pub subcompositor: Proxy<wl_subcompositor::WlSubcompositor>,
    /// The shell global, used make your surfaces into windows
    ///
    /// This tries to bind using the xdg_shell protocol, and fallbacks
    /// to wl_shell if it fails
    pub shell: Shell,
    /// The SHM global, to create shared memory buffers
    pub shm: Proxy<wl_shm::WlShm>,
    /// The data device manager, used to handle drag&drop and selection
    /// copy/paste
    pub data_device_manager: Proxy<wl_data_device_manager::WlDataDeviceManager>,
    /// A manager for handling the advertized outputs
    pub outputs: ::output::OutputMgr,
    /// The decoration manager, if the server supports server-side decorations
    pub decorations_mgr: Option<Proxy<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>>,
    shm_formats: Arc<Mutex<Vec<wl_shm::Format>>>,
}

impl Environment {
    /// Create an environment wrapping a new registry
    ///
    /// It requires you to provide the `EventQueue` as well because
    /// the initialization process does a few roundtrip to the server
    /// to initialize all the globals.
    ///
    /// This may panic or fail if you do not provide the EventQueue hosting
    /// the registry you provided.
    pub fn from_registry(
        registry: NewProxy<wl_registry::WlRegistry>,
        evq: &mut EventQueue,
    ) -> io::Result<Environment> {
        Environment::from_registry_with_cb(registry, evq, |_, _| {})
    }

    /// Create an environment wrapping a new registry
    ///
    /// Additionnaly to `from_registry`, this allows you to provide
    /// a callback to be notified of global events, just like
    /// `GlobalManager::new_with_cb`. Note that you will still
    /// receive events even if they are processed by this `Environment`.
    pub fn from_registry_with_cb<Impl>(
        registry: NewProxy<wl_registry::WlRegistry>,
        evq: &mut EventQueue,
        mut cb: Impl,
    ) -> io::Result<Environment>
    where
        Impl: Implementation<Proxy<wl_registry::WlRegistry>, GlobalEvent> + Send + 'static,
    {
        let outputs = ::output::OutputMgr::new();
        let outputs2 = outputs.clone();

        let manager = GlobalManager::new_with_cb(registry, move |event, registry: Proxy<_>| {
            match event {
                GlobalEvent::New {
                    id,
                    ref interface,
                    version,
                } => if let "wl_output" = &interface[..] {
                    outputs2.new_output(
                        id,
                        registry.bind::<wl_output::WlOutput>(version, id).unwrap(),
                    )
                },
                GlobalEvent::Removed { id, ref interface } => if let "wl_output" = &interface[..] {
                    outputs2.output_removed(id)
                },
            }
            cb.receive(event, registry);
        });

        // double sync to retrieve the global list
        // and the globals metadata
        evq.sync_roundtrip()?;
        evq.sync_roundtrip()?;

        // wl_compositor
        let compositor = manager
            .instantiate_auto::<wl_compositor::WlCompositor>()
            .expect("Server didn't advertize `wl_compositor`?!")
            .implement(|e, _| match e {});

        // wl_subcompositor
        let subcompositor = manager
            .instantiate_auto::<wl_subcompositor::WlSubcompositor>()
            .expect("Server didn't advertize `wl_subcompositor`?!")
            .implement(|e, _| match e {});

        // wl_shm
        let shm_formats = Arc::new(Mutex::new(Vec::new()));
        let shm_formats2 = shm_formats.clone();
        let shm = manager
            .instantiate_auto::<wl_shm::WlShm>()
            .expect("Server didn't advertize `wl_shm`?!")
            .implement(move |wl_shm::Event::Format { format }, _| {
                shm_formats2.lock().unwrap().push(format);
            });

        let data_device_manager = manager
            .instantiate_auto::<wl_data_device_manager::WlDataDeviceManager>()
            .expect("Server didn't advertize `wl_data_device_manager`?!")
            .implement(|e, _| match e {});

        // shells
        let shell = if let Ok(wm_base) = manager.instantiate_auto::<xdg_wm_base::XdgWmBase>() {
            Shell::Xdg(
                wm_base.implement(|xdg_wm_base::Event::Ping { serial }, proxy: Proxy<_>| {
                    use self::xdg_wm_base::RequestsTrait;
                    proxy.pong(serial)
                }),
            )
        } else if let Ok(xdg_shell) = manager.instantiate_auto::<zxdg_shell_v6::ZxdgShellV6>() {
            Shell::Zxdg(xdg_shell.implement(
                |zxdg_shell_v6::Event::Ping { serial }, proxy: Proxy<_>| {
                    use self::zxdg_shell_v6::RequestsTrait;
                    proxy.pong(serial)
                },
            ))
        } else if let Ok(shell) = manager.instantiate_auto::<wl_shell::WlShell>() {
            Shell::Wl(shell.implement(|e, _| match e {}))
        } else {
            panic!("Server didn't advertize neither `xdg_wm_base` nor `wl_shell`?!");
        };

        // try to retrieve the decoration manager
        let decorations_mgr = if let Shell::Xdg(_) = shell {
            manager
                .instantiate_auto::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>()
                .ok()
                .map(|mgr| mgr.implement(|evt, _| match evt {}))
        } else {
            None
        };

        // sync to retrieve the global events
        evq.sync_roundtrip()?;

        Ok(Environment {
            manager,
            compositor,
            subcompositor,
            shell,
            shm,
            shm_formats,
            data_device_manager,
            decorations_mgr,
            outputs,
        })
    }

    /// Retrive the accepted SHM formats of the server
    pub fn shm_formats(&self) -> Vec<wl_shm::Format> {
        self.shm_formats.lock().unwrap().clone()
    }
}
