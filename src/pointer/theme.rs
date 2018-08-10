use std::ops::Deref;
use std::sync::{Arc, Mutex};
use wayland_client::commons::Implementation;
use wayland_client::cursor::{is_available, load_theme, CursorTheme};
use wayland_client::protocol::{wl_compositor, wl_pointer, wl_shm, wl_surface};
use wayland_client::{NewProxy, Proxy, QueueToken};

use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use wayland_client::protocol::wl_pointer::RequestsTrait as PointerRequests;
use wayland_client::protocol::wl_surface::RequestsTrait as SurfaceRequests;

/// Wrapper managing a system theme for pointer images
///
/// You can use it to initialize new pointers in order
/// to theme them.
///
/// Is is also clone-able in case you need to handle several
/// pointer theming from different places.
///
/// Note that it is however not `Send` nor `Sync`
pub struct ThemeManager {
    theme: Arc<Mutex<CursorTheme>>,
    compositor: Proxy<wl_compositor::WlCompositor>,
}

impl ThemeManager {
    /// Load a system pointer theme
    ///
    /// Will use the default theme of the system if name is `None`.
    ///
    /// Fails if `libwayland-cursor` is not available.
    pub fn init(
        name: Option<&str>,
        compositor: Proxy<wl_compositor::WlCompositor>,
        shm: Proxy<wl_shm::WlShm>,
    ) -> Result<ThemeManager, ()> {
        if !is_available() {
            return Err(());
        }

        Ok(ThemeManager {
            compositor,
            theme: Arc::new(Mutex::new(load_theme(name, 16, &shm))),
        })
    }

    /// Wrap a pointer to theme it
    pub fn theme_pointer(&self, pointer: Proxy<wl_pointer::WlPointer>) -> ThemedPointer {
        let surface = self
            .compositor
            .create_surface()
            .unwrap()
            .implement(|_, _| {});
        ThemedPointer {
            pointer,
            inner: Arc::new(Mutex::new(PointerInner {
                surface,
                theme: self.theme.clone(),
                last_serial: 0,
            })),
        }
    }

    /// Initialize a new pointer as a ThemedPointer with an adapter implementation
    ///
    /// You need to provide an implementation as if implementing a `wl_pointer`, but
    /// it will receive as `meta` argument a `ThemedPointer` wrapping your pointer,
    /// rather than a `Proxy<WlPointer>`.
    pub fn theme_pointer_with_impl<Impl>(
        &self,
        pointer: NewProxy<wl_pointer::WlPointer>,
        mut implementation: Impl,
    ) -> ThemedPointer
    where
        Impl: Implementation<ThemedPointer, wl_pointer::Event> + Send + 'static,
    {
        let surface = self
            .compositor
            .create_surface()
            .unwrap()
            .implement(|_, _| {});

        let inner = Arc::new(Mutex::new(PointerInner {
            surface,
            theme: self.theme.clone(),
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

        ThemedPointer {
            pointer,
            inner: inner2,
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
    ) -> ThemedPointer
    where
        Impl: Implementation<ThemedPointer, wl_pointer::Event> + Send + 'static,
    {
        let surface = self
            .compositor
            .create_surface()
            .unwrap()
            .implement(|_, _| {});

        let inner = Arc::new(Mutex::new(PointerInner {
            surface,
            theme: self.theme.clone(),
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

        ThemedPointer {
            pointer,
            inner: inner2,
        }
    }
}

struct PointerInner {
    surface: Proxy<wl_surface::WlSurface>,
    theme: Arc<Mutex<CursorTheme>>,
    last_serial: u32,
}

/// Wrapper of a themed pointer
///
/// You can access the underlying `Proxy<wl_pointer::WlPointer>` via
/// deref. It will *not* release the proxy when dropped.
///
/// Just like `Proxy`, this is a `Rc`-like wrapper. You can clone it
/// to have several handles to the same theming machinnery of a pointer.
pub struct ThemedPointer {
    pointer: Proxy<wl_pointer::WlPointer>,
    inner: Arc<Mutex<PointerInner>>,
}

impl ThemedPointer {
    /// Change the cursor to the given cursor name
    ///
    /// Possible names depend on the theme. Does nothing and returns
    /// `Err(())` if given name is not available.
    ///
    /// If this is done as an anwser to an input event, you need to provide
    /// the associated serial otherwise the server may ignore the request.
    pub fn set_cursor(&self, name: &str, serial: Option<u32>) -> Result<(), ()> {
        let mut inner = self.inner.lock().unwrap();
        let PointerInner {
            ref theme,
            ref surface,
            ref mut last_serial,
        } = *inner;

        let theme = theme.lock().unwrap();
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

impl Drop for PointerInner {
    fn drop(&mut self) {
        self.surface.destroy();
    }
}
