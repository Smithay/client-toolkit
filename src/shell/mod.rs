//! # Shell abstractions
//!
//! A shell describes a set of wayland protocol extensions which define the capabilities of a surface and how
//! the surface is displayed.
//!
//! ## Cross desktop group (XDG) shell
//!
//! The XDG shell describes the semantics of desktop application windows.
//!
//! The XDG shell defines two types of surfaces:
//! - [`Window`] - An application window[^window].
//! - [`Popup`] - A child surface positioned relative to a window.
//!
//! ### Why use the XDG shell
//!
//! The XDG shell is the primary protocol through which application windows are created. You can be near
//! certain every desktop compositor will implement this shell so that applications may create windows.
//!
//! See the [XDG shell module documentation] for more information about creating application windows.
//!
//! [^window]: The XDG shell protocol actually refers to a window as a toplevel surface, but we use the more
//! familiar term "window" for the sake of clarity.
//!
//! [XDG shell module documentation]: self::xdg
//! [`Window`]: self::xdg::window::Window

pub mod xdg;
// TODO: Would be nice to support the layer shell.
