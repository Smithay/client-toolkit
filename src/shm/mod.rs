pub mod pool;

use wayland_client::{
    protocol::wl_shm, ConnectionHandle, DataInit, DelegateDispatch, DelegateDispatchBase, Dispatch,
    QueueHandle, WEnum,
};

pub trait ShmHandler {
    /// Indicates the compositor supports an SHM pool with the specified format.
    fn supported_format(&mut self, format: wl_shm::Format);
}

#[derive(Debug)]
pub struct ShmState {
    wl_shm: Option<wl_shm::WlShm>,
}

impl ShmState {
    pub fn new() -> ShmState {
        ShmState { wl_shm: None }
    }

    #[deprecated = "This is a temporary hack until some way to notify delegates a global was created is available."]
    pub fn shm_bind(&mut self, wl_shm: wl_shm::WlShm) {
        self.wl_shm = Some(wl_shm);
    }
}

#[derive(Debug)]
pub struct ShmDispatch<'s, H: ShmHandler>(pub &'s mut ShmState, pub &'s mut H);

impl<H: ShmHandler> DelegateDispatchBase<wl_shm::WlShm> for ShmDispatch<'_, H> {
    type UserData = ();
}

impl<D, H> DelegateDispatch<wl_shm::WlShm, D> for ShmDispatch<'_, H>
where
    H: ShmHandler,
    D: Dispatch<wl_shm::WlShm, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        _proxy: &wl_shm::WlShm,
        event: wl_shm::Event,
        _: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
        _: &mut DataInit<'_>,
    ) {
        match event {
            wl_shm::Event::Format { format } => {
                match format {
                    WEnum::Value(format) => {
                        self.1.supported_format(format);
                    }

                    // Ignore formats we don't know about.
                    WEnum::Unknown(_) => (),
                };
            }

            _ => unreachable!(),
        }
    }
}
