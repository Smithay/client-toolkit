use std::{env, path::Path};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_registry, delegate_shm, delegate_simple,
    delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState, SimpleGlobal},
    registry_handlers,
    shell::{
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::{self, WpViewport},
    wp_viewporter::{self, WpViewporter},
};

fn main() {
    env_logger::init();

    // All Wayland apps start by connecting the compositor (server).
    let conn = Connection::connect_to_env().unwrap();

    // Enumerate the list of globals to get the protocols the server implements.
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    // The compositor (not to be confused with the server which is commonly called the compositor) allows
    // configuring surfaces to be presented.
    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    // For desktop platforms, the XDG shell is the standard protocol for creating desktop windows.
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg shell is not available");
    // Since we are not using the GPU in this example, we use wl_shm to allow software rendering to a buffer
    // we share with the compositor process.
    let shm = Shm::bind(&globals, &qh).expect("wl shm is not available.");
    // In this example, we use the viewporter to allow the compositor to scale and crop presented images.
    //
    // Since the wp_viewporter protocol has no events, we can use SimpleGlobal.
    let wp_viewporter = SimpleGlobal::<wp_viewporter::WpViewporter, 1>::bind(&globals, &qh)
        .expect("wp_viewporter not available");

    let mut windows = Vec::new();

    let mut pool_size = 0;

    for path in env::args_os().skip(1) {
        let image = match image::open(&path) {
            Ok(i) => i,
            Err(e) => {
                println!("Failed to open image {}.", path.to_string_lossy());
                println!("Error was: {e:?}");
                return;
            }
        };

        // We'll need the image in RGBA for drawing it
        let image = image.to_rgba8();

        pool_size += image.width() * image.height() * 4;

        // A window is created from a surface.
        let surface = compositor.create_surface(&qh);
        // And then we can create the window.
        let window = xdg_shell.create_window(surface, WindowDecorations::RequestServer, &qh);
        // Configure the window, this may include hints to the compositor about the desired minimum size of the
        // window, app id for WM identification, the window title, etc.
        // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
        window.set_app_id("io.github.smithay.client-toolkit.ImageViewer");
        window.set_min_size(Some((256, 256)));
        let path: &Path = path.as_os_str().as_ref();
        window.set_title(path.components().last().unwrap().as_os_str().to_string_lossy());

        // In order for the window to be mapped, we need to perform an initial commit with no attached buffer.
        // For more info, see WaylandSurface::commit
        //
        // The compositor will respond with an initial configure that we can then use to present to the window with
        // the correct options.
        window.commit();

        // For scaling, create a viewport for the window.
        let viewport = wp_viewporter.get().expect("Requires wp_viewporter").get_viewport(
            window.wl_surface(),
            &qh,
            (),
        );

        windows.push(ImageViewer {
            width: image.width(),
            height: image.height(),
            window,
            viewport,
            image,
            first_configure: true,
            damaged: true,
        });
    }

    if windows.is_empty() {
        println!("USAGE: ./image_viewer <PATH> [<PATH>]...");
        return;
    }

    let pool = SlotPool::new(pool_size as usize, &shm).expect("Failed to create pool");

    let mut state = State {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        shm,
        wp_viewporter,
        pool,
        windows,
    };

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
    shm: Shm,
    wp_viewporter: SimpleGlobal<WpViewporter, 1>,

    pool: SlotPool,
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
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
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

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // Not needed for this example.
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // Not needed for this example.
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

impl WindowHandler for State {
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
            if let (Some(width), Some(height)) = configure.new_size {
                viewer.width = width.get();
                viewer.height = height.get();
                viewer.viewport.set_destination(width.get() as _, height.get() as _);
                if !viewer.first_configure {
                    viewer.window.commit();
                }
            }

            // Initiate the first draw.
            viewer.first_configure = false;
        }
        self.draw(conn, qh);
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
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

            let (buffer, canvas) = self
                .pool
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

    registry_handlers!(OutputState);
}

impl AsMut<SimpleGlobal<WpViewporter, 1>> for State {
    fn as_mut(&mut self) -> &mut SimpleGlobal<WpViewporter, 1> {
        &mut self.wp_viewporter
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
