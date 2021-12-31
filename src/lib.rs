#![warn(
//    missing_docs, // Commented out for now so the project isn't all yellow.
    missing_debug_implementations
)]
#![allow(clippy::new_without_default)]

/// Re-exports of some crates, for convenience.
pub mod reexports {
    #[cfg(feature = "calloop")]
    pub use calloop;
    pub use wayland_client as client;
    pub use wayland_protocols as protocols;
}

pub mod compositor;
pub mod output;
pub mod registry;
#[macro_use]
pub mod shm;
pub mod shell;
