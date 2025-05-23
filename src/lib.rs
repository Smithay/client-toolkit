#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(
//    missing_docs, // Commented out for now so the project isn't all yellow.
    missing_debug_implementations
)]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(clippy::new_without_default)]
// keep existing usage
#![allow(clippy::uninlined_format_args)]

/// Re-exports of some crates, for convenience.
pub mod reexports {
    #[cfg(feature = "calloop")]
    pub use calloop;
    #[cfg(feature = "calloop")]
    pub use calloop_wayland_source;
    pub use wayland_client as client;
    pub use wayland_csd_frame as csd_frame;
    pub use wayland_protocols as protocols;
    pub use wayland_protocols_experimental as protocols_experimental;
    pub use wayland_protocols_misc as protocols_misc;
    pub use wayland_protocols_wlr as protocols_wlr;
}

pub mod activation;
pub mod compositor;
pub mod data_device_manager;
pub mod dmabuf;
pub mod error;
pub mod foreign_toplevel_list;
pub mod globals;
pub mod output;
pub mod presentation_time;
pub mod primary_selection;
pub mod registry;
pub mod seat;
pub mod session_lock;
pub mod shell;
pub mod shm;
pub mod subcompositor;
