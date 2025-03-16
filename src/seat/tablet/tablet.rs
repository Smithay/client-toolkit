use std::mem;
use std::sync::Mutex;

use wayland_backend::smallvec::SmallVec;
use wayland_client::{
    Connection,
    Dispatch,
    QueueHandle,
};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_v2::{self, ZwpTabletV2};

use super::TabletState;

#[derive(Debug)]
pub enum TabletEvent {
    /// The descriptive name of the tablet device
    Name {
        name: String,
    },
    /// The USB vendor and product IDs for the tablet device
    Id {
        vid: u32,
        pid: u32,
    },
    /// System-specific device paths for the tablet.
    ///
    /// Path format is unspecified. Clients must figure out what to do with them, if they care.
    Path {
        path: String,
    },
}

pub trait TabletHandler: Sized {
    /// This is fired at the time of the `zwp_tablet_v2.done` event,
    /// and coalesces any `name`, `id` and `path` events that precede it.
    fn init_done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        tablet: &ZwpTabletV2,
        events: TabletEventList,
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

#[doc(hidden)]
#[derive(Debug)]
pub struct TabletData {
    //seat: WlSeat,
    //tablet_seat: ZwpTabletSeatV2,
    inner: Mutex<TabletDataInner>,
}

impl TabletData {
    pub fn new() -> Self {
        Self { inner: Default::default() }
    }
}

// This will typically reach 3 events: name, id, path;
// but it could be as few as zero for an unnamed virtual device,
// and it could be more for something with multiple paths.
pub type TabletEventList = SmallVec<[TabletEvent; 3]>;

#[derive(Debug, Default)]
struct TabletDataInner {
    /// List of pending events.
    pending: TabletEventList,
}

impl<D> Dispatch<ZwpTabletV2, TabletData, D>
    for TabletState
where
    D: Dispatch<ZwpTabletV2, TabletData> + TabletHandler,
{
    fn event(
        data: &mut D,
        tablet: &ZwpTabletV2,
        event: zwp_tablet_v2::Event,
        udata: &TabletData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let mut guard = udata.inner.lock().unwrap();
        match event {
            zwp_tablet_v2::Event::Name { name } => guard.pending.push(TabletEvent::Name { name }),
            zwp_tablet_v2::Event::Id { vid, pid } => guard.pending.push(TabletEvent::Id { vid, pid }),
            zwp_tablet_v2::Event::Path { path } => guard.pending.push(TabletEvent::Path { path }),
            zwp_tablet_v2::Event::Done => {
                let pending = mem::take(&mut guard.pending);
                drop(guard);
                data.init_done(conn, qh, tablet, pending);
            },
            zwp_tablet_v2::Event::Removed => {
                data.removed(conn, qh, tablet);
            },
            _ => unreachable!(),
        }
    }
}
