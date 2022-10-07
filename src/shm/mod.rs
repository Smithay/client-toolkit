pub mod multi;
pub mod raw;
pub mod slot;

use std::{
    collections::{hash_map::Entry, HashMap},
    env, io,
    sync::{Arc, Mutex},
};

use nix::errno::Errno;
use wayland_backend::client::InvalidId;
use wayland_client::{
    protocol::{wl_pointer, wl_shm, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_cursor::{Cursor, CursorTheme};

use crate::{
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
    registry::{GlobalProxy, ProvidesRegistryState, RegistryHandler},
};

pub trait ShmHandler {
    fn shm_state(&mut self) -> &mut ShmState;
}

#[derive(Debug)]
pub struct ShmState {
    wl_shm: GlobalProxy<wl_shm::WlShm>,
    formats: Vec<wl_shm::Format>,
    pointer_themes: Arc<Mutex<Themes>>,
}

impl From<wl_shm::WlShm> for ShmState {
    fn from(wl_shm: wl_shm::WlShm) -> Self {
        Self {
            wl_shm: GlobalProxy::Bound(wl_shm),
            formats: Vec::new(),
            pointer_themes: Arc::new(Mutex::new(Themes::new(ThemeSpec::System))),
        }
    }
}

impl ShmState {
    pub fn new() -> ShmState {
        ShmState {
            wl_shm: GlobalProxy::NotReady,
            formats: vec![],
            pointer_themes: Arc::new(Mutex::new(Themes::new(ThemeSpec::System))),
        }
    }

    pub fn with_pointer_theme(theme: ThemeSpec) -> ShmState {
        ShmState {
            wl_shm: GlobalProxy::NotReady,
            formats: vec![],
            pointer_themes: Arc::new(Mutex::new(Themes::new(theme))),
        }
    }

    pub fn wl_shm(&self) -> Result<&wl_shm::WlShm, GlobalError> {
        self.wl_shm.get()
    }

    /// Returns the formats supported in memory pools.
    pub fn formats(&self) -> &[wl_shm::Format] {
        &self.formats[..]
    }

    /// Theme the Pointer
    pub fn theme_pointer(
        &self,
        surface: wl_surface::WlSurface,
        pointer: wl_pointer::WlPointer,
    ) -> Result<ThemedPointer, PointerThemeError> {
        let shm = self.wl_shm.get().map_err(PointerThemeError::GlobalError)?;

        Ok(ThemedPointer {
            shm: shm.clone(),
            themes: self.pointer_themes.clone(),
            surface,
            pointer,
            current: "left_ptr".into(),
            scale: 1,
            last_serial: 0,
        })
    }
}

impl ProvidesBoundGlobal<wl_shm::WlShm, 1> for ShmState {
    fn bound_global(&self) -> Result<wl_shm::WlShm, GlobalError> {
        self.wl_shm().cloned()
    }
}

/// An error that may occur when creating a pool.
#[derive(Debug, thiserror::Error)]
pub enum CreatePoolError {
    /// The wl_shm global is not bound.
    #[error(transparent)]
    Global(#[from] GlobalError),

    /// Error while allocating the shared memory.
    #[error(transparent)]
    Create(#[from] io::Error),
}

impl From<Errno> for CreatePoolError {
    fn from(errno: Errno) -> Self {
        Into::<io::Error>::into(errno).into()
    }
}

/// Delegates the handling of [`wl_shm`] to some [`ShmState`].
///
/// This macro requires two things, the type that will delegate to [`ShmState`] and a closure specifying how
/// to obtain the state object.
///
/// ```
/// use smithay_client_toolkit::shm::{ShmHandler, ShmState};
/// use smithay_client_toolkit::delegate_shm;
///
/// struct ExampleApp {
///     /// The state object that will be our delegate.
///     shm: ShmState,
/// }
///
/// // Use the macro to delegate wl_shm to ShmState.
/// delegate_shm!(ExampleApp);
///
/// // You must implement the ShmHandler trait to provide a way to access the ShmState from your data type.
/// impl ShmHandler for ExampleApp {
///     fn shm_state(&mut self) -> &mut ShmState {
///         &mut self.shm
///     }
/// }
#[macro_export]
macro_rules! delegate_shm {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_shm::WlShm: $crate::globals::GlobalData
            ] => $crate::shm::ShmState
        );
    };
}

impl<D> Dispatch<wl_shm::WlShm, GlobalData, D> for ShmState
where
    D: Dispatch<wl_shm::WlShm, GlobalData> + ShmHandler,
{
    fn event(
        state: &mut D,
        _proxy: &wl_shm::WlShm,
        event: wl_shm::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            wl_shm::Event::Format { format } => {
                match format {
                    WEnum::Value(format) => {
                        state.shm_state().formats.push(format);
                        log::debug!(target: "sctk", "supported wl_shm format {:?}", format);
                    }

                    // Ignore formats we don't know about.
                    WEnum::Unknown(raw) => {
                        log::debug!(target: "sctk", "Unknown supported wl_shm format {:x}", raw);
                    }
                };
            }

            _ => unreachable!(),
        }
    }
}

impl<D> RegistryHandler<D> for ShmState
where
    D: Dispatch<wl_shm::WlShm, GlobalData> + ShmHandler + ProvidesRegistryState + 'static,
{
    fn ready(state: &mut D, _conn: &Connection, qh: &QueueHandle<D>) {
        state.shm_state().wl_shm = state.registry().bind_one(qh, 1..=1, GlobalData).into();
    }
}

/// Pointer themeing
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
        conn: &Connection,
        name: &str,
        serial: Option<u32>,
    ) -> Result<(), PointerThemeError> {
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

/// An error indicating that the cursor was not found.
#[derive(Debug, thiserror::Error)]
pub enum PointerThemeError {
    /// An invalid ObjectId was used.
    #[error("Invalid ObjectId")]
    InvalidId(InvalidId),

    /// A global error occurred.
    #[error("A Global Error occured")]
    GlobalError(GlobalError),

    /// The requested cursor was not found.
    #[error("Cursor not found")]
    CursorNotFound,
}

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
        conn: &Connection,
        name: &str,
        scale: u32,
        shm: &wl_shm::WlShm,
    ) -> Result<Option<&Cursor>, InvalidId> {
        // Check if the theme has been initialized at the specified scale.
        if let Entry::Vacant(e) = self.themes.entry(scale) {
            // Initialize the theme for the specified scale
            let theme = CursorTheme::load_from_name(
                conn,
                shm.clone(), // TODO: Does the cursor theme need to clone wl_shm?
                &self.name,
                self.size * scale,
            )?;

            e.insert(theme);
        }

        let theme = self.themes.get_mut(&scale).unwrap();

        Ok(theme.get_cursor(name))
    }
}

impl ThemedPointer {
    fn update_cursor(&self, conn: &Connection) -> Result<(), PointerThemeError> {
        let mut themes = self.themes.lock().unwrap();

        let cursor = themes
            .get_cursor(conn, &self.current, self.scale as u32, &self.shm)
            .map_err(PointerThemeError::InvalidId)?
            .ok_or(PointerThemeError::CursorNotFound)?;

        let image = &cursor[0];
        let (w, h) = image.dimensions();
        let (hx, hy) = image.hotspot();

        self.surface.set_buffer_scale(self.scale);
        self.surface.attach(Some(image), 0, 0);

        if self.surface.version() >= 4 {
            self.surface.damage_buffer(0, 0, w as i32, h as i32);
        } else {
            let scale = self.scale;

            // surface is old and does not support damage_buffer, so we damage
            // in surface coordinates and hope it is not rescaled
            self.surface.damage(0, 0, w as i32 / scale as i32, h as i32 / scale as i32);
        }

        // Commit the surface to place the cursor image in the compositor's memory.
        self.surface.commit();
        // Set the pointer surface to change the pointer.
        self.pointer.set_cursor(self.last_serial, Some(&self.surface), hx as i32, hy as i32);

        Ok(())
    }
}
