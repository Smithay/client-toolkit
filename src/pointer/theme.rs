use std::ops::Deref;
use std::sync::{Arc, Mutex};
use wayland_client::cursor::{is_available, load_theme, CursorTheme};
use wayland_client::protocol::{wl_compositor, wl_pointer, wl_seat, wl_shm, wl_surface};
use wayland_client::Proxy;

use wayland_client::protocol::wl_pointer::RequestsTrait as PointerRequests;
use wayland_client::protocol::wl_seat::RequestsTrait as SeatRequests;
use wayland_client::protocol::wl_surface::RequestsTrait as SurfaceRequests;

use env::Environment;
use output::OutputMgr;
use surface::SurfaceManager;

pub struct DpiCursorTheme {
    output_manager: OutputMgr,
    shm: Proxy<wl_shm::WlShm>,
    theme_name: Option<String>,
    theme: CursorTheme,
    dpi_factor: i32,
}

impl DpiCursorTheme {
    pub fn new(environment: &Environment, theme_name: Option<String>) -> Self {
        let output_manager = environment.outputs.clone();
        let shm = environment.shm.clone();
        let (dpi_factor, theme) = DpiCursorTheme::load(&output_manager, &shm, theme_name.as_ref());

        DpiCursorTheme {
            output_manager,
            shm,
            theme_name,
            theme,
            dpi_factor,
        }
    }

    pub fn load(output_manager: &OutputMgr, shm: &Proxy<wl_shm::WlShm>, name: Option<&String>) -> (i32, CursorTheme) {
        let dpi_factor = output_manager
            .with_all(|outputs| {
                outputs
                    .iter()
                    .map(|&(_, _, ref info)| info.scale_factor)
                    .max()
            })
            .unwrap_or(1);

        // No way to find the cursor size
        // Good cursor size for scale factors 1, 2 where determined
        // to be 16 and 48. A linear function is fitted to those points.
        // 32 * 1 - 16 = 16
        // 32 * 2 - 16 = 48
        let size = 32 * dpi_factor - 16;

        let theme = load_theme(name.map(|s| &**s), size as u32, shm);

        (dpi_factor, theme)
    }

    pub fn theme(&self) -> &CursorTheme {
        &self.theme
    }

    pub fn dpi_factor(&self) -> i32 {
        self.dpi_factor
    }

    pub fn reload(&mut self) {
        let (dpi_factor, theme) = DpiCursorTheme::load(&self.output_manager, &self.shm, self.theme_name.as_ref());
        self.dpi_factor = dpi_factor;
        self.theme = theme;
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
    compositor: Proxy<wl_compositor::WlCompositor>,
    surface_manager: SurfaceManager,
    theme: Arc<Mutex<DpiCursorTheme>>,
}

impl ThemeManager {
    /// Load a system pointer theme
    ///
    /// Will use the default theme of the system if name is `None`.
    ///
    /// Fails if `libwayland-cursor` is not available.
    pub fn init(environment: &Environment, theme_name: Option<String>) -> Result<Self, ()> {
        if !is_available() {
            return Err(());
        }

        let theme = Arc::new(Mutex::new(DpiCursorTheme::new(
            environment,
            theme_name,
        )));

        let compositor = environment.compositor.clone();
        let surface_manager = environment.surface_manager.clone();
        Ok(ThemeManager { compositor, surface_manager, theme })
    }

    /// Initialize a new pointer as a ThemedPointer with an adapter implementation
    ///
    /// You need to provide an implementation as if implementing a `wl_pointer`, but
    /// it will receive as `meta` argument a `ThemedPointer` wrapping your pointer,
    /// rather than a `Proxy<WlPointer>`.
    pub fn theme_pointer_with_impl<Impl, UD>(
        &self,
        seat: &Proxy<wl_seat::WlSeat>,
        mut implementation: Impl,
        user_data: UD,
    ) -> ThemedPointer
    where
        Impl: FnMut(wl_pointer::Event, ThemedPointer) + Send + 'static,
        UD: Send + Sync + 'static,
    {
        let surface = {
            let theme = self.theme.clone();
            self.surface_manager.create_surface(&self.compositor, move |dpi, _surface| {
                let mut theme = theme.lock().unwrap();
                if dpi > theme.dpi_factor() {
                    theme.reload();
                }
            })
        };

        let inner = Arc::new(Mutex::new(PointerInner {
            surface,
            theme: self.theme.clone(),
            last_serial: 0,
        }));
        let inner2 = inner.clone();

        let pointer = seat
            .get_pointer(|pointer| {
                pointer.implement(
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
    surface: Proxy<wl_surface::WlSurface>,
    theme: Arc<Mutex<DpiCursorTheme>>,
    last_serial: u32,
}

/// Wrapper of a themed pointer
///
/// You can access the underlying `Proxy<wl_pointer::WlPointer>` via
/// deref. It will *not* release the proxy when dropped.
///
/// Just like `Proxy`, this is a `Rc`-like wrapper. You can clone it
/// to have several handles to the same theming machinery of a pointer.
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
    /// If this is done as an answer to an input event, you need to provide
    /// the associated serial otherwise the server may ignore the request.
    pub fn set_cursor(&self, name: &str, serial: Option<u32>) -> Result<(), ()> {
        let mut inner = self.inner.lock().unwrap();
        let PointerInner {
            ref theme,
            ref surface,
            ref mut last_serial,
        } = *inner;

        let theme = theme.lock().unwrap();
        let cursor = theme.theme().get_cursor(name).ok_or(())?;
        let buffer = cursor.frame_buffer(0).ok_or(())?;
        let (w, h, hx, hy) = cursor
            .frame_info(0)
            .map(|(w, h, hx, hy, _)| (w as i32, h as i32, hx as i32, hy as i32))
            .unwrap_or((0, 0, 0, 0));

        if let Some(s) = serial {
            *last_serial = s;
        }

        surface.attach(Some(&buffer), 0, 0);
        surface.set_buffer_scale(theme.dpi_factor());
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
