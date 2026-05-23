use std::mem;
use std::sync::Mutex;

use wayland_client::{
    Connection,
    Dispatch,
    QueueHandle,
};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_v2::{self, ZwpTabletV2};

pub trait Handler: Sized {
    /// This is fired at the time of the `zwp_tablet_v2.done` event,
    /// and collects any preceding `name`, `id` and `path` events into an [`Info`].
    fn info(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet: &ZwpTabletV2,
        info: Info,
    );

    /// Sent when the tablet has been removed from the system.
    /// When a tablet is removed, some tools may be removed.
    ///
    /// This method is responsible for running `tablet.destroy()`.  ← TODO: true or not?
    fn removed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet: &ZwpTabletV2,
    );
}

/// The description of a tablet device.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Info {
    /// The descriptive name of the tablet device.
    pub name: Option<String>,
    /// The USB vendor and product IDs for the tablet device.
    pub id: Option<(u32, u32)>,
    /// System-specific device paths for the tablet.
    ///
    /// Path format is unspecified.
    /// Clients must figure out what to do with them, if they care.
    pub paths: Vec<String>,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Data {
    info: Mutex<Info>,
}

impl Data {
    pub fn new() -> Self {
        Self { info: Default::default() }
    }
}

impl<D> Dispatch<ZwpTabletV2, Data, D>
    for super::TabletManager
where
    D: Dispatch<ZwpTabletV2, Data> + Handler,
{
    fn event(
        data: &mut D,
        tablet: &ZwpTabletV2,
        event: zwp_tablet_v2::Event,
        udata: &Data,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let mut guard = udata.info.lock().unwrap();
        match event {
            zwp_tablet_v2::Event::Name { name } => guard.name = Some(name),
            zwp_tablet_v2::Event::Id { vid, pid } => guard.id = Some((vid, pid)),
            zwp_tablet_v2::Event::Path { path } => guard.paths.push(path),
            zwp_tablet_v2::Event::Done => {
                let info = mem::take(&mut *guard);
                drop(guard);
                data.info(conn, qh, tablet, info);
            },
            zwp_tablet_v2::Event::Removed => {
                data.removed(conn, qh, tablet);
            },
            _ => unreachable!(),
        }
    }
}
