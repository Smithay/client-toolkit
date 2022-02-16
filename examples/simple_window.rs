use std::convert::TryInto;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    seat::{
        keyboard::KeyboardHandler,
        pointer::{PointerHandler, PointerScroll},
        Capability, SeatHandler, SeatState,
    },
    shell::xdg::{
        window::{Window, WindowHandler},
        XdgShellHandler, XdgShellState,
    },
    shm::{pool::raw::RawPool, ShmHandler, ShmState},
};
use wayland_client::{
    protocol::{wl_buffer, wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, ConnectionHandle, Dispatch, QueueHandle,
};
use wayland_protocols::xdg_shell::client::xdg_surface;

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let display = conn.handle().display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&mut conn.handle(), &qh, ()).unwrap();

    let mut simple_window = SimpleWindow {
        registry_state: RegistryState::new(registry),
        seat_state: SeatState::new(),
        output_state: OutputState::new(),
        compositor_state: CompositorState::new(),
        shm_state: ShmState::new(),
        xdg_shell_state: XdgShellState::new(),

        exit: false,
        first_configure: true,
        pool: None,
        width: 256,
        height: 256,
        buffer: None,
        window: None,
        keyboard: None,
        keyboard_focus: false,
        pointer: None,
        pointer_focus: false,
    };

    event_queue.blocking_dispatch(&mut simple_window).unwrap();
    event_queue.blocking_dispatch(&mut simple_window).unwrap();

    let pool = simple_window
        .shm_state
        .new_raw_pool(
            simple_window.width as usize * simple_window.height as usize * 4,
            &mut conn.handle(),
            &qh,
            (),
        )
        .expect("Failed to create pool");
    simple_window.pool = Some(pool);

    let surface = simple_window.compositor_state.create_surface(&mut conn.handle(), &qh).unwrap();

    let window = simple_window
        .xdg_shell_state
        .create_window(&mut conn.handle(), &qh, surface)
        .expect("window");

    window.set_title(&mut conn.handle(), "A wayland window");
    // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
    window.set_app_id(&mut conn.handle(), "io.github.smithay.client-toolkit.SimpleWindow");
    window.set_min_size(&mut conn.handle(), Some((256, 256)));

    // Map the window so we receive the initial configure and can render.
    window.map(&mut conn.handle(), &qh);

    simple_window.window = Some(window);

    // We don't draw immediately, the configure will indicate to us when to first draw.

    loop {
        event_queue.blocking_dispatch(&mut simple_window).unwrap();

        if simple_window.exit {
            println!("exiting example");
            break;
        }
    }
}

struct SimpleWindow {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: ShmState,
    xdg_shell_state: XdgShellState,

    exit: bool,
    first_configure: bool,
    pool: Option<RawPool>,
    width: u32,
    height: u32,
    buffer: Option<wl_buffer::WlBuffer>,
    window: Option<Window>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    pointer: Option<wl_pointer::WlPointer>,
    pointer_focus: bool,
}

impl CompositorHandler for SimpleWindow {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn scale_factor_changed(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(conn, qh);
    }
}

impl OutputHandler for SimpleWindow {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl XdgShellHandler for SimpleWindow {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn configure(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        // The only surface we can ever have in this example is our one window, so configure the window.
        //
        // If you have more than one surface, you'll need to use this to lookup the surface you need.
        _: &xdg_surface::XdgSurface,
    ) {
        let window = self.window.as_mut().unwrap();
        let configure = window.configure().unwrap();

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

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh);
        }
    }
}

impl WindowHandler for SimpleWindow {
    fn request_close_window(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: &Window,
    ) {
        self.exit = true;
    }
}

impl SeatHandler for SimpleWindow {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &mut ConnectionHandle, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let keyboard =
                self.seat_state.get_keyboard(conn, qh, &seat).expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer =
                self.seat_state.get_pointer(conn, qh, &seat).expect("Failed to create pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        conn: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release(conn);
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release(conn);
        }
    }

    fn remove_seat(&mut self, _: &mut ConnectionHandle, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {
    }
}

impl KeyboardHandler for SimpleWindow {
    fn keyboard_focus(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
    ) {
        if self.window.as_ref().map(Window::wl_surface) == Some(surface) {
            println!("Keyboard focus on window");
            self.keyboard_focus = true;
        }
    }

    fn keyboard_release_focus(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
    ) {
        if self.window.as_ref().map(Window::wl_surface) == Some(surface) {
            println!("Release keyboard focus on window");
            self.keyboard_focus = false;
        }
    }

    fn keyboard_press_key(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        time: u32,
        key: u32,
    ) {
        println!("Key press: {} @ {}", key, time);
    }

    fn keyboard_release_key(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        time: u32,
        key: u32,
    ) {
        println!("Key release: {} @ {}", key, time);
    }

    fn keyboard_update_modifiers(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        // TODO: Other params
    ) {
    }

    fn keyboard_update_repeat_info(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _rate: u32,
        _delay: u32,
    ) {
    }
}

impl PointerHandler for SimpleWindow {
    fn pointer_focus(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        surface: &wl_surface::WlSurface,
        entered: (f64, f64),
    ) {
        if self.window.as_ref().map(Window::wl_surface) == Some(surface) {
            println!("Pointer focus on window, entering at {:?}", entered);
            self.pointer_focus = true;
        }
    }

    fn pointer_release_focus(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        surface: &wl_surface::WlSurface,
    ) {
        if self.window.as_ref().map(Window::wl_surface) == Some(surface) {
            println!("Release pointer focus on window");
            self.pointer_focus = false;
        }
    }

    fn pointer_motion(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        time: u32,
        position: (f64, f64),
    ) {
        if self.pointer_focus {
            println!("Pointer motion: {:?} @ {}", position, time);
        }
    }

    fn pointer_press_button(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        time: u32,
        button: u32,
    ) {
        if self.pointer_focus {
            println!("Pointer press button: {:?} @ {}", button, time);
        }
    }

    fn pointer_release_button(
        &mut self,
        _conn: &mut ConnectionHandle,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        time: u32,
        button: u32,
    ) {
        if self.pointer_focus {
            println!("Pointer release button: {:?} @ {}", button, time);
        }
    }

    fn pointer_axis(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        time: u32,
        scroll: PointerScroll,
    ) {
        if self.pointer_focus {
            println!("Pointer scroll: @ {}", time);

            if let Some(vertical) = scroll.axis(wl_pointer::Axis::VerticalScroll) {
                println!("\nV: {:?}", vertical);
            }

            if let Some(horizontal) = scroll.axis(wl_pointer::Axis::HorizontalScroll) {
                println!("\nH: {:?}", horizontal);
            }
        }
    }
}

impl ShmHandler for SimpleWindow {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
    }
}

impl SimpleWindow {
    pub fn draw(&mut self, conn: &mut ConnectionHandle, qh: &QueueHandle<Self>) {
        if let Some(window) = self.window.as_ref() {
            // Ensure the pool is big enough to hold the new buffer.
            self.pool
                .as_mut()
                .unwrap()
                .resize((self.width * self.height * 4) as usize, conn)
                .expect("resize pool");

            // Destroy the old buffer.
            // FIXME: Integrate this into the pool logic.
            if let Some(buffer) = self.buffer.take() {
                buffer.destroy(conn);
            }

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
                    qh,
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

            self.buffer = Some(wl_buffer);

            // Request our next frame
            window
                .wl_surface()
                .frame(conn, qh, window.wl_surface().clone())
                .expect("create callback");

            assert!(self.buffer.is_some(), "No buffer?");
            // Attach and commit to present.
            window.wl_surface().attach(conn, self.buffer.as_ref(), 0, 0);
            window.wl_surface().commit(conn);
        }
    }
}

delegate_compositor!(SimpleWindow);
delegate_output!(SimpleWindow);
delegate_shm!(SimpleWindow);

delegate_seat!(SimpleWindow);
delegate_keyboard!(SimpleWindow);
delegate_pointer!(SimpleWindow);

delegate_xdg_shell!(SimpleWindow);
delegate_xdg_window!(SimpleWindow);

delegate_registry!(SimpleWindow: [
    CompositorState,
    OutputState,
    ShmState,
    SeatState,
    XdgShellState,
]);

impl ProvidesRegistryState for SimpleWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
}

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
