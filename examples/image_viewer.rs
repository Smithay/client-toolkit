extern crate image;

use std::env;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_registry, delegate_shm, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    shell::xdg::{
        window::{Window, WindowConfigure, WindowHandler, XdgWindowState},
        XdgShellHandler, XdgShellState,
    },
    shm::{
        pool::slot::{Buffer, SlotPool},
        ShmHandler, ShmState,
    },
};
use wayland_client::{
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&qh, ()).unwrap();

    let mut state = State {
        registry_state: RegistryState::new(registry),
        output_state: OutputState::new(),
        compositor_state: CompositorState::new(),
        shm_state: ShmState::new(),
        xdg_shell_state: XdgShellState::new(),
        xdg_window_state: XdgWindowState::new(),

        pool: None,
        windows: Vec::new(),
    };

    event_queue.blocking_dispatch(&mut state).unwrap();
    event_queue.blocking_dispatch(&mut state).unwrap();

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

        state.windows.push(ImageViewer {
            width: image.width(),
            height: image.height(),
            window,
            image,
            first_configure: true,
            damaged: true,
            buffer: None,
        });
    }

    let pool = state.shm_state.new_slot_pool(pool_size as usize).expect("Failed to create pool");
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

    pool: Option<SlotPool>,
    windows: Vec<ImageViewer>,
}

struct ImageViewer {
    window: Window,
    image: image::RgbaImage,
    width: u32,
    height: u32,
    buffer: Option<Buffer>,
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
                viewer.buffer = None;
                viewer.damaged = true;
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
    pub fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        for viewer in &mut self.windows {
            if viewer.first_configure || !viewer.damaged {
                continue;
            }
            let window = &viewer.window;
            let width = viewer.width;
            let height = viewer.height;
            let stride = viewer.width as i32 * 4;
            let pool = self.pool.as_mut().unwrap();

            let buffer = viewer.buffer.get_or_insert_with(|| {
                pool.create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
                    .expect("create buffer")
                    .0
            });

            let canvas = match pool.canvas(buffer) {
                Some(canvas) => canvas,
                None => {
                    // This should be rare, but if the compositor has not released the previous
                    // buffer, we need double-buffering.
                    let (second_buffer, canvas) = pool
                        .create_buffer(
                            viewer.width as i32,
                            viewer.height as i32,
                            stride,
                            wl_shm::Format::Argb8888,
                        )
                        .expect("create buffer");
                    *buffer = second_buffer;
                    canvas
                }
            };

            // Draw to the window:
            {
                let image = image::imageops::resize(
                    &viewer.image,
                    viewer.width,
                    viewer.height,
                    image::imageops::FilterType::Nearest,
                );

                for (pixel, argb) in image.pixels().zip(canvas.chunks_exact_mut(4)) {
                    // We do this in an horribly inefficient manner, for the sake of simplicity.
                    // We'll send pixels to the server in ARGB8888 format (this is one of the only
                    // formats that are guaranteed to be supported), but image provides it in
                    // big-endian RGBA8888, so we need to do the conversion.
                    argb[3] = pixel.0[3];
                    argb[2] = pixel.0[0];
                    argb[1] = pixel.0[1];
                    argb[0] = pixel.0[2];
                }
            }

            // Damage the entire window
            window.wl_surface().damage_buffer(0, 0, viewer.width as i32, viewer.height as i32);
            viewer.damaged = false;

            // Request our next frame
            window.wl_surface().frame(qh, window.wl_surface().clone()).expect("create callback");

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

delegate_registry!(State: [
    CompositorState,
    OutputState,
    ShmState,
    XdgShellState,
    XdgWindowState,
]);

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
}
