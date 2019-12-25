//! Smithay Client Toolkit
//!
//! Provides various utilities and abstractions for comunicating with various
//! Wayland compositors.
//!
//! ## `Environment`
//!
//! The crate is structured around the [`Environment`](environment/struct.Environment.html) type,
//! which binds the wayland globals for you using a set of modular handlers. This type is initialized
//! using the [`init_environment!`](macro.init_environment.html) if you want full control, or by using the
//! [`init_default_environment!`](macro.init_default_environment.html) macro to automatically bring in all
//! SCTK modules.
//!
//! The various modules work by adding methods to the [`Environment`](environment/struct.Environment.html)
//! type, giving you more capabilities as more modules are activated.
#![warn(missing_docs)]

/*
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate dlib;
#[macro_use]
extern crate lazy_static;
*/

/// Re-exports of some crates, for convenience
pub mod reexports {
    pub use wayland_client as client;
    pub use wayland_protocols as protocols;
}

pub mod environment;
pub mod output;
pub mod shell;

mod surface;

pub use surface::{get_surface_outputs, get_surface_scale_factor};

/*
pub mod data_device;
pub mod keyboard;
pub mod pointer;
pub mod utils;
pub mod window;
*/

#[macro_export]
/// Declare a batteries-included SCTK environment
///
/// Similar to the [`declare_environment!`](macro.declare_environment.html) macro, but includes
/// all the handlers provided by SCTK, for use with the rest of the library. Its sister macro
/// is [`init_default_environment!`](macro.init_default_environment.html).
///
/// This includes handlers for the following globals:
///
/// - `wl_compositor` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
/// - `wl_data_device_manager` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
/// - `wl_output` with the [`OutputHandler`](output/struct.OutputHandler.html)
/// - `wl_subcompositor` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
///
/// If you don't need to add anything more, its use is as simple as:
///
/// ```no_run
/// # use smithay_client_toolkit::declare_default_environment;
/// declare_default_environment!(MyEnv, singles=[], multis=[], extras=[]);
/// ```
///
/// otherwise, its use is similar to [`declare_environment!`](macro.declare_environment.html).
macro_rules! declare_default_environment {
    ($env_name:ident,
        singles = [$(($sname:ident, $sty:ty)),* $(,)?],
        multis = [$(($mname:ident, $mty:ty)),* $(,)?],
        extras = [$(($ename:ident, $ety:ty)),* $(,)?]
    ) => {
        /*
         * Shell utilities
         */

        impl $crate::shell::ShellHandling for $env_name {
            fn get_shell(&self) -> Option<$crate::shell::Shell> {
                self.sctk_shell_handler.get_shell()
            }
        }
        /*
         * Final macro delegation
         */
        $crate::declare_environment!($env_name,
            singles = [
                // SimpleGlobals
                (sctk_compositor, $crate::reexports::client::protocol::wl_compositor::WlCompositor),
                (sctk_data_device_manager, $crate::reexports::client::protocol::wl_data_device_manager::WlDataDeviceManager),
                (sctk_subcompositor, $crate::reexports::client::protocol::wl_subcompositor::WlSubcompositor),
                // shell globals
                (sctk_wl_shell, $crate::reexports::client::protocol::wl_shell::WlShell),
                (sctk_xdg_shell, $crate::reexports::protocols::xdg_shell::client::xdg_wm_base::XdgWmBase),
                (sctk_zxdg_shell, $crate::reexports::protocols::unstable::xdg_shell::v6::client::zxdg_shell_v6::ZxdgShellV6),
                // user added
                $(($sname, $sty)),*
            ],
            multis = [
                // output globals
                (sctk_outputs, $crate::reexports::client::protocol::wl_output::WlOutput)
                // user added
                $(($mname, $mty)),*
            ],
            extras = [
                // shell extras
                (sctk_shell_handler, $crate::shell::ShellHandler),
                // user added
                $(($ename, $ety)),*
            ]
        );
    };
}

#[macro_export]
/// Initialize a batteries-included SCTK environment
///
/// Sister macro of [`declare_default_environment!`](macro.declare_default_environment.html). If you
/// don't need to add any handlers to the default one, its use is simply
///
/// ```no_run
/// # use smithay_client_toolkit::{declare_default_environment, init_default_environment};
/// # declare_default_environment!(MyEnv, singles=[], multis=[], extras=[]);
/// # let display = smithay_client_toolkit::reexports::client::Display::connect_to_env().unwrap();
/// # let mut queue = display.create_event_queue();
/// let env = init_default_environment!(MyEnv, &display, &mut queue, singles=[], multis=[], extras=[]);
/// ```
///
/// otherwise, its use is similar to [`init_environment!`](macro.init_environment.html)
macro_rules! init_default_environment {
    ($env_name:ident, $display:expr, $queue:expr,
        singles = [$(($sname:ident, $shandler:expr)),* $(,)?],
        multis = [$(($mname:ident, $mhandler:expr)),* $(,)?],
        extras = [$(($ename:ident, $eval:expr)),* $(,)?]
    ) => {
        {
            use $crate::reexports::client::protocol;
            use $crate::reexports::protocols;
            /*
             * Shell utilities
             */
            let shell_handler = $crate::shell::ShellHandler::new();

            /*
             * Final macro delegation
             */
            $crate::init_environment!($env_name, $display, $queue,
                singles = [
                    // SimpleGlobals
                    (sctk_compositor, $crate::environment::SimpleGlobal::new()),
                    (sctk_data_device_manager, $crate::environment::SimpleGlobal::new()),
                    (sctk_subcompositor, $crate::environment::SimpleGlobal::new()),
                    // shell globals
                    (sctk_wl_shell, shell_handler.clone()),
                    (sctk_xdg_shell, shell_handler.clone()),
                    (sctk_zxdg_shell, shell_handler.clone()),
                    // user added
                    $(($sname, $shandler)),*
                ],
                multis = [
                    // output globals
                    (sctk_outputs, $crate::output::OutputHandler::new())
                    // user added
                    $(($mname, $handler)),*
                ],
                extras = [
                    // shell extras
                    (sctk_shell_handler, shell_handler),
                    // user added
                    $(($ename, $eval)),*
                ]
            )
        }
    };
}
