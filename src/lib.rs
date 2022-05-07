#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(
//    missing_docs, // Commented out for now so the project isn't all yellow.
    missing_debug_implementations
)]
#![forbid(unsafe_op_in_unsafe_fn, rust_2021_compatibility)]
#![allow(clippy::new_without_default)]

/// Re-exports of some crates, for convenience.
pub mod reexports {
    #[cfg(feature = "calloop")]
    pub use calloop;
    pub use wayland_client as client;
    pub use wayland_protocols as protocols;
    pub use wayland_protocols_wlr as protocols_wlr;
}

pub mod compositor;
pub mod error;
#[cfg(feature = "calloop")]
pub mod event_loop;
pub mod output;
pub mod registry;
pub mod seat;
pub mod shell;
pub mod shm;
