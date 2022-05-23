pub mod pool;

use wayland_client::{
    protocol::wl_shm, Connection, DelegateDispatch, Dispatch, QueueHandle, WEnum,
};

use crate::{
    error::GlobalError,
    registry::{ProvidesRegistryState, RegistryHandler},
};

use self::pool::{raw::RawPool, slot::SlotPool, CreatePoolError};

pub trait ShmHandler {
    fn shm_state(&mut self) -> &mut ShmState;
}

#[derive(Debug)]
pub struct ShmState {
    wl_shm: Option<(u32, wl_shm::WlShm)>, // (name, global)
    formats: Vec<wl_shm::Format>,
}

impl ShmState {
    pub fn new() -> ShmState {
        ShmState { wl_shm: None, formats: vec![] }
    }

    pub fn wl_shm(&self) -> Option<&wl_shm::WlShm> {
        self.wl_shm.as_ref().map(|(_, shm)| shm)
    }

    pub fn new_slot_pool(&self, len: usize) -> Result<SlotPool, CreatePoolError> {
        Ok(SlotPool::new(self.new_raw_pool(len)?))
    }

    /// Creates a new raw pool.
    ///
    /// In most cases this is not what you want. You should use TODO name here or TODO in most cases.
    pub fn new_raw_pool(&self, len: usize) -> Result<RawPool, CreatePoolError> {
        let (_, shm) = self.wl_shm.as_ref().ok_or(GlobalError::MissingGlobals(&["wl_shm"]))?;

        RawPool::new(len, shm)
    }

    /// Returns the formats supported in memory pools.
    pub fn formats(&self) -> &[wl_shm::Format] {
        &self.formats[..]
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
    ($ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($ty:
            [
                $crate::reexports::client::protocol::wl_shm::WlShm: (),
            ] => $crate::shm::ShmState
        );
    };
}

impl<D> DelegateDispatch<wl_shm::WlShm, (), D> for ShmState
where
    D: Dispatch<wl_shm::WlShm, ()> + ShmHandler,
{
    fn event(
        state: &mut D,
        _proxy: &wl_shm::WlShm,
        event: wl_shm::Event,
        _: &(),
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
    D: Dispatch<wl_shm::WlShm, ()> + ShmHandler + ProvidesRegistryState + 'static,
{
    fn new_global(
        state: &mut D,
        _conn: &Connection,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        _: u32,
    ) {
        if interface == "wl_shm" {
            let shm = state
                .registry()
                .bind_once::<wl_shm::WlShm, _, _>(qh, name, 1, ())
                .expect("Failed to bind global");

            state.shm_state().wl_shm = Some((name, shm));
        }
    }

    fn remove_global(_state: &mut D, _conn: &Connection, _qh: &QueueHandle<D>, _name: u32) {
        // Capability global, removal is unlikely
    }
}
