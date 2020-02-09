use std::ops::Deref;
use std::sync::{Arc, Mutex};

use wayland_client::cursor::{is_available, load_theme, Cursor, CursorTheme};
use wayland_client::protocol::{wl_compositor, wl_pointer, wl_seat, wl_shm, wl_surface};
use wayland_client::NewProxy;

struct ScaledThemeList {
    shm: wl_shm::WlShm,
    name: Option<String>,
    size: u32,
    themes: Vec<(u32, CursorTheme)>,
}

impl ScaledThemeList {
    fn new(mut name: Option<String>, shm: wl_shm::WlShm) -> ScaledThemeList {
        let size = std::env::var("XCURSOR_SIZE")
            .ok()
            .and_then(|size| size.parse().ok())
            .unwrap_or(24);

        if name.is_none() {
            name = std::env::var("XCURSOR_THEME").ok().and_then(|name| {
                if name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            });
        }

        ScaledThemeList {
            shm,
            name,
            size,
            themes: Vec::new(),
        }
    }

    fn get_cursor(&mut self, name: &str, scale: u32) -> Option<Cursor> {
        // Check if we already loaded the theme for this scale factor
        let opt_index = self.themes.iter().position(|&(s, _)| s == scale);
        if let Some(idx) = opt_index {
            self.themes[idx].1.get_cursor(name)
        } else {
            let new_theme = load_theme(
                self.name.as_ref().map(|s| &s[..]),
                self.size * scale,
                &self.shm,
            );
            self.themes.push((scale, new_theme));
            self.themes.last().unwrap().1.get_cursor(name)
        }
    }
}

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
    theme: Arc<Mutex<ScaledThemeList>>,
    compositor: wl_compositor::WlCompositor,
}

impl ThemeManager {
    /// Load a system pointer theme
    ///
    /// Will use the default theme of the system if name is `None` or `XCURSOR_THEME` is unset.
    /// The size of the theme is controlled by `XCURSOR_SIZE` environment variable.
    ///
    /// Fails if `libwayland-cursor` is not available.
    pub fn init(
        name: Option<&str>,
        compositor: wl_compositor::WlCompositor,
        shm: &wl_shm::WlShm,
    ) -> Result<ThemeManager, ()> {
        if !is_available() {
            return Err(());
        }

        Ok(ThemeManager {
            compositor,
            theme: Arc::new(Mutex::new(ScaledThemeList::new(
                name.map(|name| name.to_owned()),
                shm.clone(),
            ))),
        })
    }

    /// Wrap a pointer to theme it
    pub fn theme_pointer(&self, pointer: wl_pointer::WlPointer) -> ThemedPointer {
        let surface = self
            .compositor
            .create_surface(NewProxy::implement_dummy)
            .unwrap();
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
    /// rather than a `WlPointer`.
    pub fn theme_pointer_with_impl<Impl, UD>(
        &self,
        seat: &wl_seat::WlSeat,
        mut implementation: Impl,
        user_data: UD,
    ) -> ThemedPointer
    where
        Impl: FnMut(wl_pointer::Event, ThemedPointer) + 'static,
        UD: 'static,
    {
        let surface = self
            .compositor
            .create_surface(NewProxy::implement_dummy)
            .unwrap();

        let inner = Arc::new(Mutex::new(PointerInner {
            surface,
            theme: self.theme.clone(),
            last_serial: 0,
        }));
        let inner2 = inner.clone();

        let pointer = seat
            .get_pointer(|pointer| {
                pointer.implement_closure(
                    move |event, ptr| {
                        implementation(
                            event,
                            ThemedPointer {
                                pointer: ptr,
                                inner: inner.clone(),
                            },
                        )
                    },
                    user_data,
                )
            })
            .unwrap();

        ThemedPointer {
            pointer,
            inner: inner2,
        }
    }
}

struct PointerInner {
    surface: wl_surface::WlSurface,
    theme: Arc<Mutex<ScaledThemeList>>,
    last_serial: u32,
}

/// Wrapper of a themed pointer
///
/// You can access the underlying `wl_pointer::WlPointer` via
/// deref. It will *not* release the proxy when dropped.
///
/// Just like `Proxy`, this is a `Rc`-like wrapper. You can clone it
/// to have several handles to the same theming machinery of a pointer.
pub struct ThemedPointer {
    pointer: wl_pointer::WlPointer,
    inner: Arc<Mutex<PointerInner>>,
}

impl ThemedPointer {
    /// Change the cursor to the given cursor name
    ///
    /// Possible names depend on the theme. Does nothing and returns
    /// `Err(())` if given name is not available.
    ///
    /// If this is done as an answer to an input event, you need to provide
    /// the associated serial otherwise the server may ignore the request.
    pub fn set_cursor(&self, name: &str, serial: Option<u32>) -> Result<(), ()> {
        self.set_cursor_with_scale(name, 1, serial)
    }

    /// Change the cursor to the given cursor name and apply the given scale to an underlying
    /// cursor surface
    ///
    /// Possible names depend on the theme. Does nothing and returns
    /// `Err(())` if given name is not available.
    ///
    /// If this is done as an answer to an input event, you need to provide
    /// the associated serial otherwise the server may ignore the request.
    pub fn set_cursor_with_scale(
        &self,
        name: &str,
        scale: u32,
        serial: Option<u32>,
    ) -> Result<(), ()> {
        let mut inner = self.inner.lock().unwrap();
        let PointerInner {
            ref theme,
            ref surface,
            ref mut last_serial,
        } = *inner;

        let mut theme = theme.lock().unwrap();
        let cursor = theme.get_cursor(name, scale).ok_or(())?;
        let buffer = cursor.frame_buffer(0).ok_or(())?;
        let scale = scale as i32;
        let (w, h, hx, hy) = cursor
            .frame_info(0)
            .map(|(w, h, hx, hy, _)| (w as i32, h as i32, hx as i32 / scale, hy as i32 / scale))
            .unwrap_or((0, 0, 0, 0));

        if let Some(s) = serial {
            *last_serial = s;
        }

        surface.set_buffer_scale(scale);
        surface.attach(Some(&buffer), 0, 0);
        if surface.as_ref().version() >= 4 {
            surface.damage_buffer(0, 0, w, h);
        } else {
            // surface is old and does not support damage_buffer, so we damage
            // in surface coordinates and hope it is not rescaled
            surface.damage(0, 0, w / scale, h / scale);
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
    type Target = wl_pointer::WlPointer;
    fn deref(&self) -> &wl_pointer::WlPointer {
        &self.pointer
    }
}

impl Drop for PointerInner {
    fn drop(&mut self) {
        self.surface.destroy();
    }
}
