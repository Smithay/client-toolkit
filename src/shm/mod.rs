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
    dispatch2::Dispatch2,
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

impl<D> Dispatch2<wl_shm::WlShm, D> for GlobalData
where
    D: ShmHandler,
{
    fn event(
        &self,
        state: &mut D,
        _proxy: &wl_shm::WlShm,
        event: wl_shm::Event,
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
