#![warn(missing_docs)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate dlib;
#[macro_use]
extern crate lazy_static;
extern crate memmap;
#[doc(hidden)]
pub extern crate wayland_client;
#[doc(hidden)]
pub extern crate wayland_commons;
#[doc(hidden)]
pub extern crate wayland_protocols;

/// Re-exports of some crates, for convenience
pub mod reexports {
    pub use wayland_client as client;
    pub use wayland_protocols as protocols;
}

pub mod pointer;
pub mod keyboard;
mod env;

pub use env::{Environment, Shell};
