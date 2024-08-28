use crate::{globals::GlobalData, registry::GlobalProxy};
use std::sync::{Arc, Mutex};
use wayland_client::{globals::GlobalList, Connection, Dispatch, Proxy, QueueHandle};
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

#[derive(Debug, Default)]
struct ForeignToplevelInner {
    current_info: Option<ForeignToplevelInfo>,
    pending_info: ForeignToplevelInfo,
}

#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct ForeignToplevelData(Arc<Mutex<ForeignToplevelInner>>);

#[derive(Debug)]
pub struct ForeignToplevelList {
    foreign_toplevel_list: GlobalProxy<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1>,
    toplevels: Vec<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1>,
}

impl ForeignToplevelList {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, GlobalData> + 'static,
    {
        let foreign_toplevel_list = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { foreign_toplevel_list, toplevels: Vec::new() }
    }

    /// Returns list of toplevels.
    pub fn toplevels(&self) -> &[ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1] {
        &self.toplevels
    }

    /// Returns information about a toplevel.
    ///
    /// This may be none if the toplevel has been destroyed or the compositor has not sent
    /// information about the toplevel yet.
    pub fn info(
        &self,
        toplevel: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ) -> Option<ForeignToplevelInfo> {
        toplevel.data::<ForeignToplevelData>()?.0.lock().unwrap().current_info.clone()
    }

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

    fn finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {}
}

impl<D> Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, GlobalData, D>
    for ForeignToplevelList
where
    D: Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, GlobalData>
        + Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ForeignToplevelData>
        + ForeignToplevelListHandler
        + 'static,
{
    fn event(
        state: &mut D,
        proxy: &ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &GlobalData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel: _ } => {}
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

impl<D> Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ForeignToplevelData, D>
    for ForeignToplevelList
where
    D: Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ForeignToplevelData>
        + ForeignToplevelListHandler,
{
    fn event(
        state: &mut D,
        handle: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        data: &ForeignToplevelData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                state.toplevel_closed(conn, qh, handle.clone());
                let toplevels = &mut state.foreign_toplevel_list_state().toplevels;
                if let Some(idx) = toplevels.iter().position(|x| x == handle) {
                    toplevels.remove(idx);
                }
                handle.destroy();
            }
            ext_foreign_toplevel_handle_v1::Event::Done => {
                let mut inner = data.0.lock().unwrap();
                let just_created = inner.current_info.is_none();
                inner.current_info = Some(inner.pending_info.clone());
                drop(inner);
                if just_created {
                    state.foreign_toplevel_list_state().toplevels.push(handle.clone());
                    state.new_toplevel(conn, qh, handle.clone());
                } else {
                    state.update_toplevel(conn, qh, handle.clone());
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                data.0.lock().unwrap().pending_info.title = title;
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                data.0.lock().unwrap().pending_info.app_id = app_id;
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                data.0.lock().unwrap().pending_info.identifier = identifier;
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_foreign_toplevel_list {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1: $crate::globals::GlobalData
            ] => $crate::foreign_toplevel_list::ForeignToplevelList
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1: $crate::foreign_toplevel_list::ForeignToplevelData
            ] => $crate::foreign_toplevel_list::ForeignToplevelList
        );
    };
}
