//! Utilities to work with pointers and their icons

use std::ops::Deref;

use wayland_client::commons::Implementation;
use wayland_client::protocol::{wl_compositor, wl_pointer, wl_shm};
use wayland_client::{NewProxy, Proxy, QueueToken};

mod theme;

pub use self::theme::{ThemeManager, ThemedPointer};

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
    pub fn init(
        name: Option<&str>,
        compositor: Proxy<wl_compositor::WlCompositor>,
        shm: Proxy<wl_shm::WlShm>,
    ) -> AutoThemer {
        match ThemeManager::init(name, compositor, shm) {
            Ok(mgr) => AutoThemer::Themed(mgr),
            Err(()) => AutoThemer::UnThemed,
        }
    }

    /// Wrap a pointer to theme it
    pub fn theme_pointer(&self, pointer: Proxy<wl_pointer::WlPointer>) -> AutoPointer {
        match *self {
            AutoThemer::Themed(ref mgr) => AutoPointer::Themed(mgr.theme_pointer(pointer)),
            AutoThemer::UnThemed => AutoPointer::UnThemed(pointer),
        }
    }

    /// Initialize a new pointer as a ThemedPointer with an adapter implementation
    ///
    /// You need to provide an implementation as if implementing a `wl_pointer`, but
    /// it will receive as `meta` argument an `AutoPointer` wrapping your pointer,
    /// rather than a `Proxy<WlPointer>`.
    pub fn theme_pointer_with_impl<Impl>(
        &self,
        pointer: NewProxy<wl_pointer::WlPointer>,
        mut implementation: Impl,
    ) -> AutoPointer
    where
        Impl: Implementation<AutoPointer, wl_pointer::Event> + Send + 'static,
    {
        match *self {
            AutoThemer::Themed(ref mgr) => {
                let pointer = mgr.theme_pointer_with_impl(pointer, move |event, pointer| {
                    implementation.receive(event, AutoPointer::Themed(pointer))
                });
                AutoPointer::Themed(pointer)
            }
            AutoThemer::UnThemed => {
                let pointer = pointer.implement(move |event, pointer| {
                    implementation.receive(event, AutoPointer::UnThemed(pointer))
                });
                AutoPointer::UnThemed(pointer)
            }
        }
    }

    /// Initialize a new pointer as a ThemedPointer with an adapter implementation
    ///
    /// Like `theme_pointer_with_impl` but allows you to have a non-`Send` implementation.
    ///
    /// **Unsafe** for the same reasons as `NewProxy::implement_nonsend`.
    pub unsafe fn theme_pointer_with_nonsend_impl<Impl>(
        &self,
        pointer: NewProxy<wl_pointer::WlPointer>,
        mut implementation: Impl,
        token: &QueueToken,
    ) -> AutoPointer
    where
        Impl: Implementation<AutoPointer, wl_pointer::Event> + Send + 'static,
    {
        match *self {
            AutoThemer::Themed(ref mgr) => {
                let pointer = mgr.theme_pointer_with_nonsend_impl(
                    pointer,
                    move |event, pointer| {
                        implementation.receive(event, AutoPointer::Themed(pointer))
                    },
                    token,
                );
                AutoPointer::Themed(pointer)
            }
            AutoThemer::UnThemed => {
                let pointer = pointer.implement_nonsend(
                    move |event, pointer| {
                        implementation.receive(event, AutoPointer::UnThemed(pointer))
                    },
                    token,
                );
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
    /// If this is done as an anwser to an input event, you need to provide
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
