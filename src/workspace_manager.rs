use crate::{dispatch2::Dispatch2, globals::GlobalData, registry::GlobalProxy};
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};
use wayland_client::{globals::GlobalList, Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1,
    ext_workspace_handle_v1::{self, State, WorkspaceCapabilities},
    ext_workspace_manager_v1,
};

/// Information about a workspace.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct WorkspaceInfo {
    // ID
    pub id: String,
    // Name
    pub name: String,
    // State
    pub state: Option<WEnum<State>>,
    // Coordinates
    pub coordinates: Vec<u8>,
    // Capabilities
    pub capabilities: Option<WEnum<WorkspaceCapabilities>>,
}

#[derive(Debug, Default)]
struct WorkspaceInner {
    current_info: Option<WorkspaceInfo>,
    pending_info: WorkspaceInfo,
}

#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct WorkspaceData(Arc<Mutex<WorkspaceInner>>);

#[derive(Debug)]
pub struct WorkspaceManager {
    workspace_manager: GlobalProxy<ext_workspace_manager_v1::ExtWorkspaceManagerV1>,
    workspaces: Vec<ext_workspace_handle_v1::ExtWorkspaceHandleV1>,
}

impl WorkspaceManager {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ext_workspace_manager_v1::ExtWorkspaceManagerV1, GlobalData> + 'static,
    {
        let workspace_manager = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { workspace_manager, workspaces: Vec::new() }
    }

    /// Returns list of worksapces.
    pub fn workspaces(&self) -> &[ext_workspace_handle_v1::ExtWorkspaceHandleV1] {
        &self.workspaces
    }

    /// Returns information about a workspace.
    ///
    /// This may be none if the workspace has been destroyed or the compositor has not sent
    /// information about the workspace yet.
    pub fn info(
        &self,
        workspace: &ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    ) -> Option<WorkspaceInfo> {
        workspace.data::<WorkspaceData>()?.0.lock().unwrap().current_info.clone()
    }

    pub fn stop(&self) {
        if let Ok(workspace_manager) = self.workspace_manager.get() {
            workspace_manager.stop();
        }
    }
}

/// Handler trait for ext workspaces protocol.
pub trait WorkspaceHandler: Sized {
    fn ext_workspace_state(&mut self) -> &mut WorkspaceManager;

    /// A new workspace has been created.
    fn new_workspace(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        workspace_handle: ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    );

    /// An existing workspace has changed.
    fn update_workspace(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        workspace_handle: ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    );

    /// A workspace has been removed.
    fn workspace_removed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        workspace_handle: ext_workspace_handle_v1::ExtWorkspaceHandleV1,
    );

    /// All workspaces/groups have been updated.
    fn done(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {}

    fn finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {}
}

// Workspace group
impl<D> Dispatch2<ext_workspace_manager_v1::ExtWorkspaceManagerV1, D> for GlobalData
where
    D: Dispatch<ext_workspace_handle_v1::ExtWorkspaceHandleV1, WorkspaceData>
        + Dispatch<ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, WorkspaceData>
        + WorkspaceHandler
        + 'static,
{
    fn event(
        &self,
        state: &mut D,
        proxy: &ext_workspace_manager_v1::ExtWorkspaceManagerV1,
        event: ext_workspace_manager_v1::Event,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group: _ } => {}
            ext_workspace_manager_v1::Event::Workspace { workspace } => {
                state.ext_workspace_state().workspaces.push(workspace.clone());
            }
            ext_workspace_manager_v1::Event::Done => {
                // TODO: is cloning really the best for performance?
                for ref workspace in state.ext_workspace_state().workspaces.clone() {
                    let handle = workspace.data::<WorkspaceData>().unwrap();
                    let mut inner = handle.0.lock().unwrap();
                    let just_created = inner.current_info.is_none();
                    inner.current_info = Some(inner.pending_info.clone());
                    drop(inner);
                    if just_created {
                        state.new_workspace(conn, qh, workspace.clone());
                    } else {
                        state.update_workspace(conn, qh, workspace.clone());
                    }
                }
                state.done(conn, qh);
            }
            ext_workspace_manager_v1::Event::Finished => {
                state.finished(conn, qh);
                proxy.stop();
            }
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, ext_workspace_manager_v1::ExtWorkspaceManagerV1, [
        ext_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, Default::default()),
        ext_workspace_manager_v1::EVT_WORKSPACE_OPCODE => (ext_workspace_handle_v1::ExtWorkspaceHandleV1, Default::default()),
    ]);
}

impl<D> Dispatch2<ext_workspace_handle_v1::ExtWorkspaceHandleV1, D> for WorkspaceData
where
    D: WorkspaceHandler,
{
    fn event(
        &self,
        state: &mut D,
        handle: &ext_workspace_handle_v1::ExtWorkspaceHandleV1,
        event: ext_workspace_handle_v1::Event,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_workspace_handle_v1::Event::Removed => {
                state.workspace_removed(conn, qh, handle.clone());
                let workspaces = &mut state.ext_workspace_state().workspaces;
                if let Some(idx) = workspaces.iter().position(|x| x == handle) {
                    workspaces.remove(idx);
                }
                handle.destroy();
            }
            ext_workspace_handle_v1::Event::Id { id } => {
                self.0.lock().unwrap().pending_info.id = id;
            }
            ext_workspace_handle_v1::Event::Name { name } => {
                self.0.lock().unwrap().pending_info.name = name;
            }
            ext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                self.0.lock().unwrap().pending_info.coordinates = coordinates;
            }
            ext_workspace_handle_v1::Event::State { state } => {
                self.0.lock().unwrap().pending_info.state = Some(state);
            }
            ext_workspace_handle_v1::Event::Capabilities { capabilities } => {
                self.0.lock().unwrap().pending_info.capabilities = Some(capabilities);
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch2<ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, D> for WorkspaceData
where
    D: WorkspaceHandler,
{
    fn event(
        &self,
        state: &mut D,
        handle: &ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
        event: ext_workspace_group_handle_v1::Event,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            _ => {
                println!("workspace group happened")
            }
        }
    }
}
