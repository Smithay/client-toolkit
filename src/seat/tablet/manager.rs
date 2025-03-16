use wayland_client::{
    globals::GlobalList,
    protocol::wl_seat::WlSeat,
    Connection,
    Dispatch,
    QueueHandle,
};

use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2::{self, ZwpTabletManagerV2},
    zwp_tablet_seat_v2::ZwpTabletSeatV2,
};

use crate::{error::GlobalError, globals::GlobalData, registry::GlobalProxy};
use super::seat::TabletSeatData;

#[derive(Debug)]
pub struct TabletState {
    tablet_manager: GlobalProxy<ZwpTabletManagerV2>,
}

impl TabletState {
    /// Bind `zwp_tablet_manager_v2` global, if it exists
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ZwpTabletManagerV2, GlobalData> + 'static,
    {
        Self {
            tablet_manager: GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData)),
        }
    }

    pub fn get_tablet_seat<D>(
        &self,
        seat: &WlSeat,
        qh: &QueueHandle<D>,
    ) -> Result<ZwpTabletSeatV2, GlobalError>
    where
        D: Dispatch<ZwpTabletSeatV2, TabletSeatData> + 'static,
    {
        let udata = TabletSeatData { wl_seat: seat.clone() };
        Ok(self.tablet_manager.get()?.get_tablet_seat(seat, qh, udata))
    }
}

impl<D> Dispatch<ZwpTabletManagerV2, GlobalData, D>
    for TabletState
where
    D: Dispatch<ZwpTabletManagerV2, GlobalData>,
    // TODO: which Handler traits, if any, should D be constrained to?
{
    fn event(
        _data: &mut D,
        _manager: &ZwpTabletManagerV2,
        _event: zwp_tablet_manager_v2::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}
