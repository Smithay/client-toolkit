pub mod multi;
pub mod raw;
pub mod slot;

use std::io;

use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::wl_shm,
    Connection, Dispatch, QueueHandle, WEnum,
};

use crate::{
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
};

pub trait ShmHandler {
    fn shm_state(&mut self) -> &mut Shm;
}

#[derive(Debug)]
pub struct Shm {
    wl_shm: wl_shm::WlShm,
    formats: Vec<wl_shm::Format>,
}

impl From<wl_shm::WlShm> for Shm {
    fn from(wl_shm: wl_shm::WlShm) -> Self {
        Self { wl_shm, formats: Vec::new() }
    }
}

impl Shm {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<Shm, BindError>
    where
        State: Dispatch<wl_shm::WlShm, GlobalData, State> + ShmHandler + 'static,
    {
        let wl_shm = globals.bind(qh, 1..=1, GlobalData)?;
        // Compositors must advertise Argb8888 and Xrgb8888, so let's reserve space for those formats.
        Ok(Shm { wl_shm, formats: Vec::with_capacity(2) })
    }

    pub fn wl_shm(&self) -> &wl_shm::WlShm {
        &self.wl_shm
    }

    /// Returns the formats supported in memory pools.
    pub fn formats(&self) -> &[wl_shm::Format] {
        &self.formats[..]
    }
}

impl ProvidesBoundGlobal<wl_shm::WlShm, 1> for Shm {
    fn bound_global(&self) -> Result<wl_shm::WlShm, GlobalError> {
        Ok(self.wl_shm.clone())
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

/// Delegates the handling of [`wl_shm`] to some [`Shm`].
///
/// This macro requires two things, the type that will delegate to [`Shm`] and a closure specifying how
/// to obtain the state object.
///
/// ```
/// use smithay_client_toolkit::shm::{ShmHandler, Shm};
/// use smithay_client_toolkit::delegate_shm;
///
/// struct ExampleApp {
///     /// The state object that will be our delegate.
///     shm: Shm,
/// }
///
/// // Use the macro to delegate wl_shm to Shm.
/// delegate_shm!(ExampleApp);
///
/// // You must implement the ShmHandler trait to provide a way to access the Shm from your data type.
/// impl ShmHandler for ExampleApp {
///     fn shm_state(&mut self) -> &mut Shm {
///         &mut self.shm
///     }
/// }
#[macro_export]
macro_rules! delegate_shm {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_shm::WlShm: $crate::globals::GlobalData
            ] => $crate::shm::Shm
        );
    };
}

impl<D> Dispatch<wl_shm::WlShm, GlobalData, D> for Shm
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
