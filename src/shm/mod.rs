pub mod pool;

use wayland_client::{
    protocol::{wl_shm, wl_shm_pool},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, QueueHandle, WEnum,
};

use crate::registry::{ProvidesRegistryState, RegistryHandler};

use self::pool::{raw::RawPool, simple::SimplePool, CreatePoolError};

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

    pub fn new_simple_pool<D, U>(
        &self,
        len: usize,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<SimplePool, CreatePoolError>
    where
        D: Dispatch<wl_shm_pool::WlShmPool, UserData = U> + 'static,
        U: Send + Sync + 'static,
    {
        Ok(SimplePool { inner: self.new_raw_pool(len, conn, qh, udata)? })
    }

    /// Creates a new raw pool.
    ///
    /// In most cases this is not what you want. You should use TODO name here or TODO in most cases.
    pub fn new_raw_pool<D, U>(
        &self,
        len: usize,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<RawPool, CreatePoolError>
    where
        D: Dispatch<wl_shm_pool::WlShmPool, UserData = U> + 'static,
        U: Send + Sync + 'static,
    {
        let (_, shm) = self.wl_shm.as_ref().ok_or(CreatePoolError::MissingShmGlobal)?;

        RawPool::new(len, shm, conn, qh, udata)
    }

    /// Returns the formats supported in memory pools.
    pub fn formats(&self) -> &[wl_shm::Format] {
        &self.formats[..]
    }
}

/// Delegates the handling of [`wl_shm`] and [`wl_shm_pool`] to some [`ShmState`].
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
/// // Use the macro to delegate wl_shm and wl_shm_pool to ShmState.
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
                $crate::reexports::client::protocol::wl_shm::WlShm,
                $crate::reexports::client::protocol::wl_shm_pool::WlShmPool
            ] => $crate::shm::ShmState
        );
    };
}

impl DelegateDispatchBase<wl_shm::WlShm> for ShmState {
    type UserData = ();
}

impl<D> DelegateDispatch<wl_shm::WlShm, D> for ShmState
where
    D: Dispatch<wl_shm::WlShm, UserData = Self::UserData> + ShmHandler,
{
    fn event(
        state: &mut D,
        _proxy: &wl_shm::WlShm,
        event: wl_shm::Event,
        _: &Self::UserData,
        _: &mut ConnectionHandle,
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

impl DelegateDispatchBase<wl_shm_pool::WlShmPool> for ShmState {
    type UserData = ();
}

impl<D> DelegateDispatch<wl_shm_pool::WlShmPool, D> for ShmState
where
    D: Dispatch<wl_shm_pool::WlShmPool, UserData = Self::UserData> + ShmHandler,
{
    fn event(
        _: &mut D,
        _: &wl_shm_pool::WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        unreachable!("wl_shm_pool has no events")
    }
}

impl<D> RegistryHandler<D> for ShmState
where
    D: Dispatch<wl_shm::WlShm, UserData = ()> + ShmHandler + ProvidesRegistryState + 'static,
{
    fn new_global(
        state: &mut D,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        _: u32,
    ) {
        if interface == "wl_shm" {
            let shm = state
                .registry()
                .bind_once::<wl_shm::WlShm, _, _>(conn, qh, name, 1, ())
                .expect("Failed to bind global");

            state.shm_state().wl_shm = Some((name, shm));
        }
    }

    fn remove_global(state: &mut D, _conn: &mut ConnectionHandle, _qh: &QueueHandle<D>, name: u32) {
        if let Some((bound_name, _)) = &state.shm_state().wl_shm {
            if *bound_name == name {
                // No destructor, simply toss the contents of the option.
                state.shm_state().wl_shm.take();
            }
        }
    }
}
