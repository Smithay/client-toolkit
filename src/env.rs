use std::io;
use std::sync::{Arc, Mutex};

use wayland_client::{EventQueue, GlobalManager, NewProxy, Proxy};
use wayland_client::protocol::{wl_compositor, wl_output, wl_registry, wl_seat, wl_shell, wl_shm,
                               wl_subcompositor};
use wayland_protocols::xdg_shell::client::xdg_wm_base;

/// Possible shell globals
pub enum Shell {
    /// Using xdg_shell protocol, the standart
    Xdg(Proxy<xdg_wm_base::XdgWmBase>),
    /// Using wl_shell, deprecated, compatibility mode
    Wl(Proxy<wl_shell::WlShell>),
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
        let manager = GlobalManager::new(registry);

        // sync to retrieve the global list
        evq.sync_roundtrip()?;

        // wl_compositor
        let compositor = manager
            .instanciate_auto::<wl_compositor::WlCompositor>()
            .expect("Server didn't advertize `wl_compositor`?!")
            .implement(|e, _| match e {});

        // wl_subcompositor
        let subcompositor = manager
            .instanciate_auto::<wl_subcompositor::WlSubcompositor>()
            .expect("Server didn't advertize `wl_subcompositor`?!")
            .implement(|e, _| match e {});

        // wl_shm
        let shm_formats = Arc::new(Mutex::new(Vec::new()));
        let shm_formats2 = shm_formats.clone();
        let shm = manager
            .instanciate_auto::<wl_shm::WlShm>()
            .expect("Server didn't advertize `wl_shm`?!")
            .implement(move |wl_shm::Event::Format { format }, _| {
                shm_formats2.lock().unwrap().push(format);
            });

        // shells
        let shell = if let Ok(wm_base) = manager.instanciate_auto::<xdg_wm_base::XdgWmBase>() {
            Shell::Xdg(
                wm_base.implement(|xdg_wm_base::Event::Ping { serial }, proxy: Proxy<_>| {
                    use self::xdg_wm_base::RequestsTrait;
                    proxy.pong(serial)
                }),
            )
        } else if let Ok(shell) = manager.instanciate_auto::<wl_shell::WlShell>() {
            Shell::Wl(shell.implement(|e, _| match e {}))
        } else {
            panic!("Server didn't advertize neither `xdg_wm_base` nor `wl_shell`?!");
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
        })
    }

    /// Retrive the accepted SHM formats of the server
    pub fn shm_formats(&self) -> Vec<wl_shm::Format> {
        self.shm_formats.lock().unwrap().clone()
    }
}
