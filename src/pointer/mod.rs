//! Utilities to work with pointers and their icons

use std::sync::{Arc, Mutex};
use std::ops::Deref;
use wayland_client::{NewProxy, Proxy, QueueToken};
use wayland_client::commons::Implementation;
use wayland_client::cursor::{is_available, load_theme, CursorTheme};
use wayland_client::protocol::{wl_compositor, wl_pointer, wl_shm, wl_surface};

use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use wayland_client::protocol::wl_pointer::RequestsTrait as PointerRequests;
use wayland_client::protocol::wl_surface::RequestsTrait as SurfaceRequests;

struct Inner {
    surface: Proxy<wl_surface::WlSurface>,
    theme: CursorTheme,
    last_serial: u32,
}

/// Wrapper managing a system theme for pointer images
///
/// You can access the underlying `Proxy<wl_pointer::WlPointer>` via
/// deref. It will *not* release the proxy when dropped.
///
/// Just like `Proxy`, this is a `Rc`-like wrapper. You can clone it
/// to have several handles to the same theming machinnery of a pointer.
pub struct ThemedPointer {
    pointer: Proxy<wl_pointer::WlPointer>,
    inner: Arc<Mutex<Inner>>,
}

impl ThemedPointer {
    /// Load a system pointer theme
    ///
    /// Will wrap given pointer and load the system theme of
    /// provided name to image it. Will use the default theme
    /// of the system if name is `None`.
    pub fn load(
        pointer: Proxy<wl_pointer::WlPointer>,
        name: Option<&str>,
        compositor: &Proxy<wl_compositor::WlCompositor>,
        shm: &Proxy<wl_shm::WlShm>,
    ) -> Result<ThemedPointer, Proxy<wl_pointer::WlPointer>> {
        if !is_available() {
            return Err(pointer);
        }

        let theme = load_theme(name, 16, shm);
        let surface = compositor.create_surface().unwrap().implement(|_, _| {});

        Ok(ThemedPointer {
            pointer: pointer,
            inner: Arc::new(Mutex::new(Inner {
                surface: surface,
                theme: theme,
                last_serial: 0,
            })),
        })
    }

    /// Change the cursor to the given cursor name
    ///
    /// Possible names depend on the theme. Does nothing and returns
    /// `Err(())` if given name is not available.
    ///
    /// If this is done as an anwser to an input event, you need to provide
    /// the associated serial otherwise the server may ignore the request.
    pub fn set_cursor(&self, name: &str, serial: Option<u32>) -> Result<(), ()> {
        let mut inner = self.inner.lock().unwrap();
        let Inner {
            ref theme,
            ref surface,
            ref mut last_serial,
        } = *inner;

        let cursor = theme.get_cursor(name).ok_or(())?;
        let buffer = cursor.frame_buffer(0).ok_or(())?;
        let (w, h, hx, hy) = cursor
            .frame_info(0)
            .map(|(w, h, hx, hy, _)| (w as i32, h as i32, hx as i32, hy as i32))
            .unwrap_or((0, 0, 0, 0));

        if let Some(s) = serial {
            *last_serial = s;
        }

        surface.attach(Some(&buffer), 0, 0);
        if surface.version() >= 4 {
            surface.damage_buffer(0, 0, w, h);
        } else {
            // surface is old and does not support damage_buffer, so we damage
            // in surface coordinates and hope it is not rescaled
            surface.damage(0, 0, w, h);
        }
        surface.commit();
        self.pointer.set_cursor(*last_serial, Some(surface), hx, hy);

        Ok(())
    }
}

/// Initialize a new pointer as a ThemedPointer with an adapter implementation
///
/// You need to provide an implementation as if implementing a `wl_pointer`, but
/// it will receive as `meta` argument a `ThemedPointer` wrapping your pointer,
/// rather than a `Proxy<WlPointer>`.
///
/// Also provide a `ThemedPointer`. Fails and returns the `NewProxy<WlPointer>` if
/// `libwayland-cursor` is not available.
pub fn init_pointer_themed<Impl>(
    pointer: NewProxy<wl_pointer::WlPointer>,
    name: Option<&str>,
    compositor: &Proxy<wl_compositor::WlCompositor>,
    shm: &Proxy<wl_shm::WlShm>,
    mut implementation: Impl,
) -> Result<ThemedPointer, NewProxy<wl_pointer::WlPointer>>
where
    Impl: Implementation<ThemedPointer, wl_pointer::Event> + Send + 'static,
{
    if !is_available() {
        return Err(pointer);
    }

    let theme = load_theme(name, 16, shm);
    let surface = compositor.create_surface().unwrap().implement(|_, _| {});

    let inner = Arc::new(Mutex::new(Inner {
        surface: surface,
        theme: theme,
        last_serial: 0,
    }));
    let inner2 = inner.clone();

    let pointer = pointer.implement(move |event, ptr| {
        implementation.receive(
            event,
            ThemedPointer {
                pointer: ptr,
                inner: inner.clone(),
            },
        )
    });

    Ok(ThemedPointer {
        pointer: pointer,
        inner: inner2,
    })
}

/// Initialize a new pointer as a ThemedPointer with an adapter implementation
///
/// Like `init_pointer_themed`, but does not require your implementation to be
/// `Send`.
///
/// It is unsafe for the same reasons as `NewProxy::implement_nonsend`.
pub unsafe fn init_pointer_themed_nonsend<Impl>(
    pointer: NewProxy<wl_pointer::WlPointer>,
    name: Option<&str>,
    compositor: &Proxy<wl_compositor::WlCompositor>,
    shm: &Proxy<wl_shm::WlShm>,
    mut implementation: Impl,
    token: &QueueToken,
) -> Result<ThemedPointer, NewProxy<wl_pointer::WlPointer>>
where
    Impl: Implementation<ThemedPointer, wl_pointer::Event> + Send + 'static,
{
    if !is_available() {
        return Err(pointer);
    }

    let theme = load_theme(name, 16, shm);
    let surface = compositor.create_surface().unwrap().implement(|_, _| {});

    let inner = Arc::new(Mutex::new(Inner {
        surface: surface,
        theme: theme,
        last_serial: 0,
    }));
    let inner2 = inner.clone();

    let pointer = pointer.implement_nonsend(
        move |event, ptr| {
            implementation.receive(
                event,
                ThemedPointer {
                    pointer: ptr,
                    inner: inner.clone(),
                },
            )
        },
        token,
    );

    Ok(ThemedPointer {
        pointer: pointer,
        inner: inner2,
    })
}

impl Clone for ThemedPointer {
    fn clone(&self) -> ThemedPointer {
        ThemedPointer {
            pointer: self.pointer.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl Deref for ThemedPointer {
    type Target = Proxy<wl_pointer::WlPointer>;
    fn deref(&self) -> &Proxy<wl_pointer::WlPointer> {
        &self.pointer
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.surface.destroy();
    }
}
