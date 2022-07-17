use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, WaylandHandle};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_registry, delegate_seat, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
    shell::xdg::{
        window::{Window, WindowConfigure, WindowHandler, XdgWindowState},
        XdgShellHandler, XdgShellState,
    },
};
use wayland_client::{
    protocol::{wl_output, wl_seat, wl_surface},
    Connection, Proxy, QueueHandle,
};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    // Initialize wgpu
    let instance = wgpu::Instance::new(wgpu::Backends::all());

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut wgpu = Wgpu {
        registry_state: RegistryState::new(&conn, &qh),
        seat_state: SeatState::new(),
        output_state: OutputState::new(),
        compositor_state: CompositorState::new(),
        xdg_shell_state: XdgShellState::new(),
        xdg_window_state: XdgWindowState::new(),

        exit: false,
        width: 256,
        height: 256,
        window: None,
        instance,
        device: None,
        surface: None,
        adapter: None,
        queue: None,
    };

    while !wgpu.registry_state.ready() {
        event_queue.blocking_dispatch(&mut wgpu).unwrap();
    }

    let surface = wgpu.compositor_state.create_surface(&qh).unwrap();

    let window = Window::builder()
        .title("wgpu wayland window")
        // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
        .app_id("io.github.smithay.client-toolkit.WgpuExample")
        .min_size((256, 256))
        .map(&qh, &wgpu.xdg_shell_state, &mut wgpu.xdg_window_state, surface)
        .expect("window creation");

    wgpu.window = Some(window);

    // Initialize wgpu's device and surface
    let handle = {
        let mut handle = WaylandHandle::empty();
        handle.display = conn.backend().display_ptr() as *mut _;
        handle.surface = wgpu.window.as_ref().unwrap().wl_surface().id().as_ptr() as *mut _;
        let window_handle = RawWindowHandle::Wayland(handle);

        /// https://github.com/rust-windowing/raw-window-handle/issues/49
        struct YesRawWindowHandleImplementingHasRawWindowHandleIsUnsound(RawWindowHandle);

        unsafe impl HasRawWindowHandle for YesRawWindowHandleImplementingHasRawWindowHandleIsUnsound {
            fn raw_window_handle(&self) -> RawWindowHandle {
                self.0
            }
        }

        YesRawWindowHandleImplementingHasRawWindowHandleIsUnsound(window_handle)
    };

    wgpu.surface = Some(unsafe { wgpu.instance.create_surface(&handle) });

    // Pick the first supported adapter for the surface.
    let adapter = pollster::block_on(wgpu.instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: wgpu.surface.as_ref(),
        ..Default::default()
    }))
    .expect("Failed to find suitable adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default(), None))
        .expect("Failed to request device");
    wgpu.adapter = Some(adapter);
    wgpu.device = Some(device);
    wgpu.queue = Some(queue);

    // We don't draw immediately, the configure will notify us when to first draw.
    loop {
        event_queue.blocking_dispatch(&mut wgpu).unwrap();

        if wgpu.exit {
            println!("exiting example");
            break;
        }
    }

    // On exit we must destroy the surface before the connection is dropped.
    wgpu.surface.take();
}

struct Wgpu {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    xdg_window_state: XdgWindowState,

    exit: bool,
    width: u32,
    height: u32,
    window: Option<Window>,

    instance: wgpu::Instance,
    /// Can't initialize the adapter until we have a window.
    adapter: Option<wgpu::Adapter>,
    /// Can't initialize the device until we have a window.
    device: Option<wgpu::Device>,
    /// Can't initialize the queue until we have a window.
    queue: Option<wgpu::Queue>,
    /// Can't initialize the surface until we have a window.
    surface: Option<wgpu::Surface>,
}

impl CompositorHandler for Wgpu {
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

impl OutputHandler for Wgpu {
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

impl XdgShellHandler for Wgpu {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }
}

impl WindowHandler for Wgpu {
    fn xdg_window_state(&mut self) -> &mut XdgWindowState {
        &mut self.xdg_window_state
    }

    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        match configure.new_size {
            Some(size) => {
                self.width = size.0;
                self.height = size.1;
            }
            None => {
                self.width = 256;
                self.height = 256;
            }
        }

        let adapter = self.adapter.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(adapter)[0],
            width: self.width,
            height: self.height,
            // Wayland is inherently a mailbox system.
            present_mode: wgpu::PresentMode::Mailbox,
        };

        surface.configure(self.device.as_ref().unwrap(), &surface_config);

        // We don't plan to render much in this example, just clear the surface.
        let surface_texture =
            surface.get_current_texture().expect("failed to acquire next swapchain texture");
        let texture_view =
            surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let _renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        // Submit the command in the queue to execute
        queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

impl SeatHandler for Wgpu {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

delegate_compositor!(Wgpu);
delegate_output!(Wgpu);

delegate_seat!(Wgpu);

delegate_xdg_shell!(Wgpu);
delegate_xdg_window!(Wgpu);

delegate_registry!(Wgpu);

impl ProvidesRegistryState for Wgpu {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![CompositorState, OutputState, SeatState, XdgShellState, XdgWindowState,];
}
