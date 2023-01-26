use std::{
    convert::TryInto,
    fs::File,
    io::{Read, Write},
    time::Duration,
};

use calloop::{LoopHandle, RegistrationToken};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    data_device_manager::{
        data_device::{DataDevice, DataDeviceDataExt, DataDeviceHandler},
        data_offer::{DataDeviceOffer, DataOfferHandler, DragOffer, SelectionOffer},
        data_source::{CopyPasteSource, DataSourceHandler, DragSource},
        DataDeviceManagerState,
    },
    delegate_compositor, delegate_data_device, delegate_data_device_manager, delegate_data_offer,
    delegate_data_source, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler, BTN_LEFT},
        Capability, SeatHandler, SeatState,
    },
    shell::xdg::{
        window::{Window, WindowConfigure, WindowHandler, XdgWindowState},
        XdgShellState,
    },
    shm::{
        slot::{Buffer, SlotPool},
        ShmHandler, ShmState,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{
        wl_data_device_manager::DndAction,
        wl_keyboard::{self, WlKeyboard},
        wl_output,
        wl_pointer::{self, WlPointer},
        wl_seat::{self, WlSeat},
        wl_shm, wl_surface,
    },
    Connection, QueueHandle, WaylandSource,
};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();
    let (globals, event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();
    let mut event_loop = calloop::EventLoop::try_new().unwrap();
    WaylandSource::new(event_queue).unwrap().insert(event_loop.handle()).unwrap();

    let mut simple_window = SimpleWindow {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor_state: CompositorState::bind(&globals, &qh)
            .expect("wl_compositor not available"),
        shm_state: ShmState::bind(&globals, &qh).expect("wl_shm not available"),
        xdg_shell_state: XdgShellState::bind(&globals, &qh).expect("xdg shell not available"),
        xdg_window_state: XdgWindowState::bind(&globals, &qh),
        data_device_manager_state: DataDeviceManagerState::bind(&globals, &qh)
            .expect("data device manager is not available"),

        exit: false,
        first_configure: true,
        pool: None,
        width: 256,
        height: 256,
        shift: None,
        buffer: None,
        window: None,
        keyboard: None,
        keyboard_focus: false,
        pointer: None,
        data_devices: Vec::new(),
        copy_paste_sources: Vec::new(),
        drag_sources: Vec::new(),
        loop_handle: event_loop.handle(),
        accept_counter: 0,
        dnd_offers: Vec::new(),
        selection_offers: Vec::new(),
    };

    let pool = SlotPool::new(
        simple_window.width as usize * simple_window.height as usize * 4,
        &simple_window.shm_state,
    )
    .expect("Failed to create pool");
    simple_window.pool = Some(pool);

    let surface = simple_window.compositor_state.create_surface(&qh);

    let window = Window::builder()
        .title("A wayland window")
        // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
        .app_id("io.github.smithay.client-toolkit.SimpleWindow")
        .min_size((256, 256))
        .map(&qh, &simple_window.xdg_shell_state, &mut simple_window.xdg_window_state, surface)
        .expect("window creation");

    simple_window.window = Some(window);

    // We don't draw immediately, the configure will notify us when to first draw.

    loop {
        event_loop.dispatch(Duration::from_millis(30), &mut simple_window).unwrap();

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
    xdg_window_state: XdgWindowState,
    data_device_manager_state: DataDeviceManagerState,

    exit: bool,
    first_configure: bool,
    pool: Option<SlotPool>,
    width: u32,
    height: u32,
    shift: Option<u32>,
    buffer: Option<Buffer>,
    window: Option<Window>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    pointer: Option<wl_pointer::WlPointer>,
    dnd_offers: Vec<(DragOffer, String, Option<RegistrationToken>)>,
    selection_offers: Vec<(SelectionOffer, String, Option<RegistrationToken>)>,
    data_devices: Vec<(WlSeat, Option<WlKeyboard>, Option<WlPointer>, DataDevice)>,
    copy_paste_sources: Vec<CopyPasteSource>,
    drag_sources: Vec<(DragSource, bool)>,
    loop_handle: LoopHandle<'static, SimpleWindow>,
    accept_counter: u32,
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
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        match configure.new_size {
            Some(size) => {
                self.width = size.0;
                self.height = size.1;
                self.buffer = None;
            }
            None => {
                self.width = 256;
                self.height = 256;
                self.buffer = None;
            }
        }

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

    fn new_seat(&mut self, _: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        let data_device =
            if let Some(data_device) = self.data_devices.iter_mut().find(|(s, ..)| s == &seat) {
                data_device
            } else {
                // create the data device here for this seat
                let data_device_manager = &self.data_device_manager_state;
                let data_device = data_device_manager.get_data_device(qh, &seat);
                self.data_devices.push((seat.clone(), None, None, data_device));
                self.data_devices.last_mut().unwrap()
            };
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let keyboard =
                self.seat_state.get_keyboard(qh, &seat, None).expect("Failed to create keyboard");
            self.keyboard = Some(keyboard.clone());
            data_device.1.replace(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self.seat_state.get_pointer(qh, &seat).expect("Failed to create pointer");
            self.pointer = Some(pointer.clone());
            data_device.2.replace(pointer);
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

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
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
        _keysyms: &[u32],
    ) {
        if self.window.as_ref().map(Window::wl_surface) == Some(surface) {
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
        if self.window.as_ref().map(Window::wl_surface) == Some(surface) {
            self.keyboard_focus = false;
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        kbd: &wl_keyboard::WlKeyboard,
        serial: u32,
        event: KeyEvent,
    ) {
        match event.utf8 {
            Some(s) if s.to_lowercase() == "c" => {
                println!("Creating copy paste source and setting selection...");
                if let Some(data_device) =
                    self.data_devices.iter().find(|(_, d_kbd, ..)| d_kbd.as_ref() == Some(&kbd))
                {
                    let source = self
                        .data_device_manager_state
                        .create_copy_paste_source(qh, vec!["text/plain"]);
                    source.set_selection(&data_device.3, serial);
                    self.copy_paste_sources.push(source);
                }
            }
            Some(s) => {
                dbg!(s);
            }
            _ => {}
        };
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
    ) {
    }
}

impl PointerHandler for SimpleWindow {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            // Ignore events for other surfaces
            if Some(&event.surface) != self.window.as_ref().map(Window::wl_surface) {
                continue;
            }
            let surface = event.surface.clone();
            match event.kind {
                Press { button, serial, .. } if button == BTN_LEFT => {
                    if let Some(data_device) = self
                        .data_devices
                        .iter()
                        .find(|(_, _, d_pointer, ..)| d_pointer.as_ref() == Some(&pointer))
                    {
                        self.shift = self.shift.xor(Some(0));
                        let source = self.data_device_manager_state.create_drag_and_drop_source(
                            qh,
                            vec!["text/plain"],
                            DndAction::Copy,
                        );

                        source.start_drag(&data_device.3, &surface, None, serial);
                        self.drag_sources.push((source, false));
                    }
                }
                Motion { .. } => {
                    // dbg!(event.position);
                }
                _ => {}
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
    pub fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        if let Some(window) = self.window.as_ref() {
            let width = self.width;
            let height = self.height;
            let stride = self.width as i32 * 4;
            let pool = self.pool.as_mut().unwrap();

            let buffer = self.buffer.get_or_insert_with(|| {
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
                            self.width as i32,
                            self.height as i32,
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

            // Damage the entire window
            window.wl_surface().damage_buffer(0, 0, self.width as i32, self.height as i32);

            // Request our next frame
            window.wl_surface().frame(qh, window.wl_surface().clone());

            // Attach and commit to present.
            buffer.attach_to(window.wl_surface()).expect("buffer attach");
            window.wl_surface().commit();
        }
    }
}

impl DataDeviceHandler for SimpleWindow {
    fn enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: DataDevice) {
        let mut drag_offer = data_device.drag_offer().unwrap();
        dbg!(drag_offer.x, drag_offer.y);

        for mime in drag_offer.mime_types.clone() {
            self.accept_counter += 1;
            println!("mime: {}", mime);
            drag_offer.accept_mime_type(self.accept_counter, Some(mime));
        }
    }

    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: DataDevice) {
        println!("data offer left");
    }

    fn motion(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: DataDevice) {
        let DragOffer { x, y, time, .. } = data_device.drag_offer().unwrap();

        dbg!((time, x, y));
    }

    fn selection(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, data_device: DataDevice) {
        if let Some(offer) = data_device.selection_offer() {
            self.selection_offers.push((offer, String::new(), None));
            let cur_offer = self.selection_offers.last_mut().unwrap();
            let mime_type = match cur_offer.0.mime_types.get(0) {
                Some(mime) => mime,
                None => return,
            };

            if let Ok(read_pipe) = cur_offer.0.receive(mime_type.clone()) {
                let offer_clone = cur_offer.0.clone();
                match self.loop_handle.insert_source(read_pipe, move |_, f, state| {
                    let (_, mut contents, token) = state
                        .selection_offers
                        .iter()
                        .position(|o| &o.0 == &offer_clone)
                        .map(|p| state.selection_offers.remove(p))
                        .unwrap();

                    f.read_to_string(&mut contents).unwrap();
                    println!("TEXT FROM Selection: {contents}");
                    state.loop_handle.remove(token.unwrap());
                }) {
                    Ok(token) => {
                        cur_offer.2.replace(token);
                    }
                    Err(err) => {
                        eprintln!("{:?}", err);
                    }
                }
            }
        }
    }

    fn drop_performed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        data_device: DataDevice,
    ) {
        if let Some(offer) = data_device.drag_offer() {
            dbg!(&offer);
            self.dnd_offers.push((offer, String::new(), None));
            let cur_offer = self.dnd_offers.last_mut().unwrap();
            let mime_type = match cur_offer.0.mime_types.get(0).cloned() {
                Some(mime) => mime,
                None => return,
            };
            dbg!(&mime_type);
            self.accept_counter += 1;
            cur_offer.0.accept_mime_type(self.accept_counter, Some(mime_type.clone()));
            cur_offer.0.set_actions(DndAction::Copy, DndAction::Copy);
            if let Ok(read_pipe) = cur_offer.0.receive(mime_type.clone()) {
                let offer_clone = cur_offer.0.clone();
                match self.loop_handle.insert_source(read_pipe, move |_, f, state| {
                    let (offer, mut contents, token) = state
                        .dnd_offers
                        .iter()
                        .position(|o| &o.0.inner() == &offer_clone.inner())
                        .map(|p| state.dnd_offers.remove(p))
                        .unwrap();

                    f.read_to_string(&mut contents).unwrap();
                    println!("TEXT FROM drop: {contents}");
                    state.loop_handle.remove(token.unwrap());

                    offer.finish();
                }) {
                    Ok(token) => {
                        cur_offer.2.replace(token);
                    }
                    Err(err) => {
                        eprintln!("{:?}", err);
                    }
                }
            }
        }
    }
}

impl DataOfferHandler for SimpleWindow {
    fn offer(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DataDeviceOffer,
        mime_type: String,
    ) {
        let serial = self.accept_counter;
        self.accept_counter += 1;
        if &mime_type == "text/plain" {
            offer.accept_mime_type(serial, Some(mime_type.clone()));
        }
    }

    fn source_actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DataDeviceOffer,
        actions: wayland_client::protocol::wl_data_device_manager::DndAction,
    ) {
        dbg!(actions);
        offer.set_actions(DndAction::Copy, DndAction::Copy);
    }

    fn actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _offer: &mut DataDeviceOffer,
        actions: wayland_client::protocol::wl_data_device_manager::DndAction,
    ) {
        dbg!(actions);
        // TODO ?
    }
}

impl DataSourceHandler for SimpleWindow {
    fn accept_mime(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
        _mime: Option<String>,
    ) {
    }

    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &wayland_client::protocol::wl_data_source::WlDataSource,
        mime: String,
        fd: wayland_backend::io_lifetimes::OwnedFd,
    ) {
        dbg!(&self.drag_sources);

        if let Some(_) = self
            .copy_paste_sources
            .iter_mut()
            .find(|s| s.inner() == source && mime == "text/plain".to_string())
        {
            let mut f = File::from(fd);
            writeln!(f, "Copied from selection via sctk").unwrap();
        } else if let Some(_) = self
            .drag_sources
            .iter_mut()
            .find(|s| s.0.inner() == source && mime == "text/plain".to_string() && s.1)
        {
            let mut f = File::from(fd);
            writeln!(f, "Dropped via sctk").unwrap();
        }
    }

    fn cancelled(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &wayland_client::protocol::wl_data_source::WlDataSource,
    ) {
        self.copy_paste_sources
            .iter()
            .position(|s| s.inner() == source)
            .map(|pos| self.copy_paste_sources.remove(pos));
        source.destroy();
    }

    fn dnd_dropped(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
    ) {
        println!("DROP PERFORMED");
    }

    fn dnd_finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &wayland_client::protocol::wl_data_source::WlDataSource,
    ) {
        self.copy_paste_sources.iter().position(|s| s.inner() == source).map(|pos| {
            self.copy_paste_sources.remove(pos);
        });
        source.destroy();
    }

    fn action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &wayland_client::protocol::wl_data_source::WlDataSource,
        action: wayland_client::protocol::wl_data_device_manager::DndAction,
    ) {
        if let Some(source) = self.drag_sources.iter_mut().find(|s| s.0.inner() == source) {
            source.1 = action.contains(DndAction::Copy);
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

delegate_data_device_manager!(SimpleWindow);
delegate_data_device!(SimpleWindow);
delegate_data_source!(SimpleWindow);
delegate_data_offer!(SimpleWindow);

delegate_registry!(SimpleWindow);

impl ProvidesRegistryState for SimpleWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
