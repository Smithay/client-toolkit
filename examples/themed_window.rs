use std::sync::Arc;
use std::time::Duration;
use std::{convert::TryInto, num::NonZeroU32};

use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, Proxy, QueueHandle,
};
use smithay_client_toolkit::reexports::csd_frame::{
    DecorationsFrame, FrameAction, FrameClick, ResizeEdge,
};
use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge as XdgResizeEdge;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_subcompositor, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{
            CursorIcon, PointerData, PointerEvent, PointerEventKind, PointerHandler, ThemeSpec,
            ThemedPointer,
        },
        Capability, SeatHandler, SeatState,
    },
    shell::{
        xdg::{
            fallback_frame::FallbackFrame,
            window::{DecorationMode, Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell, XdgSurface,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
    subcompositor::SubcompositorState,
};

// Cursor shapes.
const CURSORS: &[CursorIcon] = &[
    CursorIcon::Default,
    CursorIcon::Crosshair,
    CursorIcon::Pointer,
    CursorIcon::Move,
    CursorIcon::Text,
    CursorIcon::Wait,
    CursorIcon::Help,
    CursorIcon::Progress,
    CursorIcon::NotAllowed,
    CursorIcon::ContextMenu,
    CursorIcon::Cell,
    CursorIcon::VerticalText,
    CursorIcon::Alias,
    CursorIcon::Copy,
    CursorIcon::NoDrop,
    CursorIcon::Grab,
    CursorIcon::Grabbing,
    CursorIcon::AllScroll,
    CursorIcon::ZoomIn,
    CursorIcon::ZoomOut,
    CursorIcon::EResize,
    CursorIcon::NResize,
    CursorIcon::NeResize,
    CursorIcon::NwResize,
    CursorIcon::SResize,
    CursorIcon::SeResize,
    CursorIcon::SwResize,
    CursorIcon::WResize,
    CursorIcon::EwResize,
    CursorIcon::NsResize,
    CursorIcon::NeswResize,
    CursorIcon::NwseResize,
    CursorIcon::ColResize,
    CursorIcon::RowResize,
];

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();
    let registry_state = RegistryState::new(&globals);
    let seat_state = SeatState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);
    let compositor_state =
        CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let subcompositor_state =
        SubcompositorState::bind(compositor_state.wl_compositor().clone(), &globals, &qh)
            .expect("wl_subcompositor not available");
    let shm_state = Shm::bind(&globals, &qh).expect("wl_shm not available");
    let xdg_shell_state = XdgShell::bind(&globals, &qh).expect("xdg shell not available");

    let width = NonZeroU32::new(256).unwrap();
    let height = NonZeroU32::new(256).unwrap();
    let pool = SlotPool::new(width.get() as usize * height.get() as usize * 4, &shm_state)
        .expect("Failed to create pool");

    let window_surface = compositor_state.create_surface(&qh);

    let window =
        xdg_shell_state.create_window(window_surface, WindowDecorations::ServerDefault, &qh);
    window.set_title("A wayland window");
    // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
    window.set_app_id("io.github.smithay.client-toolkit.SimpleWindow");
    window.set_min_size(Some((width.get(), height.get())));

    // In order for the window to be mapped, we need to perform an initial commit with no attached buffer.
    // For more info, see WaylandSurface::commit
    //
    // The compositor will respond with an initial configure that we can then use to present to the window with
    // the correct options.
    window.commit();

    println!("Press `n` to cycle through cursor icons.");

    let mut simple_window = SimpleWindow {
        registry_state,
        seat_state,
        output_state,
        compositor_state,
        subcompositor_state: Arc::new(subcompositor_state),
        shm_state,
        _xdg_shell_state: xdg_shell_state,

        exit: false,
        first_configure: true,
        pool,
        width,
        height,
        shift: None,
        buffer: None,
        window,
        window_frame: None,
        keyboard: None,
        keyboard_focus: false,
        themed_pointer: None,
        set_cursor: false,
        window_cursor_icon_idx: 0,
        decorations_cursor: None,
    };

    // We don't draw immediately, the configure will notify us when to first draw.
    loop {
        event_queue.blocking_dispatch(&mut simple_window).unwrap();

        if simple_window.exit {
            println!("Exiting example.");
            break;
        }
    }
}

struct SimpleWindow {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    subcompositor_state: Arc<SubcompositorState>,
    shm_state: Shm,
    _xdg_shell_state: XdgShell,

    exit: bool,
    first_configure: bool,
    pool: SlotPool,
    width: NonZeroU32,
    height: NonZeroU32,
    shift: Option<u32>,
    buffer: Option<Buffer>,
    window: Window,
    window_frame: Option<FallbackFrame<Self>>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    themed_pointer: Option<ThemedPointer>,
    set_cursor: bool,
    window_cursor_icon_idx: usize,
    decorations_cursor: Option<CursorIcon>,
}

impl CompositorHandler for SimpleWindow {
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

impl OutputHandler for SimpleWindow {
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

impl WindowHandler for SimpleWindow {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        self.buffer = None;

        println!(
            "Configure size {:?}, decorations: {:?}",
            configure.new_size, configure.decoration_mode
        );

        let (width, height) = if configure.decoration_mode == DecorationMode::Client {
            let window_frame = self.window_frame.get_or_insert_with(|| {
                FallbackFrame::new(
                    &self.window,
                    &self.shm_state,
                    self.subcompositor_state.clone(),
                    qh.clone(),
                )
                .expect("failed to create client side decorations frame.")
            });

            // Un-hide the frame.
            window_frame.set_hidden(false);

            // Configure state before touching any resizing.
            window_frame.update_state(configure.state);

            // Update the capabilities.
            window_frame.update_wm_capabilities(configure.capabilities);

            let (width, height) = match configure.new_size {
                (Some(width), Some(height)) => {
                    // The size could be 0.
                    window_frame.subtract_borders(width, height)
                }
                _ => {
                    // You might want to consider checking for configure bounds.
                    (Some(self.width), Some(self.height))
                }
            };

            // Clamp the size to at least one pixel.
            let width = width.unwrap_or(NonZeroU32::new(1).unwrap());
            let height = height.unwrap_or(NonZeroU32::new(1).unwrap());

            println!("New dimentions: {width}, {height}");
            window_frame.resize(width, height);

            let (x, y) = window_frame.location();
            let outer_size = window_frame.add_borders(width.get(), height.get());
            window.xdg_surface().set_window_geometry(
                x,
                y,
                outer_size.0 as i32,
                outer_size.1 as i32,
            );

            (width, height)
        } else {
            // Hide the frame, if any.
            if let Some(frame) = self.window_frame.as_mut() {
                frame.set_hidden(true)
            }
            let width = configure.new_size.0.unwrap_or(self.width);
            let height = configure.new_size.1.unwrap_or(self.height);
            self.window.xdg_surface().set_window_geometry(
                0,
                0,
                width.get() as i32,
                height.get() as i32,
            );
            (width, height)
        };

        // Update new width and height;
        self.width = width;
        self.height = height;

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh);
        }
    }
}

impl SeatHandler for SimpleWindow {
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
            let keyboard =
                self.seat_state.get_keyboard(qh, &seat, None).expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.themed_pointer.is_none() {
            println!("Set pointer capability");
            let surface = self.compositor_state.create_surface(qh);
            let themed_pointer = self
                .seat_state
                .get_pointer_with_theme(
                    qh,
                    &seat,
                    self.shm_state.wl_shm(),
                    surface,
                    ThemeSpec::default(),
                )
                .expect("Failed to create pointer");
            self.themed_pointer.replace(themed_pointer);
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

        if capability == Capability::Pointer && self.themed_pointer.is_some() {
            println!("Unset pointer capability");
            self.themed_pointer.take().unwrap().pointer().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for SimpleWindow {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        keysyms: &[Keysym],
    ) {
        if self.window.wl_surface() == surface {
            println!("Keyboard focus on window with pressed syms: {keysyms:?}");
            self.keyboard_focus = true;
        }
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if self.window.wl_surface() == surface {
            self.keyboard_focus = false;
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if event.keysym == Keysym::N {
            // Cycle through cursor icons.
            self.window_cursor_icon_idx = (self.window_cursor_icon_idx + 1) % CURSORS.len();
            println!("Setting cursor icon to: {}", CURSORS[self.window_cursor_icon_idx].name());
            self.set_cursor = true;
        }
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
    }
}

impl PointerHandler for SimpleWindow {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            let (x, y) = event.position;
            match event.kind {
                Enter { .. } => {
                    self.set_cursor = true;
                    self.decorations_cursor = self.window_frame.as_mut().and_then(|frame| {
                        frame.click_point_moved(Duration::ZERO, &event.surface.id(), x, y)
                    });
                }
                Leave { .. } => {
                    if &event.surface != self.window.wl_surface() {
                        if let Some(window_frame) = self.window_frame.as_mut() {
                            window_frame.click_point_left();
                        }
                    }
                }
                Motion { time } => {
                    if let Some(new_cursor) = self.window_frame.as_mut().and_then(|frame| {
                        frame.click_point_moved(
                            Duration::from_millis(time as u64),
                            &event.surface.id(),
                            x,
                            y,
                        )
                    }) {
                        self.set_cursor = true;
                        self.decorations_cursor = Some(new_cursor);
                    }
                }
                Press { button, serial, time } | Release { button, serial, time } => {
                    let pressed = matches!(event.kind, Press { .. });
                    if &event.surface != self.window.wl_surface() {
                        let click = match button {
                            0x110 => FrameClick::Normal,
                            0x111 => FrameClick::Alternate,
                            _ => continue,
                        };

                        if let Some(action) = self.window_frame.as_mut().and_then(|frame| {
                            frame.on_click(Duration::from_millis(time as u64), click, pressed)
                        }) {
                            self.frame_action(pointer, serial, action);
                        }
                    } else if pressed {
                        self.shift = self.shift.xor(Some(0));
                    }
                }
                Axis { .. } => {}
            }
        }
    }
}

impl SimpleWindow {
    fn frame_action(&mut self, pointer: &wl_pointer::WlPointer, serial: u32, action: FrameAction) {
        let pointer_data = pointer.data::<PointerData>().unwrap();
        let seat = pointer_data.seat();
        match action {
            FrameAction::Close => self.exit = true,
            FrameAction::Minimize => self.window.set_minimized(),
            FrameAction::Maximize => self.window.set_maximized(),
            FrameAction::UnMaximize => self.window.unset_maximized(),
            FrameAction::ShowMenu(x, y) => self.window.show_window_menu(seat, serial, (x, y)),
            FrameAction::Resize(edge) => {
                let edge = match edge {
                    ResizeEdge::None => XdgResizeEdge::None,
                    ResizeEdge::Top => XdgResizeEdge::Top,
                    ResizeEdge::Bottom => XdgResizeEdge::Bottom,
                    ResizeEdge::Left => XdgResizeEdge::Left,
                    ResizeEdge::TopLeft => XdgResizeEdge::TopLeft,
                    ResizeEdge::BottomLeft => XdgResizeEdge::BottomLeft,
                    ResizeEdge::Right => XdgResizeEdge::Right,
                    ResizeEdge::TopRight => XdgResizeEdge::TopRight,
                    ResizeEdge::BottomRight => XdgResizeEdge::BottomRight,
                    _ => return,
                };
                self.window.resize(seat, serial, edge);
            }
            FrameAction::Move => self.window.move_(seat, serial),
            _ => (),
        }
    }
}

impl ShmHandler for SimpleWindow {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl SimpleWindow {
    pub fn draw(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        if self.set_cursor {
            let cursor_icon =
                self.decorations_cursor.unwrap_or(CURSORS[self.window_cursor_icon_idx]);
            let _ = self.themed_pointer.as_mut().unwrap().set_cursor(conn, cursor_icon);
            self.set_cursor = false;
        }

        let width = self.width.get();
        let height = self.height.get();
        let stride = self.width.get() as i32 * 4;

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
                .expect("create buffer")
                .0
        });

        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                // This should be rare, but if the compositor has not released the previous
                // buffer, we need double-buffering.
                let (second_buffer, canvas) = self
                    .pool
                    .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
                    .expect("create buffer");
                *buffer = second_buffer;
                canvas
            }
        };

        // Draw to the window:
        {
            let shift = self.shift.unwrap_or(0);
            canvas.chunks_exact_mut(4).enumerate().for_each(|(index, chunk)| {
                let x = ((index + shift as usize) % width as usize) as u32;
                let y = (index / width as usize) as u32;

                let a = 0xFF;
                let r = u32::min(((width - x) * 0xFF) / width, ((height - y) * 0xFF) / height);
                let g = u32::min((x * 0xFF) / width, ((height - y) * 0xFF) / height);
                let b = u32::min(((width - x) * 0xFF) / width, (y * 0xFF) / height);
                let color = (a << 24) + (r << 16) + (g << 8) + b;

                let array: &mut [u8; 4] = chunk.try_into().unwrap();
                *array = color.to_le_bytes();
            });

            if let Some(shift) = &mut self.shift {
                *shift = (*shift + 1) % width;
            }
        }

        // Draw the decorations frame.
        if let Some(frame) = self.window_frame.as_mut() {
            if frame.is_dirty() && !frame.is_hidden() {
                frame.draw();
            }
        }

        // Damage the entire window
        self.window.wl_surface().damage_buffer(0, 0, width as i32, height as i32);

        // Request our next frame
        self.window.wl_surface().frame(qh, self.window.wl_surface().clone());

        // Attach and commit to present.
        buffer.attach_to(self.window.wl_surface()).expect("buffer attach");
        self.window.wl_surface().commit();
    }
}

delegate_compositor!(SimpleWindow);
delegate_subcompositor!(SimpleWindow);
delegate_output!(SimpleWindow);
delegate_shm!(SimpleWindow);

delegate_seat!(SimpleWindow);
delegate_keyboard!(SimpleWindow);
delegate_pointer!(SimpleWindow);

delegate_xdg_shell!(SimpleWindow);
delegate_xdg_window!(SimpleWindow);

delegate_registry!(SimpleWindow);

impl ProvidesRegistryState for SimpleWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}
