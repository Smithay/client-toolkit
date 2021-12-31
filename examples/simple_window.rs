use std::{convert::TryInto, marker::PhantomData};

use smithay_client_toolkit::{
    compositor::{CompositorState, SurfaceData, SurfaceDispatch, SurfaceHandler},
    delegate_output, delegate_registry, delegate_shm,
    output::{OutputDispatch, OutputHandler, OutputState},
    registry::RegistryHandle,
    shell::window::{
        DecorationMode, ShellHandler, Window, WindowData, XdgShellDispatch, XdgShellState,
        XdgSurfaceData,
    },
    shm::{pool::raw::RawPool, ShmState},
};
use wayland_client::{
    delegate_dispatch,
    protocol::{wl_buffer, wl_callback, wl_compositor, wl_output, wl_shm, wl_surface},
    Connection, ConnectionHandle, Dispatch, QueueHandle,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::{
        zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
    },
    xdg_shell::client::{
        xdg_surface,
        xdg_toplevel::{self, State},
        xdg_wm_base,
    },
};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let display = conn.handle().display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&mut conn.handle(), &qh, ()).unwrap();

    let mut simple_window = SimpleWindow {
        inner: InnerApp {
            exit: false,
            first_configure: true,
            pool: None,
            width: 256,
            height: 256,
            buffer: None,
            window: None,
        },

        registry_handle: RegistryHandle::new(registry),
        output_state: OutputState::new(),
        compositor_state: CompositorState::new(),
        shm_state: ShmState::new(),
        xdg_shell: XdgShellState::new(),
    };

    event_queue.blocking_dispatch(&mut simple_window).unwrap();
    event_queue.blocking_dispatch(&mut simple_window).unwrap();

    let pool = simple_window
        .shm_state
        .new_raw_pool(
            simple_window.inner.width as usize * simple_window.inner.height as usize * 4,
            &mut conn.handle(),
            &qh,
            (),
        )
        .expect("Failed to create pool");
    simple_window.inner.pool = Some(pool);

    let surface = simple_window.compositor_state.create_surface(&mut conn.handle(), &qh).unwrap();

    let window = simple_window
        .xdg_shell
        .create_window(&mut conn.handle(), &qh, surface.clone(), DecorationMode::ServerDecides)
        .expect("window");

    window.set_title(&mut conn.handle(), "A wayland window");
    // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
    window.set_app_id(&mut conn.handle(), "io.github.smithay.client-toolkit.SimpleWindow");
    window.set_min_size(&mut conn.handle(), (256, 256));

    // Map the window so we receive the initial configure and can render.
    window.map(&mut conn.handle());

    simple_window.inner.window = Some(window);

    // We don't draw immediately, the configure will indicate to us when to first draw.

    loop {
        event_queue.blocking_dispatch(&mut simple_window).unwrap();

        if simple_window.inner.exit {
            println!("exiting example");
            break;
        }
    }
}

struct SimpleWindow {
    inner: InnerApp,
    registry_handle: RegistryHandle,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: ShmState,
    xdg_shell: XdgShellState,
}

struct InnerApp {
    exit: bool,
    first_configure: bool,
    pool: Option<RawPool>,
    width: u32,
    height: u32,
    buffer: Option<wl_buffer::WlBuffer>,
    window: Option<Window>,
}

impl SurfaceHandler<SimpleWindow> for InnerApp {
    fn scale_factor_changed(
        &mut self,
        _: &mut CompositorState,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<SimpleWindow>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // TODO
    }

    fn frame(
        &mut self,
        _: &mut CompositorState,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<SimpleWindow>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(conn, qh);
    }
}

impl OutputHandler<SimpleWindow> for InnerApp {
    fn new_output(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<SimpleWindow>,
        _state: &OutputState,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<SimpleWindow>,
        _state: &OutputState,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<SimpleWindow>,
        _state: &OutputState,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ShellHandler<SimpleWindow> for InnerApp {
    fn request_close(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<SimpleWindow>,
        _: &Window,
    ) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<SimpleWindow>,
        size: (u32, u32),
        _: Vec<State>, // We don't particularly care for the states at the moment.
        _: &Window,
    ) {
        if size == (0, 0) {
            self.width = 256;
            self.height = 256;
        } else {
            self.width = size.0;
            self.height = size.1;
        }

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh);
        }
    }
}

impl InnerApp {
    pub fn draw(&mut self, conn: &mut ConnectionHandle, qh: &QueueHandle<SimpleWindow>) {
        if let Some(window) = self.window.as_ref() {
            // Ensure the pool is big enough to hold the new buffer.
            self.pool
                .as_mut()
                .unwrap()
                .resize((self.width * self.height * 4) as usize, conn)
                .expect("resize pool");

            // Destroy the old buffer.
            // FIXME: Integrate this into the pool logic.
            self.buffer.take().map(|buffer| {
                buffer.destroy(conn);
            });

            let offset = 0;
            let stride = self.width as i32 * 4;
            let pool = self.pool.as_mut().unwrap();

            let wl_buffer = pool
                .create_buffer(
                    offset,
                    self.width as i32,
                    self.height as i32,
                    stride,
                    wl_shm::Format::Argb8888,
                    (),
                    conn,
                    &qh,
                )
                .expect("create buffer");

            // TODO: Upgrade to a better pool type
            let len = self.height as usize * stride as usize; // length of a row
            let buffer = &mut pool.mmap()[offset as usize..][..len];

            // Draw to the window:
            {
                let width = self.width;
                let height = self.height;

                buffer.chunks_exact_mut(4).enumerate().for_each(|(index, chunk)| {
                    let x = (index % width as usize) as u32;
                    let y = (index / width as usize) as u32;

                    let a = 0xFF;
                    let r = u32::min(((width - x) * 0xFF) / width, ((height - y) * 0xFF) / height);
                    let g = u32::min((x * 0xFF) / width, ((height - y) * 0xFF) / height);
                    let b = u32::min(((width - x) * 0xFF) / width, (y * 0xFF) / height);
                    let color = (a << 24) + (r << 16) + (g << 8) + b;

                    let array: &mut [u8; 4] = chunk.try_into().unwrap();
                    *array = color.to_le_bytes();
                });
            }

            self.buffer = Some(wl_buffer.clone());

            // Request our next frame
            window
                .wl_surface()
                .frame(conn, qh, window.wl_surface().clone())
                .expect("create callback");

            assert!(self.buffer.is_some(), "No buffer?");
            // Attach and commit to present.
            window.wl_surface().attach(conn, self.buffer.clone(), 0, 0);
            window.wl_surface().commit(conn);
        }
    }
}

delegate_dispatch!(SimpleWindow: <UserData = ()> [wl_compositor::WlCompositor] => SurfaceDispatch<'_, SimpleWindow, InnerApp> ; |app| {
    &mut SurfaceDispatch(&mut app.compositor_state, &mut app.inner, PhantomData)
});

delegate_dispatch!(SimpleWindow: <UserData = SurfaceData> [wl_surface::WlSurface] => SurfaceDispatch<'_, SimpleWindow, InnerApp> ; |app| {
    &mut SurfaceDispatch(&mut app.compositor_state, &mut app.inner, PhantomData)
});

delegate_dispatch!(SimpleWindow: <UserData = wl_surface::WlSurface> [wl_callback::WlCallback] => SurfaceDispatch<'_, SimpleWindow, InnerApp> ; |app| {
    &mut SurfaceDispatch(&mut app.compositor_state, &mut app.inner, PhantomData)
});

delegate_dispatch!(SimpleWindow: <UserData = ()>
[
    xdg_wm_base::XdgWmBase,
    zxdg_decoration_manager_v1::ZxdgDecorationManagerV1
] => XdgShellDispatch<'_, SimpleWindow, InnerApp> ; |app| {
    &mut XdgShellDispatch(&mut app.xdg_shell, &mut app.inner, PhantomData)
});

delegate_dispatch!(SimpleWindow: <UserData = XdgSurfaceData> [xdg_surface::XdgSurface] => XdgShellDispatch<'_, SimpleWindow, InnerApp> ; |app| {
    &mut XdgShellDispatch(&mut app.xdg_shell, &mut app.inner, PhantomData)
});

delegate_dispatch!(SimpleWindow: <UserData = WindowData> [xdg_toplevel::XdgToplevel, zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1] => XdgShellDispatch<'_, SimpleWindow, InnerApp> ; |app| {
    &mut XdgShellDispatch(&mut app.xdg_shell, &mut app.inner, PhantomData)
});

delegate_output!(SimpleWindow => InnerApp: |app| {
    &mut OutputDispatch(&mut app.output_state, &mut app.inner, PhantomData)
});

delegate_shm!(SimpleWindow: |app| {
    &mut app.shm_state
});

delegate_registry!(SimpleWindow:
    |app| {
        &mut app.registry_handle
    },
    handlers = [
        { &mut app.xdg_shell },
        { &mut app.shm_state },
        { &mut app.compositor_state },
        { &mut OutputDispatch(&mut app.output_state, &mut app.inner, PhantomData) }
    ]
);

// impl Dispatch<wl_callback::WlCallback> for SimpleWindow {
//     type UserData = ();

//     fn event(
//         &mut self,
//         _: &wl_callback::WlCallback,
//         event: wl_callback::Event,
//         _: &Self::UserData,
//         conn: &mut ConnectionHandle,
//         qh: &QueueHandle<Self>,
//     ) {
//         if let wl_callback::Event::Done { .. } = event {
//             self.inner.draw(conn, qh);
//         }
//     }
// }

// TODO
impl Dispatch<wl_buffer::WlBuffer> for SimpleWindow {
    type UserData = ();

    fn event(
        &mut self,
        _: &wl_buffer::WlBuffer,
        _: wl_buffer::Event,
        _: &Self::UserData,
        _: &mut wayland_client::ConnectionHandle,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        // todo
    }
}
