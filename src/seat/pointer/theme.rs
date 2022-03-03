//! Pointer theming helpers
//!
//!

use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};

use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_pointer, wl_shm, wl_surface},
    ConnectionHandle, Dispatch, Proxy, QueueHandle,
};
use wayland_cursor::{Cursor, CursorTheme};

use crate::{
    registry::{ProvidesRegistryState, RegistryHandler},
    shm::WL_SHM_VERSION,
};

#[derive(Debug)]
pub struct ThemedPointer {
    shm: wl_shm::WlShm,
    themes: Arc<Mutex<Themes>>,
    surface: wl_surface::WlSurface,
    pointer: wl_pointer::WlPointer,
    current: String,
    scale: i32,
    last_serial: u32,
}

impl ThemedPointer {
    pub fn set_cursor(
        &mut self,
        conn: &mut ConnectionHandle,
        name: &str,
        serial: Option<u32>,
    ) -> Result<(), CursorNotFound> {
        self.current = name.into();

        if let Some(serial) = serial {
            self.last_serial = serial;
        }

        self.update_cursor(conn)
    }

    pub fn pointer(&self) -> &wl_pointer::WlPointer {
        &self.pointer
    }
}

/// Specifies which cursor theme should be used by the theme manager.
#[derive(Debug)]
pub enum ThemeSpec<'a> {
    /// Use this specific theme with the given base size.
    Named {
        /// Name of the cursor theme.
        name: &'a str,

        /// Base size of the cursor names.
        ///
        /// Note this size assumes a scale factor of 1. Cursor image sizes may be multiplied by the base size
        /// for HiDPI outputs.
        size: u32,
    },

    /// Use the system provided theme
    ///
    /// In this case SCTK will read the `XCURSOR_THEME` and
    /// `XCURSOR_SIZE` environment variables to figure out the
    /// theme to use.
    System,
}

#[derive(Debug)]
pub struct PointerThemeManager {
    wl_shm: Option<(u32, wl_shm::WlShm)>,
    themes: Arc<Mutex<Themes>>,
}

impl PointerThemeManager {
    pub fn new(spec: ThemeSpec) -> PointerThemeManager {
        let themes = Arc::new(Mutex::new(Themes::new(spec)));
        PointerThemeManager { wl_shm: None, themes }
    }

    pub fn theme_pointer<D>(
        &self,
        surface: wl_surface::WlSurface,
        pointer: wl_pointer::WlPointer,
    ) -> Result<ThemedPointer, ()> {
        let (_, shm) = self.wl_shm.as_ref().expect("TODO");

        Ok(ThemedPointer {
            shm: shm.clone(),
            themes: self.themes.clone(),
            surface,
            pointer,
            current: "left_ptr".into(),
            scale: 1,
            last_serial: 0,
        })
    }
}

/// An error indicating that the cursor was not found.
#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("Cursor not found")]
pub struct CursorNotFound;

#[derive(Debug)]
struct Themes {
    name: String,
    size: u32,
    // Scale -> CursorTheme
    themes: HashMap<u32, CursorTheme>,
}

impl Themes {
    fn new(spec: ThemeSpec) -> Themes {
        let (name, size) = match spec {
            ThemeSpec::Named { name, size } => (name.into(), size),
            ThemeSpec::System => {
                let name = env::var("XCURSOR_THEME").ok().unwrap_or_else(|| "default".into());
                let size = env::var("XCURSOR_SIZE").ok().and_then(|s| s.parse().ok()).unwrap_or(24);
                (name, size)
            }
        };

        Themes { name, size, themes: HashMap::new() }
    }

    fn get_cursor(
        &mut self,
        conn: &mut ConnectionHandle,
        name: &str,
        scale: u32,
        shm: &wl_shm::WlShm,
    ) -> Result<Option<&Cursor>, InvalidId> {
        // Check if the theme has been initialized at the specified scale.
        if !self.themes.contains_key(&scale) {
            // Initialize the theme for the specified scale
            let theme = CursorTheme::load_from_name(
                conn,
                shm.clone(), // TODO: Does the cursor theme need to clone wl_shm?
                &self.name,
                self.size * scale,
            )?;

            self.themes.insert(scale, theme);
        }

        let theme = self.themes.get_mut(&scale).unwrap();

        Ok(theme.get_cursor(conn, name))
    }
}

impl ThemedPointer {
    fn update_cursor(&self, conn: &mut ConnectionHandle) -> Result<(), CursorNotFound> {
        let mut themes = self.themes.lock().unwrap();

        let cursor = themes
            .get_cursor(conn, &self.current, self.scale as u32, &self.shm)
            .expect("TODO: Error")
            .ok_or(CursorNotFound)?;

        let image = &cursor[0];
        let (w, h) = image.dimensions();
        let (hx, hy) = image.hotspot();

        self.surface.set_buffer_scale(conn, self.scale);
        self.surface.attach(conn, Some(image), 0, 0);

        if self.surface.version() >= 4 {
            self.surface.damage_buffer(conn, 0, 0, w as i32, h as i32);
        } else {
            let scale = self.scale;

            // surface is old and does not support damage_buffer, so we damage
            // in surface coordinates and hope it is not rescaled
            self.surface.damage(conn, 0, 0, w as i32 / scale as i32, h as i32 / scale as i32);
        }

        // Commit the surface to place the cursor image in the compositor's memory.
        self.surface.commit(conn);
        // Set the pointer surface to change the pointer.
        self.pointer.set_cursor(conn, self.last_serial, Some(&self.surface), hx as i32, hy as i32);

        Ok(())
    }
}

impl<D> RegistryHandler<D> for PointerThemeManager
where
    D: Dispatch<wl_shm::WlShm, UserData = ()>
        + ProvidesRegistryState
        + AsMut<PointerThemeManager>
        + 'static,
{
    fn new_global(
        data: &mut D,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        _: u32,
    ) {
        if interface == "wl_shm" {
            let shm = data
                .registry()
                .bind_cached::<wl_shm::WlShm, _, _, _>(conn, qh, name, || (WL_SHM_VERSION, ()))
                .expect("Failed to bind global");

            data.as_mut().wl_shm = Some((name, shm));
        }
    }

    fn remove_global(_: &mut D, _: &mut ConnectionHandle, _: &QueueHandle<D>, _: u32) {
        // Do nothing since wl_shm is a capability style global.
    }
}
