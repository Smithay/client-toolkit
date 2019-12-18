//! Smithay Client Toolkit
//!
//! Provides various utilities and abstractions for comunicating with various
//! Wayland compositors.
#![warn(missing_docs)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate dlib;
#[macro_use]
extern crate lazy_static;

/// Re-exports of some crates, for convenience
pub mod reexports {
    pub use wayland_client as client;
    pub use wayland_protocols as protocols;
}

pub mod data_device;
pub mod keyboard;
pub mod output;
pub mod pointer;
pub mod shell;
pub mod surface;
pub mod utils;
pub mod window;

mod env;

pub use crate::env::{Environment, Shell};
