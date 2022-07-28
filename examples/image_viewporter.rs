use std::env;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_registry, delegate_shm, delegate_simple,
    delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState, SimpleGlobal},
    registry_handlers,
    shell::xdg::{
        window::{Window, WindowConfigure, WindowHandler, XdgWindowState},
        XdgShellHandler, XdgShellState,
    },
    shm::{slot::SlotPool, ShmHandler, ShmState},
};
use wayland_client::{
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::{self, WpViewport},
    wp_viewporter::WpViewporter,
};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut state = State {
        registry_state: RegistryState::new(&conn, &qh),
        output_state: OutputState::new(),
        compositor_state: CompositorState::new(),
        shm_state: ShmState::new(),
        xdg_shell_state: XdgShellState::new(),
        xdg_window_state: XdgWindowState::new(),
        viewporter: SimpleGlobal::new(),

        pool: None,
        windows: Vec::new(),
    };

    while !state.registry_state.ready() {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }

    let mut pool_size = 0;

    for path in env::args_os().skip(1) {
        let image = match image::open(&path) {
            Ok(i) => i,
            Err(e) => {
                println!("Failed to open image {}.", path.to_string_lossy());
                println!("Error was: {:?}", e);
                return;
            }
        };

        // We'll need the image in RGBA for drawing it
        let image = image.to_rgba8();

        let surface = state.compositor_state.create_surface(&qh).unwrap();

        pool_size += image.width() * image.height() * 4;

        let window = Window::builder()
            .title("A wayland window")
            // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
            .app_id("io.github.smithay.client-toolkit.ImageViewer")
            .map(&qh, &state.xdg_shell_state, &mut state.xdg_window_state, surface)
            .expect("window creation");

        let viewport = state.viewporter.get().expect("Requires wp_viewporter").get_viewport(
            window.wl_surface(),
            &qh,
            (),
        );

        state.windows.push(ImageViewer {
            width: image.width(),
            height: image.height(),
            window,
            viewport,
            image,
            first_configure: true,
            damaged: true,
        });
    }

    let pool = SlotPool::new(pool_size as usize, &state.shm_state).expect("Failed to create pool");
    state.pool = Some(pool);

    if state.windows.is_empty() {
        println!("USAGE: ./image_viewer <PATH> [<PATH>]...");
        return;
    }

    // We don't draw immediately, the configure will notify us when to first draw.

    loop {
        event_queue.blocking_dispatch(&mut state).unwrap();

        if state.windows.is_empty() {
            println!("exiting example");
            break;
        }
    }
}

struct State {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: ShmState,
    xdg_shell_state: XdgShellState,
    xdg_window_state: XdgWindowState,
    viewporter: SimpleGlobal<WpViewporter, 1>,

    pool: Option<SlotPool>,
    windows: Vec<ImageViewer>,
}

struct ImageViewer {
    window: Window,
    image: image::RgbaImage,
    viewport: WpViewport,
    width: u32,
    height: u32,
    first_configure: bool,
    damaged: bool,
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(conn, qh);
    }
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }
}

impl WindowHandler for State {
    fn xdg_window_state(&mut self) -> &mut XdgWindowState {
        &mut self.xdg_window_state
    }

    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        self.windows.retain(|v| v.window != *window);
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        for viewer in &mut self.windows {
            if viewer.window != *window {
                continue;
            }
            if let Some(size) = configure.new_size {
                viewer.width = size.0;
                viewer.height = size.1;
                viewer.viewport.set_destination(size.0 as _, size.1 as _);
                if !viewer.first_configure {
                    viewer.window.wl_surface().commit();
                }
            }

            // Initiate the first draw.
            viewer.first_configure = false;
        }
        self.draw(conn, qh);
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
    }
}

impl State {
    pub fn draw(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {
        for viewer in &mut self.windows {
            if viewer.first_configure || !viewer.damaged {
                continue;
            }
            let window = &viewer.window;
            let width = viewer.image.width();
            let height = viewer.image.height();
            let stride = width as i32 * 4;
            let pool = self.pool.as_mut().unwrap();

            let (buffer, canvas) = pool
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
                .expect("create buffer");

            // Draw to the window
            for (pixel, argb) in viewer.image.pixels().zip(canvas.chunks_exact_mut(4)) {
                // We do this in an horribly inefficient manner, for the sake of simplicity.
                // We'll send pixels to the server in ARGB8888 format (this is one of the only
                // formats that are guaranteed to be supported), but image provides it in
                // big-endian RGBA8888, so we need to do the conversion.
                argb[3] = pixel.0[3];
                argb[2] = pixel.0[0];
                argb[1] = pixel.0[1];
                argb[0] = pixel.0[2];
            }

            // Damage the entire window (using the real dimensions)
            window.wl_surface().damage_buffer(0, 0, viewer.width as i32, viewer.height as i32);
            viewer.damaged = false;

            // Set the entire buffer as the source area for the viewport.
            // Destination was set during configure.
            viewer.viewport.set_source(0.0, 0.0, viewer.width as f64, viewer.height as f64);

            // Attach and commit to present.
            buffer.attach_to(window.wl_surface()).expect("buffer attach");
            window.wl_surface().commit();
        }
    }
}

delegate_compositor!(State);
delegate_output!(State);
delegate_shm!(State);

delegate_xdg_shell!(State);
delegate_xdg_window!(State);

delegate_simple!(State, WpViewporter, 1);

delegate_registry!(State);

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(CompositorState, OutputState, ShmState, XdgShellState, XdgWindowState, SimpleGlobal<WpViewporter, 1>);
}

impl AsMut<SimpleGlobal<WpViewporter, 1>> for State {
    fn as_mut(&mut self) -> &mut SimpleGlobal<WpViewporter, 1> {
        &mut self.viewporter
    }
}

impl Dispatch<WpViewport, ()> for State {
    fn event(
        _: &mut State,
        _: &WpViewport,
        _: wp_viewport::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        unreachable!("wp_viewport::Event is empty in version 1")
    }
}

impl Drop for ImageViewer {
    fn drop(&mut self) {
        self.viewport.destroy()
    }
}
