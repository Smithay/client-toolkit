pub mod pool;

use wayland_client::{
    protocol::{wl_shm, wl_shm_pool},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, QueueHandle, WEnum,
};

use crate::registry::{RegistryHandle, RegistryHandler};

use self::pool::{raw::RawPool, simple::SimplePool, CreatePoolError};

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
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<SimplePool, CreatePoolError>
    where
        D: Dispatch<wl_shm_pool::WlShmPool, UserData = U> + 'static,
        U: Send + Sync + 'static,
    {
        Ok(SimplePool { inner: self.new_raw_pool(len, cx, qh, udata)? })
    }

    /// Creates a new raw pool.
    ///
    /// In most cases this is not what you want. You should use TODO name here or TODO in most cases.
    pub fn new_raw_pool<D, U>(
        &self,
        len: usize,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<RawPool, CreatePoolError>
    where
        D: Dispatch<wl_shm_pool::WlShmPool, UserData = U> + 'static,
        U: Send + Sync + 'static,
    {
        let (_, shm) = self.wl_shm.as_ref().ok_or(CreatePoolError::MissingShmGlobal)?;

        RawPool::new(len, shm, cx, qh, udata)
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
/// use smithay_client_toolkit::shm::ShmState;
/// use smithay_client_toolkit::delegate_shm;
///
/// struct ExampleApp {
///     /// The state object that will be our delegate.
///     shm: ShmState,
/// }
///
/// // Use the macro to delegate wl_shm and wl_shm_pool to ShmState.
/// delegate_shm!(ExampleApp ; |app| {
///     // This needs to return an `&mut ShmState` to dispatch events with.
///     &mut app.shm
/// });
#[macro_export]
macro_rules! delegate_shm {
    ($ty: ty: |$dispatcher: ident| $closure: block) => {
        $crate::reexports::client::delegate_dispatch!($ty:
            [
                $crate::reexports::client::protocol::wl_shm::WlShm,
                $crate::reexports::client::protocol::wl_shm_pool::WlShmPool
            ] => $crate::shm::ShmState ; |$dispatcher| { $closure }
        );
    };
}

impl DelegateDispatchBase<wl_shm::WlShm> for ShmState {
    type UserData = ();
}

impl<D> DelegateDispatch<wl_shm::WlShm, D> for ShmState
where
    D: Dispatch<wl_shm::WlShm, UserData = Self::UserData>,
{
    fn event(
        &mut self,
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
                        self.formats.push(format);
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
    D: Dispatch<wl_shm_pool::WlShmPool, UserData = Self::UserData>,
{
    fn event(
        &mut self,
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
    D: Dispatch<wl_shm::WlShm, UserData = ()> + 'static,
{
    fn new_global(
        &mut self,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        _: u32,
        handle: &mut RegistryHandle,
    ) {
        if interface == "wl_shm" {
            let shm = handle
                .bind_once::<wl_shm::WlShm, _, _>(cx, qh, name, 1, ())
                .expect("Failed to bind global");

            self.wl_shm = Some((name, shm));
        }
    }

    fn remove_global(&mut self, _cx: &mut ConnectionHandle, name: u32) {
        if let Some((bound_name, _)) = &self.wl_shm {
            if *bound_name == name {
                // No destructor, simply toss the contents of the option.
                self.wl_shm.take();
            }
        }
    }
}
