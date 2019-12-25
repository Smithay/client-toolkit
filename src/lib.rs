//! Smithay Client Toolkit
//!
//! Provides various utilities and abstractions for comunicating with various
//! Wayland compositors.
//!
//! ## `Environment`
//!
//! The crate is structured around the [`Environment`](environment/struct.Environment.html) type,
//! which binds the wayland globals for you using a set of modular handlers. This type is initialized
//! using the [`environment!`](macro.environment.html) if you want full control, or by using the
//! [`default_environment!](macro.default_environment.html) to automatically bring in all SCTK modules.
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

mod surface;

pub use surface::{get_surface_outputs, get_surface_scale_factor};

/*
pub mod data_device;
pub mod keyboard;
pub mod pointer;
pub mod shell;
pub mod utils;
pub mod window;
*/

#[macro_export]
/// A batteries-included SCTK environment
///
/// Similar to the [`environment!`](macro.environment.html) macro, but includes all the handlers
/// provided by SCTK, for use with the rest of the library.
///
/// This includes handlers for the following globals:
///
/// - `wl_compositor` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
/// - `wl_data_device_manager` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
/// - `wl_output` with the [`OutputHandler`](output/struct.OutputHandler.html)
/// - `wl_subcompositor` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
macro_rules! default_environment {
    ($env_name:ident, $display:expr, $queue:expr, singles = [$(($sname:ident, $sty:ty, $shandler:expr)),* $(,)?], multis = [$(($mname:ident, $mty:ty, $mhandler:expr)),* $(,)?]) => {
        $crate::environment!($env_name, $display, $queue,
            singles = [
                (sctk_compositor, $crate::reexports::client::protocol::wl_compositor::WlCompositor, $crate::environment::SimpleGlobal::new()),
                (sctk_data_device_manager, $crate::reexports::client::protocol::wl_data_device_manager::WlDataDeviceManager, $crate::environment::SimpleGlobal::new()),
                (sctk_subcompositor, $crate::reexports::client::protocol::wl_subcompositor::WlSubcompositor, $crate::environment::SimpleGlobal::new()),
                $(($sname, $sty, $shandler)),*
            ],
            multis = [
                (sctk_outputs, $crate::reexports::client::protocol::wl_output::WlOutput, $crate::output::OutputHandler::new())
                $(($mname, $mty, $handler)),*
            ]
        )
    };
}
