//! Smithay Client Toolkit
//!
//! Provides various utilities and abstractions for comunicating with various
//! Wayland compositors.
//!
//! ## `Environment`
//!
//! The crate is structured around the [`Environment`](environment/struct.Environment.html) type,
//! which binds the wayland globals for you using a set of modular handlers. This type is used in conjunction
//! with the [`environment!`](macro.environment.html) if you want full control, or by using the
//! [`default_environment!`](macro.default_environment.html) macro to automatically bring in all
//! SCTK modules.
//!
//! The various modules work by adding methods to the [`Environment`](environment/struct.Environment.html)
//! type, giving you more capabilities as more modules are activated.
//!
//! ## Event Loops
//!
//! SCTK integrates with `calloop` to provide an event loop abstraction. Indeed most Wayland
//! apps will need to handle more event sources than the single Wayland connection. These are
//! necessary to handle things like keyboard repetition, copy-paste, or animated cursors.
//!
//! [`WaylandSource`](struct.WaylandSource.html) is an adapter to insert a Wayland `EventQueue` into
//! a calloop event loop. And some of the modules of SCTK will provide you with other event sources
//! that you need to insert into calloop for them to work correctly.
#![warn(missing_docs)]

#[macro_use]
extern crate dlib;

/// Re-exports of some crates, for convenience
pub mod reexports {
    pub use calloop;
    pub use wayland_client as client;
    pub use wayland_protocols as protocols;
}

pub mod data_device;
pub mod environment;
pub mod output;
pub mod seat;
pub mod shell;
pub mod shm;
pub mod window;

mod event_loop;
mod surface;

pub use event_loop::WaylandSource;
pub use surface::{get_surface_outputs, get_surface_scale_factor};

#[macro_export]
/// Declare a batteries-included SCTK environment
///
/// Similar to the [`environment!`](macro.environment.html) macro, but creates the typr for you and
/// includes all the handlers provided by SCTK, for use with the rest of the library. Its sister macro
/// [`init_default_environment!`](macro.init_default_environment.html) need to be used to initialize it.
///
/// This includes handlers for the following globals:
///
/// - `wl_compositor` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
/// - `wl_data_device_manager` as a [`DataDeviceHandler`](data_device/struct.DataDeviceHandler.html)
/// - `wl_output` with the [`OutputHandler`](output/struct.OutputHandler.html)
/// - `wl_seat` with the [`SeatHandler`](seat/struct.SeatHandler.html)
/// - `wl_subcompositor` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
/// - `wl_shm` as a [`ShmHandler`](shm/struct.ShmHandler.html)
///
/// If you don't need to add anything more, its use is as simple as:
///
/// ```no_run
/// # use smithay_client_toolkit::default_environment;
/// default_environment!(MyEnv);
/// ```
///
/// The macro also provides some presets including more globals depending on your use-case:
///
/// - the `desktop` preset, invoked as `default_environment!(MyEnv, desktop);` aditionnaly includes:
///   - `xdg_shell` and `wl_shell` with the [`ShellHandler`](shell/struct.ShellHandler.html)
///   - `xdg_decoration_manager` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
///
/// You can also add the `fields` argument to add additional fields to the generated struct, and
/// the `singles` and `multis` arguments to route additional globals like with the
/// [`environment!`](macro.environment.html) macro. These three fields are optional, but they must
/// appear in this order, and after the optional preset
///
/// ```no_run
/// # use smithay_client_toolkit::default_environment;
/// default_environment!(MyEnv,
///     desktop, // the chosen preset, can be ommited
///     fields=[
///         somefield: u32,
///         otherfield: String,
///     ],
///     singles=[
///         // Add some routing here
///     ],
///     multis=[
///         // add some routing here
///     ]
/// );
/// ```
macro_rules! default_environment {
    ($env_name:ident, desktop
        $(,fields = [$($fname:ident : $fty:ty),* $(,)?])?
        $(,singles = [$($sty:ty => $sname: ident),* $(,)?])?
        $(,multis = [$($mty:ty => $mname:ident),* $(,)?])?
        $(,)?
    ) => {
        $crate::default_environment!($env_name,
            fields=[
                // shell
                sctk_shell: $crate::shell::ShellHandler,
                // decoration
                sctk_decoration_mgr: $crate::environment::SimpleGlobal<$crate::reexports::protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
                // others
                $($($fname : $fty,)*)?
            ],
            singles = [
                // shell globals
                $crate::reexports::client::protocol::wl_shell::WlShell => sctk_shell,
                $crate::reexports::protocols::xdg_shell::client::xdg_wm_base::XdgWmBase => sctk_shell,
                $crate::reexports::protocols::unstable::xdg_shell::v6::client::zxdg_shell_v6::ZxdgShellV6 => sctk_shell,
                // decoration
                $crate::reexports::protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1 => sctk_decoration_mgr,
                // others
                $($($sty => $sname,)*)?
            ],
            multis = [ $($($mty => $mname,)*)?  ],
        );

        // Shell utility
        impl $crate::shell::ShellHandling for $env_name {
            fn get_shell(&self) -> Option<$crate::shell::Shell> {
                self.sctk_shell.get_shell()
            }
        }
    };
    ($env_name:ident
        $(,fields = [$($fname:ident : $fty:ty),* $(,)?])?
        $(,singles = [$($sty:ty => $sname:ident),* $(,)?])?
        $(,multis = [$($mty:ty => $mname:ident),* $(,)?])?
        $(,)?
    ) => {
        /*
         * Declare the type
         */
        pub struct $env_name {
            // SimpleGlobals
            sctk_compositor: $crate::environment::SimpleGlobal<$crate::reexports::client::protocol::wl_compositor::WlCompositor>,
            sctk_subcompositor: $crate::environment::SimpleGlobal<$crate::reexports::client::protocol::wl_subcompositor::WlSubcompositor>,
            // shm
            sctk_shm: $crate::shm::ShmHandler,
            // output
            sctk_outputs: $crate::output::OutputHandler,
            // seat
            sctk_seats: $crate::seat::SeatHandler,
            // data device
            sctk_data_device_manager: $crate::data_device::DataDeviceHandler,
            // user added
            $($(
                $fname : $fty,
            )*)?
        }

        // SHM utility
        impl $crate::shm::ShmHandling for $env_name {
            fn shm_formats(&self) -> Vec<$crate::reexports::client::protocol::wl_shm::Format> {
                self.sctk_shm.shm_formats()
            }
        }

        // Seat utility
        impl $crate::seat::SeatHandling for $env_name {
            fn listen<F>(&mut self, f: F) -> $crate::seat::SeatListener
            where F: FnMut(
                $crate::reexports::client::Attached<$crate::reexports::client::protocol::wl_seat::WlSeat>,
                &$crate::seat::SeatData,
                $crate::reexports::client::DispatchData
            ) + 'static
            {
                self.sctk_seats.listen(f)
            }
        }

        // Data Device Utility
        impl $crate::data_device::DataDeviceHandling for $env_name {
            fn set_callback<F>(&mut self, callback: F) -> Result<(), ()>
            where F: FnMut(
                $crate::reexports::client::protocol::wl_seat::WlSeat,
                $crate::data_device::DndEvent,
                $crate::reexports::client::DispatchData
            ) + 'static
            {
                self.sctk_data_device_manager.set_callback(callback)
            }

            fn with_device<F: FnOnce(&$crate::data_device::DataDevice)>(
                &self,
                seat: &$crate::reexports::client::protocol::wl_seat::WlSeat,
                f: F
            ) -> Result<(), ()> {
                self.sctk_data_device_manager.with_device(seat, f)
            }
        }

        /*
         * Final macro delegation
         */
        $crate::environment!($env_name,
            singles = [
                // SimpleGlobals
                $crate::reexports::client::protocol::wl_compositor::WlCompositor => sctk_compositor,
                $crate::reexports::client::protocol::wl_subcompositor::WlSubcompositor => sctk_subcompositor,
                // shm
                $crate::reexports::client::protocol::wl_shm::WlShm => sctk_shm,
                // data device
                $crate::reexports::client::protocol::wl_data_device_manager::WlDataDeviceManager => sctk_data_device_manager,
                // user added
                $($($sty => $sname),*)?
            ],
            multis = [
                // output globals
                $crate::reexports::client::protocol::wl_output::WlOutput => sctk_outputs,
                // seat globals
                $crate::reexports::client::protocol::wl_seat::WlSeat => sctk_seats,
                // user added
                $($($mty => $mname),*)?
            ]
        );
    };
}

#[macro_export]
/// Initialize a batteries-included SCTK environment
///
/// Sister macro of [`default_environment!`](macro.default_environment.html). You need
/// to use it to initialize the environment instead of
/// [`Envrionment::init`](environment/struct.Environment.html). It has the same semantics.
///
/// If a preset was used for [`default_environment!`](macro.default_environment.html), it
/// must be provided here as well.
///
/// ```no_run
/// # use smithay_client_toolkit::{default_environment, init_default_environment};
/// # default_environment!(MyEnv, fields=[somefield: u32, otherfield: String,], singles=[], multis=[],);
/// # let display = smithay_client_toolkit::reexports::client::Display::connect_to_env().unwrap();
/// # let queue = display.create_event_queue();
/// let env = init_default_environment!(MyEnv,
///     desktop,           // the optional preset
///     &display,          // the WlDisplay for initialization
///     &mut queue,        // the event queue that should be used for initialization
///     /* initializers for your extra fields if any, can be ommited if no fields are added */
///     fields=[
///         somefield: 42,
///         otherfield: String::from("Hello World"),
///     ]
/// );
/// ```
macro_rules! init_default_environment {
    ($env_name:ident, desktop, $display:expr, $queue:expr
        $(,fields = [$($fname:ident : $fval:expr),* $(,)?])?
        $(,)?
    ) => {
        $crate::init_default_environment!($env_name, $display, $queue,
            fields = [
                sctk_shell: $crate::shell::ShellHandler::new(),
                sctk_decoration_mgr: $crate::environment::SimpleGlobal::new(),
                $($(
                    $fname: $fval,
                )*)?
            ]
        )
    };
    ($env_name:ident, $display:expr, $queue:expr
        $(,fields = [$($fname:ident : $fval:expr),* $(,)?])?
        $(,)?
    ) => {
        {
            let mut sctk_seats = $crate::seat::SeatHandler::new();
            let sctk_data_device_manager = $crate::data_device::DataDeviceHandler::init(&mut sctk_seats);

            let display = $crate::reexports::client::Proxy::clone(&$display);
            let env = $crate::environment::Environment::init(&display.attach($queue.token()), $env_name {
                sctk_compositor: $crate::environment::SimpleGlobal::new(),
                sctk_subcompositor: $crate::environment::SimpleGlobal::new(),
                sctk_shm: $crate::shm::ShmHandler::new(),
                sctk_outputs: $crate::output::OutputHandler::new(),
                sctk_seats,
                sctk_data_device_manager,
                $($(
                    $fname: $fval,
                )*)?
            });

            // two roundtrips to init the environment
            $queue
                .sync_roundtrip(&mut (), |_, _, _| unreachable!())
                .unwrap();
            $queue
                .sync_roundtrip(&mut (), |_, _, _| unreachable!())
                .unwrap();

            env
        }
    };
}
