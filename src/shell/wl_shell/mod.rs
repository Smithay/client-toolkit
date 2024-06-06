use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::{wl_shell, wl_shell_surface, wl_surface::WlSurface},
    Connection, Dispatch, QueueHandle
};

use crate::{
    error::GlobalError, globals::{GlobalData, ProvidesBoundGlobal}
};

pub mod window;

use window::Window;


#[derive(Debug)]
pub struct WlShell {
    wl_shell: wl_shell::WlShell,
}

impl WlShell {
    pub fn bind<State>(globals: &GlobalList, qh: &QueueHandle<State>) -> Result<WlShell, BindError>
    where
        State: Dispatch<wl_shell::WlShell, GlobalData, State>  + 'static,
    {
        let wl_shell = globals.bind(qh, 1..=1, GlobalData)?;
        
        Ok(WlShell { wl_shell})
    }

    pub fn create_window<State>(&self, surface: WlSurface, qh: &QueueHandle<State>) -> Window 
    where
        State: Dispatch<wl_shell_surface::WlShellSurface, GlobalData, State> + 'static
    {
        let wl_shell_surface = self.wl_shell.get_shell_surface(&surface, qh, GlobalData);

        Window::new(surface, wl_shell_surface)
    }

    pub fn wl_shell(&self) -> &wl_shell::WlShell {
        &self.wl_shell
    }
}


impl ProvidesBoundGlobal<wl_shell::WlShell, 1> for WlShell {
    fn bound_global(&self) -> Result<wl_shell::WlShell, GlobalError> {
        Ok(self.wl_shell.clone())
    }
}

impl<D> Dispatch<wl_shell_surface::WlShellSurface, GlobalData, D> for WlShell
where
    D: Dispatch<wl_shell_surface::WlShellSurface, GlobalData>{
    fn event(
        _state: &mut D,
        proxy: &wl_shell_surface::WlShellSurface,
        event: wl_shell_surface::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qhandle: &QueueHandle<D>,
    ) {
        match event {
            wl_shell_surface::Event::Ping { serial } => {
                proxy.pong(serial);
            },
            _ => unreachable!(),
        }
    }
}
