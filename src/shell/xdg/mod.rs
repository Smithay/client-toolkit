//! ## Cross desktop group (XDG) shell
// TODO: Examples

use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use wayland_client::{ConnectionHandle, QueueHandle};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1,
    xdg_shell::client::{xdg_toplevel::State, xdg_wm_base},
};

use self::{
    inner::XdgSurfaceDataInner,
    window::{inner::WindowInner, Window},
};

pub mod popup;
pub mod window;

#[derive(Debug)]
pub struct XdgShellState {
    // (name, global)
    xdg_wm_base: Option<(u32, xdg_wm_base::XdgWmBase)>,
    zxdg_decoration_manager_v1: Option<(u32, zxdg_decoration_manager_v1::ZxdgDecorationManagerV1)>,

    windows: Vec<Weak<WindowInner>>,
}

impl XdgShellState {
    pub fn new() -> XdgShellState {
        XdgShellState { xdg_wm_base: None, zxdg_decoration_manager_v1: None, windows: vec![] }
    }
}

pub trait XdgShellHandler<D> {
    fn configure_window(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        new_size: Option<(u32, u32)>,
        states: Vec<State>,
        state: &mut XdgShellState,
        window: &Window,
    );

    fn request_close_window(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &mut XdgShellState,
        window: &Window,
    );
}

/// Data associated with an XDG surface created by Smithay's client toolkit.
#[derive(Debug, Clone)]
pub struct XdgSurfaceData(Arc<Mutex<XdgSurfaceDataInner>>);

#[derive(Debug)]
pub struct XdgShellDispatch<'s, D, H: XdgShellHandler<D>>(
    pub &'s mut XdgShellState,
    pub &'s mut H,
    pub PhantomData<D>,
);

mod inner;
