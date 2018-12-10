use std::io;
use std::sync::{Arc, Mutex};

use shell::{create_shell_surface, Event, ShellSurface};
use surface::{create_surface, SurfaceUserData};

use wayland_client::protocol::{
    wl_compositor, wl_data_device_manager, wl_display, wl_registry, wl_shell, wl_shm,
    wl_subcompositor, wl_surface,
};
use wayland_client::{EventQueue, GlobalEvent, GlobalManager, Proxy};
use wayland_protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1;
use wayland_protocols::unstable::xdg_shell::v6::client::zxdg_shell_v6;
use wayland_protocols::xdg_shell::client::xdg_wm_base;

/// Possible shell globals
pub enum Shell {
    /// Using xdg_shell protocol, the standard
    Xdg(Proxy<xdg_wm_base::XdgWmBase>),
    /// Old version of xdg_shell, for compatibility
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
    /// A manager for handling the advertised outputs
    pub outputs: ::output::OutputMgr,
    /// The decoration manager, if the server supports server-side decorations
    pub decorations_mgr: Option<Proxy<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>>,
    shm_formats: Arc<Mutex<Vec<wl_shm::Format>>>,
    surfaces: Arc<Mutex<Vec<Proxy<wl_surface::WlSurface>>>>,
}

impl Environment {
    /// Create an environment wrapping a new registry
    ///
    /// It requires you to provide the `EventQueue` as well because
    /// the initialization process does a few roundtrip to the server
    /// to initialize all the globals.
    pub fn from_display(
        display: &Proxy<wl_display::WlDisplay>,
        evq: &mut EventQueue,
    ) -> io::Result<Environment> {
        Environment::from_display_with_cb(display, evq, |_, _| {})
    }

    /// Create an environment wrapping a new registry
    ///
    /// Additionally to `from_display`, this allows you to provide
    /// a callback to be notified of global events, just like
    /// `GlobalManager::new_with_cb`. Note that you will still
    /// receive events even if they are processed by this `Environment`.
    pub fn from_display_with_cb<Impl>(
        display: &Proxy<wl_display::WlDisplay>,
        evq: &mut EventQueue,
        mut cb: Impl,
    ) -> io::Result<Environment>
    where
        Impl: FnMut(GlobalEvent, Proxy<wl_registry::WlRegistry>) + Send + 'static,
    {
        let outputs = ::output::OutputMgr::new();
        let outputs2 = outputs.clone();

        let surfaces: Arc<Mutex<Vec<Proxy<wl_surface::WlSurface>>>> =
            Arc::new(Mutex::new(Vec::new()));
        let surfaces2 = surfaces.clone();

        let display_wrapper = display.make_wrapper(&evq.get_token()).unwrap();
        let manager = GlobalManager::new_with_cb(&display_wrapper, move |event, registry| {
            match event {
                GlobalEvent::New {
                    id,
                    ref interface,
                    version,
                } => {
                    if let "wl_output" = &interface[..] {
                        outputs2.new_output(id, version, &registry)
                    }
                }
                GlobalEvent::Removed { id, ref interface } => {
                    if let "wl_output" = &interface[..] {
                        let output = outputs2
                            .find_id(id, |output, _info| output.clone())
                            .unwrap();
                        for surface in &*surfaces2.lock().unwrap() {
                            surface
                                .user_data::<Mutex<SurfaceUserData>>()
                                .expect("Surface was not created with create_surface.")
                                .lock()
                                .unwrap()
                                .leave(&output, surface.clone())
                        }
                        outputs2.output_removed(id)
                    }
                }
            }
            cb(event, registry);
        });

        // double sync to retrieve the global list
        // and the globals metadata
        evq.sync_roundtrip()?;
        evq.sync_roundtrip()?;

        // wl_compositor
        let compositor = manager
            .instantiate_auto(|compositor| compositor.implement(|_, _| {}, ()))
            .expect("Server didn't advertise `wl_compositor`?!");

        // wl_subcompositor
        let subcompositor = manager
            .instantiate_auto(|subcompositor| subcompositor.implement(|_, _| {}, ()))
            .expect("Server didn't advertise `wl_subcompositor`?!");

        // wl_shm
        let shm_formats = Arc::new(Mutex::new(Vec::new()));
        let shm_formats2 = shm_formats.clone();
        let shm = manager
            .instantiate_auto(|shm| {
                shm.implement(
                    move |wl_shm::Event::Format { format }, _| {
                        shm_formats2.lock().unwrap().push(format);
                    },
                    (),
                )
            })
            .expect("Server didn't advertise `wl_shm`?!");

        let data_device_manager = manager
            .instantiate_auto(|data_device_manager| data_device_manager.implement(|_, _| {}, ()))
            .expect("Server didn't advertise `wl_data_device_manager`?!");

        // shells
        let shell = if let Ok(wm_base) = manager.instantiate_auto(|wm_base| {
            wm_base.implement(
                |xdg_wm_base::Event::Ping { serial }, proxy: Proxy<_>| {
                    use self::xdg_wm_base::RequestsTrait;
                    proxy.pong(serial)
                },
                (),
            )
        }) {
            Shell::Xdg(wm_base)
        } else if let Ok(xdg_shell) = manager.instantiate_auto(|xdg_shell| {
            xdg_shell.implement(
                |zxdg_shell_v6::Event::Ping { serial }, proxy: Proxy<_>| {
                    use self::zxdg_shell_v6::RequestsTrait;
                    proxy.pong(serial)
                },
                (),
            )
        }) {
            Shell::Zxdg(xdg_shell)
        } else if let Ok(wl_shell) =
            manager.instantiate_auto(|wl_shell| wl_shell.implement(|_, _| {}, ()))
        {
            Shell::Wl(wl_shell)
        } else {
            panic!("Server didn't advertise neither `xdg_wm_base` nor `wl_shell`?!");
        };

        // try to retrieve the decoration manager
        let decorations_mgr = if let Shell::Xdg(_) = shell {
            manager
                .instantiate_auto(|mgr| mgr.implement(|_, _| {}, ()))
                .ok()
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
            surfaces,
        })
    }

    /// Retrieve the accepted SHM formats of the server
    pub fn shm_formats(&self) -> Vec<wl_shm::Format> {
        self.shm_formats.lock().unwrap().clone()
    }

    /// Create a new dpi aware surface
    ///
    /// The provided callback will be fired whenever the DPI factor associated to it
    /// changes.
    ///
    /// The DPI factor associated to a surface is defined as the maximum of the DPI
    /// factors of the outputs it is displayed on.
    pub fn create_surface<F>(&self, dpi_change: F) -> Proxy<wl_surface::WlSurface>
    where
        F: FnMut(i32, Proxy<wl_surface::WlSurface>) + Send + 'static,
    {
        let surface = create_surface(&self, Box::new(dpi_change));
        self.surfaces.lock().unwrap().push(surface.clone());
        surface
    }

    /// Create a new shell surface
    pub fn create_shell_surface<Impl>(
        &self,
        surface: &Proxy<wl_surface::WlSurface>,
        shell_impl: Impl,
    ) -> Box<ShellSurface>
    where
        Impl: FnMut(Event) + Send + 'static,
    {
        create_shell_surface(&self.shell, surface, shell_impl)
    }
}
