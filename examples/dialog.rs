use std::collections::hash_map::RandomState;
use std::env;
use std::hash::{BuildHasher, Hasher};

use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle};
use smithay_client_toolkit::seat::keyboard::KeyboardHandler;
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::xdg::dialog::{Dialog, DialogHandler};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, FrameCallbackData},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};
use wayland_client::protocol::wl_keyboard;
use wayland_client::protocol::wl_seat;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};

#[derive(PartialEq)]
enum WindowType {
    Window(Window),
    Dialog(Dialog),
}

impl WaylandSurface for WindowType {
    fn wl_surface(&self) -> &wl_surface::WlSurface {
        match self {
            Self::Window(window) => window.wl_surface(),
            Self::Dialog(dialog) => dialog.wl_surface(),
        }
    }
}

struct Viewer {
    window: WindowType,
    width: u32,
    height: u32,
    buffer: Option<Buffer>,
    first_configure: bool,
    damaged: bool,
}

struct State {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    xdg_shell_state: XdgShell,
    pool: Option<SlotPool>,
    windows: Vec<Viewer>,

    have_dialog: bool,
    loop_handle: LoopHandle<'static, State>,
    seat_state: SeatState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
}

/// Creates a simple red window
/// When pressing any keyboard key a dialog (green window) is created
/// this window is modal to the red window so that first this dialog must be close
/// by pressing again any key
/// The modality can be changed by setting set_modal(false) on the dialog
fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh: QueueHandle<State> = event_queue.handle();

    let event_loop: EventLoop<State> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");

    let mut state = State {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        compositor_state: CompositorState::bind(&globals, &qh)
            .expect("wl_compositor not available"),
        shm_state: Shm::bind(&globals, &qh).expect("wl_shm not available"),
        xdg_shell_state: XdgShell::bind(&globals, &qh).expect("xdg shell not available"),

        pool: None,
        windows: Vec::new(),
        have_dialog: false,

        loop_handle: event_loop.handle(),
        seat_state: SeatState::new(&globals, &qh),
        keyboard: None,
    };

    let surface = state.compositor_state.create_surface(&qh);
    let window =
        state.xdg_shell_state.create_window(surface, WindowDecorations::ServerDefault, &qh);
    window.set_title("A wayland window");
    // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
    window.set_app_id("io.github.smithay.client-toolkit.Dialog");

    // In order for the window to be mapped, we need to perform an initial commit with no attached buffer.
    // For more info, see WaylandSurface::commit
    //
    // The compositor will respond with an initial configure that we can then use to present to the window with
    // the correct options.
    window.commit();

    state.windows.push(Viewer {
        width: 500,
        height: 500,
        window: WindowType::Window(window),
        first_configure: true,
        damaged: true,
        buffer: None,
    });

    let pool = SlotPool::new(2, &state.shm_state).expect("Failed to create pool");
    state.pool = Some(pool);

    loop {
        event_queue.blocking_dispatch(&mut state).unwrap();

        if state.windows.is_empty() {
            println!("exiting example");
            break;
        }
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

            // Fill the window with a random color:
            {
                let (r, g, b) = if matches!(viewer.window, WindowType::Window(..)) {
                    (0xff, 0x00, 0x00)
                } else {
                    (0x00, 0xff, 0x00)
                };

                for argb in canvas.chunks_exact_mut(4) {
                    // Send pixels to the server in ARGB8888 format (this is one of the only
                    // formats that are guaranteed to be supported).
                    argb[3] = 0xff;
                    argb[2] = r;
                    argb[1] = g;
                    argb[0] = b;
                }
            }

            // Damage the entire window
            window.wl_surface().damage_buffer(0, 0, viewer.width as i32, viewer.height as i32);
            viewer.damaged = false;

            // Request our next frame
            window.wl_surface().frame(qh, FrameCallbackData(window.wl_surface().clone()));

            // Attach and commit to present.
            buffer.attach_to(window.wl_surface()).expect("buffer attach");
            window.wl_surface().commit();
        }
    }
}

impl PartialEq<Window> for WindowType {
    fn eq(&self, other: &Window) -> bool {
        match self {
            Self::Window(window) => window == other,
            _ => false,
        }
    }
}

impl PartialEq<Dialog> for WindowType {
    fn eq(&self, other: &Dialog) -> bool {
        match self {
            Self::Dialog(dialog) => dialog == other,
            _ => false,
        }
    }
}

// Trait implementations

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
        println!("Closing a Window");
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

            viewer.buffer = None;
            viewer.width = configure.new_size.0.map(|v| v.get()).unwrap_or(256);
            viewer.height = configure.new_size.1.map(|v| v.get()).unwrap_or(256);
            viewer.damaged = true;

            // Initiate the first draw.
            viewer.first_configure = false;
        }
        self.draw(conn, qh);
    }
}

impl DialogHandler for State {
    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &smithay_client_toolkit::shell::xdg::dialog::Dialog,
        configure: WindowConfigure,
        serial: u32,
    ) {
        for viewer in &mut self.windows {
            if viewer.window != *window {
                continue;
            }

            viewer.buffer = None;
            viewer.width = configure.new_size.0.map(|v| v.get()).unwrap_or(100);
            viewer.height = configure.new_size.1.map(|v| v.get()).unwrap_or(100);
            viewer.damaged = true;

            // Initiate the first draw.
            viewer.first_configure = false;
        }
        self.draw(conn, qh);
    }

    fn request_close(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &smithay_client_toolkit::shell::xdg::dialog::Dialog,
    ) {
        println!("Closing a Popup");
        self.windows.retain(|v| v.window != *window);
        self.have_dialog = false;
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|_state, _wl_kbd, event| {
                        println!("Repeat: {:?} ", event);
                    }),
                )
                .expect("Failed to create keyboard");

            self.keyboard = Some(keyboard);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        serial: u32,
        raw: &[u32],
        keysyms: &[xkeysym::Keysym],
    ) {
    }

    fn leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        serial: u32,
    ) {
    }

    fn press_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        if self.have_dialog {
            // Close the dialog
            self.windows.retain(|w| matches!(w.window, WindowType::Window(..)));
            self.have_dialog = false;
        } else {
            // Create a new dialog
            self.have_dialog = true;
            let surface = self.compositor_state.create_surface(qh);
            if let WindowType::Window(window) = &self.windows.first().unwrap().window {
                let window = window.clone();
                let parent_surface = window.xdg_toplevel();
                let dialog = self
                    .xdg_shell_state
                    .create_dialog(surface, WindowDecorations::ServerDefault, qh, parent_surface)
                    .unwrap();
                dialog.commit();
                dialog.set_modal(true);
                self.windows.push(Viewer {
                    window: WindowType::Dialog(dialog),
                    width: 200,
                    height: 200,
                    buffer: None,
                    first_configure: true,
                    damaged: true,
                });
            }
        }
    }

    fn release_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
    }

    fn repeat_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
    }

    fn update_keymap(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _keymap: smithay_client_toolkit::seat::keyboard::Keymap<'_>,
    ) {
    }

    fn update_modifiers(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        serial: u32,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
        raw_modifiers: smithay_client_toolkit::seat::keyboard::RawModifiers,
        layout: u32,
    ) {
    }

    fn update_repeat_info(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _info: smithay_client_toolkit::seat::keyboard::RepeatInfo,
    ) {
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

delegate_registry!(State);

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState);
}

smithay_client_toolkit::delegate_dispatch2!(State);
