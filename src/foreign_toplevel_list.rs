//! Foreign Toplevel List

use crate::{dispatch2::Dispatch2, globals::GlobalData, registry::GlobalProxy};
use log::warn;
use std::sync::{Arc, Mutex};
use wayland_client::{globals::GlobalList, Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1, ext_foreign_toplevel_list_v1,
};

/// Information about a toplevel.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ForeignToplevelInfo {
    /// Title
    pub title: String,
    /// App id
    pub app_id: String,
    /// Identifier to check if two toplevel handles refer to same toplevel
    pub identifier: String,
}

#[derive(Debug)]
struct ForeignToplevelInner {
    toplevel_handle: ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,

    current_info: Option<ForeignToplevelInfo>,
    pending_info: ForeignToplevelInfo,
}

/// Data associated with a [`ExtForeignToplevelHandleV1`]
///
/// [`ExtForeignToplevelHandleV1`]: ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1
#[derive(Debug, Default, Clone)]
pub struct ForeignToplevelData(Arc<Mutex<ForeignToplevelInfo>>);

impl ForeignToplevelData {
    /// Access the underlying [`ForeignToplevelInfo`]
    ///
    /// Reentrant calls withing the `callback` will deadlock.
    pub fn with_info<F, T>(&self, callback: F) -> T
    where
        F: FnOnce(&ForeignToplevelInfo) -> T,
    {
        let guard = self.0.lock().unwrap();
        callback(&guard)
    }
}

/// A handler for delegating [`ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1`].
#[derive(Debug)]
pub struct ForeignToplevelList {
    foreign_toplevel_list: GlobalProxy<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1>,
    toplevels: Vec<ForeignToplevelInner>,
}

impl ForeignToplevelList {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, GlobalData> + 'static,
    {
        let foreign_toplevel_list = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { foreign_toplevel_list, toplevels: Vec::new() }
    }

    /// Returns an iterator over all foreign toplevels.
    pub fn toplevels(
        &self,
    ) -> impl Iterator<Item = ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1> {
        self.toplevels
            .iter()
            .map(|toplevel| &toplevel.toplevel_handle)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Returns information about a toplevel.
    ///
    /// This may be `None` if the toplevel has been destroyed or the compositor has not sent
    /// information about the toplevel yet.
    pub fn info(
        &self,
        toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) -> Option<ForeignToplevelInfo> {
        self.toplevels
            .iter()
            .find(|inner| &inner.toplevel_handle == toplevel)
            .and_then(|inner| inner.current_info.clone())
    }

    /// Notify the compositor that the client no longer wishes to receive events for new toplevels.
    pub fn stop(&self) {
        if let Ok(toplevel_list) = self.foreign_toplevel_list.get() {
            toplevel_list.stop();
        }
    }
}

/// Handler trait for foreign toplevel list protocol.
pub trait ForeignToplevelListHandler: Sized {
    fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelList;

    /// A new toplevel has been opened.
    fn new_toplevel(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel_handle: ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    );

    /// An existing toplevel has changed.
    fn update_toplevel(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel_handle: ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    );

    /// A toplevel has closed.
    fn toplevel_closed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        toplevel_handle: ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    );

    /// The compositor is done sending events.
    ///
    /// This event notifies that the compositor will no longer send events.
    fn finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {}
}

impl<D> Dispatch2<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, D> for GlobalData
where
    D: Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ForeignToplevelData>
        + ForeignToplevelListHandler
        + 'static,
{
    fn event(
        &self,
        state: &mut D,
        proxy: &ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } => {
                let inner = ForeignToplevelInner {
                    toplevel_handle: toplevel,
                    current_info: None,
                    pending_info: Default::default(),
                };

                state.foreign_toplevel_list_state().toplevels.push(inner);
            }
            ext_foreign_toplevel_list_v1::Event::Finished => {
                state.finished(conn, qh);
                proxy.destroy();
            }
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, Default::default())
    ]);
}

impl<D> Dispatch2<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, D>
    for ForeignToplevelData
where
    D: ForeignToplevelListHandler,
{
    fn event(
        &self,
        state: &mut D,
        handle: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let inner = match state
            .foreign_toplevel_list_state()
            .toplevels
            .iter_mut()
            .find(|inner| &inner.toplevel_handle == handle)
        {
            Some(inner) => inner,
            None => {
                warn!("Received {event:?} for dead ext_foreign_toplevel_handle");
                return;
            }
        };

        match event {
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                state.toplevel_closed(conn, qh, handle.clone());
                let toplevels = &mut state.foreign_toplevel_list_state().toplevels;
                if let Some(idx) = toplevels.iter().position(|x| &x.toplevel_handle == handle) {
                    toplevels.remove(idx);
                }
                handle.destroy();
            }
            ext_foreign_toplevel_handle_v1::Event::Done => {
                let just_created = inner.current_info.is_none();
                inner.current_info = Some(inner.pending_info.clone());

                self.set(inner.pending_info.clone());

                if just_created {
                    state.new_toplevel(conn, qh, handle.clone());
                } else {
                    state.update_toplevel(conn, qh, handle.clone());
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                inner.pending_info.title = title;
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                inner.pending_info.app_id = app_id;
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                inner.pending_info.identifier = identifier;
            }
            _ => unreachable!(),
        }
    }
}

impl ForeignToplevelData {
    pub(crate) fn set(&self, info: ForeignToplevelInfo) {
        let mut guard = self.0.lock().unwrap();
        *guard = info;
    }
}
