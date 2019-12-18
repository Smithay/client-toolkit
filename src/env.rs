use std::io;
use std::sync::{Arc, Mutex};

use crate::shell::{create_shell_surface, Event, ShellSurface};
use crate::surface::{create_surface, SurfaceUserData};

use wayland_client::protocol::{
    wl_compositor, wl_data_device_manager, wl_display, wl_registry, wl_shell, wl_shm,
    wl_subcompositor, wl_surface,
};
use wayland_client::{Attached, EventQueue, GlobalEvent, GlobalManager};
use wayland_protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1;
use wayland_protocols::unstable::xdg_shell::v6::client::zxdg_shell_v6;
use wayland_protocols::xdg_shell::client::xdg_wm_base;

/// Possible shell globals
pub enum Shell {
    /// Using xdg_shell protocol, the standard
    Xdg(Attached<xdg_wm_base::XdgWmBase>),
    /// Old version of xdg_shell, for compatibility
    Zxdg(Attached<zxdg_shell_v6::ZxdgShellV6>),
    /// Using wl_shell, deprecated, compatibility mode
    Wl(Attached<wl_shell::WlShell>),
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
    pub compositor: Attached<wl_compositor::WlCompositor>,
    /// The subcompositor global, used to create subsurfaces
    pub subcompositor: Attached<wl_subcompositor::WlSubcompositor>,
    /// The shell global, used make your surfaces into windows
    ///
    /// This tries to bind using the xdg_shell protocol, and fallbacks
    /// to wl_shell if it fails
    pub shell: Shell,
    /// The SHM global, to create shared memory buffers
    pub shm: Attached<wl_shm::WlShm>,
    /// The data device manager, used to handle drag&drop and selection
    /// copy/paste
    pub data_device_manager: Attached<wl_data_device_manager::WlDataDeviceManager>,
    /// A manager for handling the advertised outputs
    pub outputs: crate::output::OutputMgr,
    /// The decoration manager, if the server supports server-side decorations
    pub decorations_mgr: Option<Attached<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>>,
    shm_formats: Arc<Mutex<Vec<wl_shm::Format>>>,
    surfaces: Arc<Mutex<Vec<wl_surface::WlSurface>>>,
}

impl Environment {
    /// Create an environment wrapping a new registry
    ///
    /// It requires you to provide the `EventQueue` as well because
    /// the initialization process does a few roundtrip to the server
    /// to initialize all the globals.
    pub fn from_display(
        display: &wl_display::WlDisplay,
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
        display: &wl_display::WlDisplay,
        evq: &mut EventQueue,
        mut cb: Impl,
    ) -> io::Result<Environment>
    where
        Impl: FnMut(GlobalEvent, Attached<wl_registry::WlRegistry>) + 'static,
    {
        let outputs = crate::output::OutputMgr::new();
        let outputs2 = outputs.clone();

        let surfaces: Arc<Mutex<Vec<wl_surface::WlSurface>>> = Arc::new(Mutex::new(Vec::new()));
        let surfaces2 = surfaces.clone();

        let attached_display = display.as_ref().clone().attach(evq.get_token());
        let manager = GlobalManager::new_with_cb(&attached_display, move |event, registry| {
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
                                .as_ref()
                                .user_data()
                                .get::<Mutex<SurfaceUserData>>()
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
        // no orphan event should exist at this point!
        evq.sync_roundtrip(|evt, obj| {
            panic!(
                "SCTK: orphan event: {}@{} -> {:?}",
                evt.interface,
                obj.as_ref().id(),
                evt.name
            )
        })?;
        evq.sync_roundtrip(|evt, obj| {
            panic!(
                "SCTK: orphan event: {}@{} -> {:?}",
                evt.interface,
                obj.as_ref().id(),
                evt.name
            )
        })?;

        // wl_compositor
        let compositor = manager
            .instantiate_range::<wl_compositor::WlCompositor>(1, 4)
            .expect("Server didn't advertise `wl_compositor`?!");

        // wl_subcompositor
        let subcompositor = manager
            .instantiate_range::<wl_subcompositor::WlSubcompositor>(1, 1)
            .expect("Server didn't advertise `wl_subcompositor`?!");

        // wl_shm
        let shm_formats = Arc::new(Mutex::new(Vec::new()));
        let shm_formats2 = shm_formats.clone();
        let shm = manager
            .instantiate_range::<wl_shm::WlShm>(1, 1)
            .expect("Server didn't advertise `wl_shm`?!");
        shm.assign_mono(move |_, evt| {
            if let wl_shm::Event::Format { format } = evt {
                shm_formats2.lock().unwrap().push(format);
            }
        });

        let data_device_manager = manager
            .instantiate_range::<wl_data_device_manager::WlDataDeviceManager>(1, 3)
            .expect("Server didn't advertise `wl_data_device_manager`?!");

        // shells
        let shell = if let Ok(wm_base) =
            manager
                .instantiate_exact::<xdg_wm_base::XdgWmBase>(1)
                .map(|wm_base| {
                    wm_base.assign_mono(|shell, evt| {
                        if let xdg_wm_base::Event::Ping { serial } = evt {
                            shell.pong(serial)
                        }
                    });
                    wm_base
                }) {
            Shell::Xdg((*wm_base).clone())
        } else if let Ok(xdg_shell) = manager
            .instantiate_exact::<zxdg_shell_v6::ZxdgShellV6>(1)
            .map(|xdg_shell| {
                xdg_shell.assign_mono(|shell, evt| {
                    if let zxdg_shell_v6::Event::Ping { serial } = evt {
                        shell.pong(serial);
                    }
                });
                xdg_shell
            })
        {
            Shell::Zxdg((*xdg_shell).clone())
        } else if let Ok(wl_shell) = manager.instantiate_exact::<wl_shell::WlShell>(1) {
            Shell::Wl((*wl_shell).clone())
        } else {
            panic!("Server didn't advertise neither `xdg_wm_base` nor `wl_shell`?!");
        };

        // try to retrieve the decoration manager
        let decorations_mgr = if let Shell::Xdg(_) = shell {
            manager
                .instantiate_exact::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>(1)
                .ok()
        } else {
            None
        };

        // sync to retrieve the global events
        evq.sync_roundtrip(|evt, obj| {
            panic!(
                "SCTK: orphan event: {}@{} -> {:?}",
                evt.interface,
                obj.as_ref().id(),
                evt.name
            )
        })?;

        Ok(Environment {
            manager,
            compositor: (*compositor).clone(),
            subcompositor: (*subcompositor).clone(),
            shell,
            shm: (*shm).clone(),
            shm_formats,
            data_device_manager: (*data_device_manager).clone(),
            decorations_mgr: decorations_mgr.map(|mgr| (*mgr).clone()),
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
    pub fn create_surface<F>(&self, dpi_change: F) -> wl_surface::WlSurface
    where
        F: FnMut(i32, wl_surface::WlSurface) + Send + 'static,
    {
        let surface = create_surface(&self, Box::new(dpi_change));
        self.surfaces.lock().unwrap().push(surface.clone());
        surface
    }

    /// Create a new shell surface
    pub fn create_shell_surface<Impl>(
        &self,
        surface: &wl_surface::WlSurface,
        shell_impl: Impl,
    ) -> Box<dyn ShellSurface>
    where
        Impl: FnMut(Event) + Send + 'static,
    {
        create_shell_surface(&self.shell, surface, shell_impl)
    }
}
