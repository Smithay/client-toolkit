//! Utilities to work with pointers and their icons

use std::ops::Deref;

use wayland_client::protocol::{wl_pointer, wl_seat};
use wayland_client::Proxy;

use wayland_client::protocol::wl_seat::RequestsTrait as SeatRequests;

mod theme;

pub use self::theme::{ThemeManager, ThemedPointer};
use env::Environment;

/// Wrapper to gracefully handle a missing `libwayland-cursor`
///
/// This wrapper has the same API as `ThemeManager`, but will
/// gracefully handle the case of a missing `libwayland-cursor`
/// by doing nothing.
///
/// It is a convenience wrapper to handle systems where
/// `libwayland-client.so` is available but not `libwayland-cursor.so`.
pub enum AutoThemer {
    /// The theme could be loaded
    Themed(ThemeManager),
    /// `libwayland-cursor.so` is not available
    UnThemed,
}

impl AutoThemer {
    /// Load a system pointer theme
    ///
    /// Will use the default theme of the system if name is `None`.
    ///
    /// Falls back to `UnThemed` if `libwayland-cursor` is not available.
    pub fn init(environment: &Environment, name: Option<String>) -> Self {
        match ThemeManager::init(environment, name) {
            Ok(mgr) => AutoThemer::Themed(mgr),
            Err(()) => AutoThemer::UnThemed,
        }
    }

    /// Initialize a new pointer as a ThemedPointer with an adapter implementation
    ///
    /// You need to provide an implementation as if implementing a `wl_pointer`, but
    /// it will receive as `meta` argument an `AutoPointer` wrapping your pointer,
    /// rather than a `Proxy<WlPointer>`.
    pub fn theme_pointer_with_impl<Impl, UD>(
        &self,
        seat: &Proxy<wl_seat::WlSeat>,
        mut implementation: Impl,
        user_data: UD,
    ) -> AutoPointer
    where
        Impl: FnMut(wl_pointer::Event, AutoPointer) + Send + 'static,
        UD: Send + Sync + 'static,
    {
        match *self {
            AutoThemer::Themed(ref mgr) => {
                let pointer = mgr.theme_pointer_with_impl(
                    seat,
                    move |event, seat| implementation(event, AutoPointer::Themed(seat)),
                    user_data,
                );
                AutoPointer::Themed(pointer)
            }
            AutoThemer::UnThemed => {
                let pointer = seat
                    .get_pointer(|pointer| {
                        pointer.implement(
                            move |event, seat| implementation(event, AutoPointer::UnThemed(seat)),
                            user_data,
                        )
                    })
                    .unwrap();
                AutoPointer::UnThemed(pointer)
            }
        }
    }
}

/// A pointer wrapper to gracefully handle a missing `libwayland-cursor`
///
/// It has the same API as `ThemedPointer`, but falls back to doing nothing
/// in its `Unthemed` variant.
pub enum AutoPointer {
    /// The `ThemedPointer`
    Themed(ThemedPointer),
    /// The regular pointer if theme capability is not available
    UnThemed(Proxy<wl_pointer::WlPointer>),
}

impl AutoPointer {
    /// Change the cursor to the given cursor name
    ///
    /// Possible names depend on the theme. Does nothing and returns
    /// `Err(())` if given name is not available.
    ///
    /// Does nothing an returns `Ok(())` if no theme is loaded (if
    /// `wayland-cursor` is not available).
    ///
    /// If this is done as an answer to an input event, you need to provide
    /// the associated serial otherwise the server may ignore the request.
    pub fn set_cursor(&self, name: &str, serial: Option<u32>) -> Result<(), ()> {
        match *self {
            AutoPointer::Themed(ref themed) => themed.set_cursor(name, serial),
            AutoPointer::UnThemed(_) => Ok(()),
        }
    }
}

impl Deref for AutoPointer {
    type Target = Proxy<wl_pointer::WlPointer>;
    fn deref(&self) -> &Proxy<wl_pointer::WlPointer> {
        match *self {
            AutoPointer::Themed(ref themed) => &**themed,
            AutoPointer::UnThemed(ref ptr) => ptr,
        }
    }
}

impl Clone for AutoPointer {
    fn clone(&self) -> AutoPointer {
        match *self {
            AutoPointer::Themed(ref themed) => AutoPointer::Themed(themed.clone()),
            AutoPointer::UnThemed(ref ptr) => AutoPointer::UnThemed(ptr.clone()),
        }
    }
}
