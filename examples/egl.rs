use std::{ffi::CString, num::NonZeroU32};

use glow::HasContext;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_registry, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::xdg::{
        window::{Window, WindowConfigure, WindowHandler, XdgWindowState},
        XdgShellHandler, XdgShellState,
    },
};
use wayland_client::{
    protocol::{wl_output, wl_surface},
    Connection, Proxy, QueueHandle,
};

use glutin::{api::egl, config::ConfigSurfaceTypes, prelude::*, surface::WindowSurface};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut example = EglExample {
        registry_state: RegistryState::new(&conn, &qh),
        compositor_state: CompositorState::new(),
        output_state: OutputState::new(),
        xdg_shell_state: XdgShellState::new(),
        xdg_window_state: XdgWindowState::new(),
        exit: false,
        width: 1280,
        height: 720,
        context: None,
        surface: None,
        glow: None,
    };

    while !example.registry_state.ready() {
        event_queue.blocking_dispatch(&mut example).unwrap();
    }

    let surface = example.compositor_state.create_surface(&qh).unwrap();
    let window = Window::builder()
        .title("An EGL wayland window")
        .min_size((850, 480))
        // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
        .app_id("io.github.smithay.client-toolkit.Egl")
        .map(&qh, &example.xdg_shell_state, &mut example.xdg_window_state, surface)
        .unwrap();

    let (context, surface) = init_egl(window.wl_surface(), 1280, 720);
    let context = context.make_current(&surface).unwrap();

    let glow = unsafe {
        glow::Context::from_loader_function(|name| {
            // TODO: When glow updates, the CString conversion can be removed.
            let name = CString::new(name).unwrap();
            context.get_proc_address(name.as_c_str())
        })
    };

    example.context = Some(context);
    example.surface = Some(surface);
    example.glow = Some(glow);

    loop {
        event_queue.blocking_dispatch(&mut example).unwrap();

        if example.exit {
            println!("exiting example");
            break;
        }
    }
}

struct EglExample {
    registry_state: RegistryState,
    compositor_state: CompositorState,
    output_state: OutputState,
    xdg_shell_state: XdgShellState,
    xdg_window_state: XdgWindowState,

    exit: bool,
    width: i32,
    height: i32,
    context: Option<egl::context::PossiblyCurrentContext>,
    surface: Option<egl::surface::Surface<glutin::surface::WindowSurface>>,
    glow: Option<glow::Context>,
}

impl EglExample {
    pub fn resize(&mut self) {
        let context = self.context.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();

        surface.resize(
            context,
            NonZeroU32::new(self.width as u32).unwrap(),
            NonZeroU32::new(self.height as u32).unwrap(),
        );
    }

    pub fn draw(&mut self) {
        let context = self.context.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let glow = self.glow.as_ref().unwrap();

        unsafe {
            glow.viewport(0, 0, self.width, self.height);
            glow.clear(glow::COLOR_BUFFER_BIT);
            glow.clear_color(0.1, 0.2, 0.3, 1.0);
        }

        surface.swap_buffers(context).unwrap();
    }
}

fn init_egl(
    surface: &wl_surface::WlSurface,
    width: u32,
    height: u32,
) -> (egl::context::NotCurrentContext, egl::surface::Surface<glutin::surface::WindowSurface>) {
    let mut display_handle = raw_window_handle::WaylandDisplayHandle::empty();
    display_handle.display =
        surface.backend().upgrade().expect("Connection has been closed").display_ptr() as *mut _;
    let display_handle = raw_window_handle::RawDisplayHandle::Wayland(display_handle);
    let mut window_handle = raw_window_handle::WaylandWindowHandle::empty();
    window_handle.surface = surface.id().as_ptr() as *mut _;
    let window_handle = raw_window_handle::RawWindowHandle::Wayland(window_handle);

    // Initialize the EGL Wayland platform
    //
    // SAFETY: The connection is valid.
    let display = unsafe { egl::display::Display::from_raw(display_handle) }
        .expect("Failed to initialize Wayland EGL platform");

    // Find a suitable config for the window.
    let config_template = glutin::config::ConfigTemplateBuilder::default()
        .compatible_with_native_window(window_handle)
        .with_surface_type(ConfigSurfaceTypes::WINDOW)
        .with_api(
            glutin::config::Api::GLES2 | glutin::config::Api::GLES3 | glutin::config::Api::OPENGL,
        )
        .build();
    let config = unsafe { display.find_configs(config_template) }
        .unwrap()
        .next()
        .expect("No available configs");
    let gl_attrs = glutin::context::ContextAttributesBuilder::default()
        .with_context_api(glutin::context::ContextApi::OpenGl(None))
        .build(Some(window_handle));
    let gles_attrs = glutin::context::ContextAttributesBuilder::default()
        .with_context_api(glutin::context::ContextApi::Gles(None))
        .build(Some(window_handle));

    // Create a context, trying OpenGL and then Gles.
    let context = unsafe { display.create_context(&config, &gl_attrs) }
        .or_else(|_| unsafe { display.create_context(&config, &gles_attrs) })
        .expect("Failed to create context");

    let surface_attrs = glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::default()
        .build(window_handle, NonZeroU32::new(width).unwrap(), NonZeroU32::new(height).unwrap());
    let surface = unsafe { display.create_window_surface(&config, &surface_attrs) }
        .expect("Failed to create surface");

    (context, surface)
}

impl CompositorHandler for EglExample {
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
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }
}

impl OutputHandler for EglExample {
    fn output_state(&mut self) -> &mut smithay_client_toolkit::output::OutputState {
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

impl XdgShellHandler for EglExample {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }
}

impl WindowHandler for EglExample {
    fn xdg_window_state(&mut self) -> &mut XdgWindowState {
        &mut self.xdg_window_state
    }

    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size.unwrap_or((1280, 800));

        self.width = width as i32;
        self.height = height as i32;
        self.resize();
        self.draw();
    }
}

delegate_compositor!(EglExample);
delegate_output!(EglExample);
delegate_xdg_shell!(EglExample);
delegate_xdg_window!(EglExample);
delegate_registry!(EglExample);

impl ProvidesRegistryState for EglExample {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![CompositorState, OutputState, XdgShellState, XdgWindowState];
}
