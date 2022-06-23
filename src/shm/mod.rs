pub mod pool;

use wayland_client::{
    protocol::wl_shm, Connection, DelegateDispatch, Dispatch, QueueHandle, WEnum,
};

use crate::{
    error::GlobalError,
    globals::ProvidesBoundGlobal,
    registry::{GlobalProxy, ProvidesRegistryState, RegistryHandler},
};

use self::pool::{multi::MultiPool, raw::RawPool, slot::SlotPool, CreatePoolError};

pub trait ShmHandler {
    fn shm_state(&mut self) -> &mut ShmState;
}

#[derive(Debug)]
pub struct ShmState {
    wl_shm: GlobalProxy<wl_shm::WlShm>,
    formats: Vec<wl_shm::Format>,
}

impl From<wl_shm::WlShm> for ShmState {
    fn from(wl_shm: wl_shm::WlShm) -> Self {
        Self { wl_shm: GlobalProxy::Bound(wl_shm), formats: Vec::new() }
    }
}

impl ShmState {
    pub fn new() -> ShmState {
        ShmState { wl_shm: GlobalProxy::NotReady, formats: vec![] }
    }

    pub fn wl_shm(&self) -> Result<&wl_shm::WlShm, GlobalError> {
        self.wl_shm.get()
    }

    pub fn new_slot_pool(&self, len: usize) -> Result<SlotPool, CreatePoolError> {
        SlotPool::new(len, self)
    }

    pub fn new_multi_pool<K>(&self, len: usize) -> Result<MultiPool<K>, CreatePoolError> {
        Ok(MultiPool::new(self.new_raw_pool(len)?))
    }

    /// Creates a new raw pool.
    ///
    /// In most cases this is not what you want. You should use TODO name here or TODO in most cases.
    pub fn new_raw_pool(&self, len: usize) -> Result<RawPool, CreatePoolError> {
        let shm = self.wl_shm()?;

        RawPool::new(len, shm)
    }

    /// Returns the formats supported in memory pools.
    pub fn formats(&self) -> &[wl_shm::Format] {
        &self.formats[..]
    }
}

impl ProvidesBoundGlobal<wl_shm::WlShm, 1> for ShmState {
    fn bound_global(&self) -> Result<wl_shm::WlShm, GlobalError> {
        self.wl_shm().cloned()
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
    fn ready(state: &mut D, _conn: &Connection, qh: &QueueHandle<D>) {
        state.shm_state().wl_shm = state.registry().bind_one(qh, 1..=1, ()).into();
    }
}
