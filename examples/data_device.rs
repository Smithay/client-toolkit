use std::{
    convert::TryInto,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    os::unix::io::OwnedFd,
    time::Duration,
};

use smithay_client_toolkit::reexports::calloop::{
    EventLoop, LoopHandle, PostAction, RegistrationToken,
};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    data_device_manager::{
        data_device::{DataDevice, DataDeviceHandler},
        data_offer::{DataOfferHandler, DragOffer, SelectionOffer},
        data_source::{CopyPasteSource, DataSourceHandler, DragSource},
        DataDeviceManagerState, WritePipe,
    },
    delegate_compositor, delegate_data_device, delegate_keyboard, delegate_output,
    delegate_pointer, delegate_primary_selection, delegate_registry, delegate_seat, delegate_shm,
    delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    primary_selection::{
        device::{PrimarySelectionDevice, PrimarySelectionDeviceHandler},
        offer::PrimarySelectionOffer,
        selection::{PrimarySelectionSource, PrimarySelectionSourceHandler},
        PrimarySelectionManagerState,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler, BTN_LEFT},
        Capability, SeatHandler, SeatState,
    },
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
use wayland_client::{
    globals::registry_queue_init,
    protocol::{
        wl_data_device::WlDataDevice,
        wl_data_device_manager::DndAction,
        wl_keyboard::{self, WlKeyboard},
        wl_output,
        wl_pointer::{self, WlPointer},
        wl_seat::{self, WlSeat},
        wl_shm, wl_surface,
    },
    Connection, QueueHandle,
};
use wayland_protocols::wp::primary_selection::zv1::client::{
    zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1,
    zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1,
};

fn main() {
    println!(
        "Press c to set the selection, p to set primary selection, or click and drag on \
         the window to drag and drop. Selection contents are printed automatically. Ctrl \
         + click and drag to start an internal drag."
    );
    env_logger::init();

    // All Wayland apps start by connecting the compositor (server).
    let conn = Connection::connect_to_env().unwrap();

    // Enumerate the list of globals to get the protocols the server implements.
    let (globals, event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();
    let mut event_loop: EventLoop<DataDeviceWindow> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn.clone(), event_queue).insert(loop_handle).unwrap();

    // The compositor (not to be confused with the server which is commonly called the compositor) allows
    // configuring surfaces to be presented.
    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    // For desktop platforms, the XDG shell is the standard protocol for creating desktop windows.
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg shell is not available");
    // Since we are not using the GPU in this example, we use wl_shm to allow software rendering to a buffer
    // we share with the compositor process.
    let shm = Shm::bind(&globals, &qh).expect("wl shm is not available.");

    // A window is created from a surface.
    let surface = compositor.create_surface(&qh);
    // And then we can create the window.
    let window = xdg_shell.create_window(surface, WindowDecorations::RequestServer, &qh);
    // Configure the window, this may include hints to the compositor about the desired minimum size of the
    // window, app id for WM identification, the window title, etc.
    window.set_title("A wayland window");
    // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
    window.set_app_id("io.github.smithay.client-toolkit.SimpleWindow");
    window.set_min_size(Some((256, 256)));
    // In order for the window to be mapped, we need to perform an initial commit with no attached buffer.
    // For more info, see WaylandSurface::commit
    //
    // The compositor will respond with an initial configure that we can then use to present to the window with
    // the correct options.
    window.commit();
    let pool = SlotPool::new(256 * 256 * 4, &shm).expect("Failed to create pool");

    // Create primary selection manager state and log if it's not present.
    let primary_selection_manager_state = PrimarySelectionManagerState::bind(&globals, &qh).ok();
    if primary_selection_manager_state.is_none() {
        eprintln!("zwp_primary_selection_v1 is not available.");
    }

    let mut simple_window = DataDeviceWindow {
        compositor,
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm_state: shm,
        data_device_manager_state: DataDeviceManagerState::bind(&globals, &qh)
            .expect("data device manager is not available"),

        primary_selection_manager_state,
        exit: false,
        first_configure: true,
        pool,
        shift: None,
        buffer: None,
        window,
        height: 256,
        width: 256,
        keyboard: None,
        keyboard_focus: false,
        modifiers: Modifiers::default(),
        pointer: None,
        seat_objects: Vec::new(),
        copy_paste_sources: Vec::new(),
        selection_sources: Vec::new(),
        drag_sources: Vec::new(),
        loop_handle: event_loop.handle(),
        accept_counter: 0,
        dnd_offers: Vec::new(),
        selection_offers: Vec::new(),
        primary_selection_offers: Vec::new(),
        drag_surface: None,
    };

    // We don't draw immediately, the configure will notify us when to first draw.

    loop {
        event_loop.dispatch(Duration::from_millis(30), &mut simple_window).unwrap();

        if simple_window.exit {
            println!("exiting example");
            break;
        }
    }
}

struct DataDeviceWindow {
    compositor: CompositorState,
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm_state: Shm,
    data_device_manager_state: DataDeviceManagerState,
    primary_selection_manager_state: Option<PrimarySelectionManagerState>,

    exit: bool,
    first_configure: bool,
    pool: SlotPool,
    width: u32,
    height: u32,
    shift: Option<u32>,
    buffer: Option<Buffer>,
    window: Window,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    modifiers: Modifiers,
    pointer: Option<wl_pointer::WlPointer>,
    dnd_offers: Vec<(DragOffer, Vec<u8>, Option<RegistrationToken>)>,
    selection_offers: Vec<(SelectionOffer, Vec<u8>, Option<RegistrationToken>)>,
    primary_selection_offers: Vec<(PrimarySelectionOffer, Vec<u8>, Option<RegistrationToken>)>,
    seat_objects: Vec<SeatObject>,
    copy_paste_sources: Vec<CopyPasteSource>,
    selection_sources: Vec<PrimarySelectionSource>,
    drag_sources: Vec<(DragSource, bool)>,
    loop_handle: LoopHandle<'static, DataDeviceWindow>,
    accept_counter: u32,
    drag_surface: Option<wl_surface::WlSurface>,
}

impl CompositorHandler for DataDeviceWindow {
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

impl OutputHandler for DataDeviceWindow {
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

impl WindowHandler for DataDeviceWindow {
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
        self.width = configure.new_size.0.map(|w| w.get()).unwrap_or(self.width);
        self.height = configure.new_size.1.map(|h| h.get()).unwrap_or(self.height);
        self.buffer = None;
        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh);
        }
    }
}

impl SeatHandler for DataDeviceWindow {
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
        let seat_object =
            if let Some(seat_object) = self.seat_objects.iter_mut().find(|s| s.seat == seat) {
                seat_object
            } else {
                // create the data device here for this seat
                let data_device_manager = &self.data_device_manager_state;
                let data_device = data_device_manager.get_data_device(qh, &seat);

                let primary_device = self
                    .primary_selection_manager_state
                    .as_ref()
                    .map(|manager| manager.get_selection_device(qh, &seat));
                self.seat_objects.push(SeatObject {
                    seat: seat.clone(),
                    keyboard: None,
                    pointer: None,
                    data_device,
                    primary_device,
                });
                self.seat_objects.last_mut().unwrap()
            };
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard =
                self.seat_state.get_keyboard(qh, &seat, None).expect("Failed to create keyboard");
            self.keyboard = Some(keyboard.clone());
            seat_object.keyboard.replace(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self.seat_state.get_pointer(qh, &seat).expect("Failed to create pointer");
            self.pointer = Some(pointer.clone());
            seat_object.pointer.replace(pointer);
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

impl KeyboardHandler for DataDeviceWindow {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _keysyms: &[Keysym],
    ) {
        if self.window.wl_surface() == surface {
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
        qh: &QueueHandle<Self>,
        kbd: &wl_keyboard::WlKeyboard,
        serial: u32,
        event: KeyEvent,
    ) {
        match event.utf8 {
            Some(s) if s.to_lowercase() == "c" => {
                println!("Creating copy paste source and setting selection...");
                if let Some(data_device) = self.seat_objects.iter().find_map(|seat| {
                    if seat.keyboard.as_ref() == Some(kbd) {
                        Some(&seat.data_device)
                    } else {
                        None
                    }
                }) {
                    let source = self
                        .data_device_manager_state
                        .create_copy_paste_source(qh, SUPPORTED_MIME_TYPES.to_vec());
                    source.set_selection(data_device, serial);
                    self.copy_paste_sources.push(source);
                }
            }
            Some(s) if s.to_lowercase() == "p" => {
                println!("Creating primary selection source and setting selection...");
                if let Some(primary_selection_device) = self.seat_objects.iter().find_map(|seat| {
                    if seat.keyboard.as_ref() == Some(kbd) {
                        seat.primary_device.as_ref()
                    } else {
                        None
                    }
                }) {
                    let source = self
                        .primary_selection_manager_state
                        .as_ref()
                        .unwrap()
                        .create_selection_source(qh, SUPPORTED_MIME_TYPES.to_vec());
                    source.set_selection(primary_selection_device, serial);
                    self.selection_sources.push(source);
                }
            }
            _ => {}
        };
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
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        self.modifiers = modifiers;
    }
}

impl PointerHandler for DataDeviceWindow {
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
            if self.window.wl_surface() != &event.surface {
                continue;
            }
            let surface = event.surface.clone();

            match event.kind {
                Press { button, serial, .. } if button == BTN_LEFT && self.modifiers.ctrl => {
                    if let Some(seat) =
                        self.seat_objects.iter().find(|seat| seat.pointer.as_ref() == Some(pointer))
                    {
                        println!("Starting an internal drag...");
                        DragSource::start_internal_drag(
                            &seat.data_device,
                            self.window.wl_surface(),
                            None,
                            serial,
                        );
                    }
                }
                Press { button, serial, .. } if button == BTN_LEFT => {
                    if let Some(seat) =
                        self.seat_objects.iter().find(|seat| seat.pointer.as_ref() == Some(pointer))
                    {
                        println!("Creating drag and drop source and starting drag...");
                        self.shift = self.shift.xor(Some(0));
                        let source = self.data_device_manager_state.create_drag_and_drop_source(
                            qh,
                            SUPPORTED_MIME_TYPES.to_vec(),
                            DndAction::Copy,
                        );

                        // Create a solid blue surface to use as drag surface
                        let drag_surface = self.compositor.create_surface(qh);
                        let mut pool = SlotPool::new(64 * 64 * 4, &self.shm_state)
                            .expect("Failed to create pool");
                        let (buffer, data) = pool
                            .create_buffer(64, 64, 64 * 4, wl_shm::Format::Argb8888)
                            .expect("create buffer");
                        for i in data.chunks_mut(4) {
                            i.copy_from_slice(&[255, 0, 0, 255]);
                        }
                        buffer.attach_to(&drag_surface).unwrap();
                        drag_surface.damage(0, 0, i32::MAX, i32::MAX);
                        drag_surface.commit();

                        source.start_drag(&seat.data_device, &surface, Some(&drag_surface), serial);
                        self.drag_surface = Some(drag_surface);
                        self.drag_sources.push((source, false));
                    }
                }
                Motion { .. } => {}
                _ => {}
            }
        }
    }
}

impl ShmHandler for DataDeviceWindow {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl DataDeviceWindow {
    pub fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;
        let stride = self.width as i32 * 4;

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
        self.window.wl_surface().damage_buffer(0, 0, self.width as i32, self.height as i32);

        // Request our next frame
        self.window.wl_surface().frame(qh, self.window.wl_surface().clone());

        // Attach and commit to present.
        buffer.attach_to(self.window.wl_surface()).expect("buffer attach");
        self.window.wl_surface().commit();
    }
}

impl DataDeviceHandler for DataDeviceWindow {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        wl_data_device: &WlDataDevice,
        x: f64,
        y: f64,
        _surface: &wl_surface::WlSurface,
    ) {
        println!("Data device enter x: {x:.2} y: {y:.2}");
        let data_device = &self
            .seat_objects
            .iter()
            .find(|seat| seat.data_device.inner() == wl_data_device)
            .unwrap()
            .data_device;

        let Some(drag_offer) = data_device.data().drag_offer() else {
            println!("Internal drag");
            return;
        };

        // Accept the first mime type we support.
        if let Some(mime) = drag_offer.with_mime_types(|mime_types| {
            for mime in mime_types {
                if SUPPORTED_MIME_TYPES.contains(&mime.as_str()) {
                    return Some(mime.clone());
                }
            }

            None
        }) {
            drag_offer.accept_mime_type(0, Some(mime));
        }

        // Accept the action now just in case
        drag_offer.set_actions(DndAction::Copy, DndAction::Copy);
    }

    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _data_device: &WlDataDevice) {
        println!("Data device leave event");
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _wl_data_device: &WlDataDevice,
        x: f64,
        y: f64,
    ) {
        println!("Data Device motion event x: {:.2} y: {:.2}", x, y);
    }

    fn selection(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        wl_data_device: &WlDataDevice,
    ) {
        let data_device = &self
            .seat_objects
            .iter()
            .find(|seat| seat.data_device.inner() == wl_data_device)
            .unwrap()
            .data_device;
        if let Some(offer) = data_device.data().selection_offer() {
            offer.with_mime_types(|mimes| {
                println!("Received selection offer with mime types:");
                for mime in mimes {
                    println!("\t{mime}");
                }
            });

            self.selection_offers.push((offer.clone(), Vec::new(), None));
            let cur_offer = self.selection_offers.last_mut().unwrap();

            let mime_type = match offer.with_mime_types(pick_mime) {
                Some(mime_type) => mime_type,
                None => return,
            };

            let read_pipe = match offer.receive(mime_type) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to receive the offer: {:?}", e);
                    return;
                }
            };
            let cur_offer_ = cur_offer.0.clone();
            if let Ok(token) = self.loop_handle.insert_source(read_pipe, move |_, f, state| {
                let offer = match state.selection_offers.iter().position(|o| o.0 == cur_offer_) {
                    Some(s) => state.selection_offers.remove(s),
                    None => return PostAction::Continue,
                };
                let (offer, mut data, token) = match offer {
                    (o, d, Some(t)) => (o, d, t),
                    _ => return PostAction::Continue,
                };
                // SAFETY: it's safe as long as we don't close the underlying file.
                let f: &mut fs::File = unsafe { f.get_mut() };
                let mut reader = BufReader::new(f);
                let consumed = match reader.fill_buf() {
                    Ok(buf) => {
                        if buf.is_empty() {
                            println!("selection data: {:?}", String::from_utf8(data.clone()));
                            state.selection_offers.push((offer, Vec::new(), None));
                            return PostAction::Remove;
                        } else {
                            data.extend_from_slice(buf);
                            state.selection_offers.push((offer, data, Some(token)));
                        }
                        buf.len()
                    }
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::Interrupted) => {
                        state.selection_offers.push((offer, data, Some(token)));
                        return PostAction::Continue;
                    }
                    Err(e) => {
                        eprintln!("Error reading selection data: {}", e);
                        state.selection_offers.push((offer, Vec::new(), None));

                        return PostAction::Remove;
                    }
                };
                reader.consume(consumed);
                PostAction::Continue
            }) {
                cur_offer.2 = Some(token);
            }
        }
    }

    fn drop_performed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        wl_data_device: &WlDataDevice,
    ) {
        let data_device = &self
            .seat_objects
            .iter()
            .find(|seat| seat.data_device.inner() == wl_data_device)
            .unwrap()
            .data_device;
        if let Some(offer) = data_device.data().drag_offer() {
            println!("Data device dropped event: {offer:?}");
            self.dnd_offers.push((offer.clone(), Vec::new(), None));
            let cur_offer = self.dnd_offers.last_mut().unwrap();
            let mime_type = match offer.with_mime_types(pick_mime) {
                Some(mime) => mime,
                None => return,
            };
            let read_pipe = match cur_offer.0.receive(mime_type.clone()) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to receive the offer: {:?}", e);
                    return;
                }
            };

            self.accept_counter += 1;
            cur_offer.0.accept_mime_type(self.accept_counter, Some(mime_type));
            cur_offer.0.set_actions(DndAction::Copy, DndAction::Copy);
            let cur_offer_ = cur_offer.0.clone();
            match self.loop_handle.insert_source(read_pipe, move |_, f, state| {
                let offer = match state.dnd_offers.iter().position(|o| o.0 == cur_offer_) {
                    Some(s) => state.dnd_offers.remove(s),
                    None => return PostAction::Continue,
                };
                let (offer, mut data, token) = match offer {
                    (o, d, Some(t)) => (o, d, t),
                    _ => return PostAction::Continue,
                };
                // SAFETY: it's safe as long as we don't close the underlying file.
                let f: &mut fs::File = unsafe { f.get_mut() };
                let mut reader = BufReader::new(f);
                let consumed = match reader.fill_buf() {
                    Ok(buf) => {
                        if buf.is_empty() {
                            println!("Dropped data: {:?}", String::from_utf8(data.clone()));
                            offer.finish();
                            offer.destroy();
                            state.dnd_offers.push((offer, Vec::new(), None));
                            return PostAction::Remove;
                        } else {
                            data.extend_from_slice(buf);
                            state.dnd_offers.push((offer, data, Some(token)));
                        }
                        buf.len()
                    }
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::Interrupted) => {
                        state.dnd_offers.push((offer, data, Some(token)));
                        return PostAction::Continue;
                    }
                    Err(e) => {
                        eprintln!("Error reading dropped data: {}", e);
                        offer.finish();
                        offer.destroy();

                        return PostAction::Remove;
                    }
                };
                reader.consume(consumed);
                PostAction::Continue
            }) {
                Ok(token) => {
                    cur_offer.2 = Some(token);
                }
                Err(err) => {
                    eprintln!("{err}");
                    cur_offer.0.finish();
                }
            }
        } else {
            println!("Internal drop performed");
        }
    }
}

impl DataOfferHandler for DataDeviceWindow {
    fn source_actions(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        offer: &mut DragOffer,
        actions: wayland_client::protocol::wl_data_device_manager::DndAction,
    ) {
        println!("Source actions: {actions:?}");
        offer.set_actions(DndAction::Copy, DndAction::Copy);
    }

    fn selected_action(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _offer: &mut DragOffer,
        actions: wayland_client::protocol::wl_data_device_manager::DndAction,
    ) {
        // In this small example there isn't much to do here.
        // Normal applications might track the action and then handling each differently.
        println!("Selected action: {actions:?}");
    }
}

impl DataSourceHandler for DataDeviceWindow {
    fn accept_mime(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
        mime: Option<String>,
    ) {
        println!("Source mime type: {mime:?} was accepted");
    }

    fn send_request(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &wayland_client::protocol::wl_data_source::WlDataSource,
        mime: String,
        write_pipe: WritePipe,
    ) {
        let fd = OwnedFd::from(write_pipe);
        if self
            .copy_paste_sources
            .iter_mut()
            .any(|s| s.inner() == source && SUPPORTED_MIME_TYPES.contains(&mime.as_str()))
        {
            let mut f = File::from(fd);
            writeln!(f, "Copied from selection via sctk").unwrap();
        } else if self
            .drag_sources
            .iter_mut()
            .any(|s| s.0.inner() == source && SUPPORTED_MIME_TYPES.contains(&mime.as_str()) && s.1)
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
        self.drag_sources.retain(|s| s.0.inner() != source);
        self.drag_surface = None;
        source.destroy();
    }

    fn dnd_dropped(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _source: &wayland_client::protocol::wl_data_source::WlDataSource,
    ) {
        println!("Drop performed");
    }

    fn dnd_finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        source: &wayland_client::protocol::wl_data_source::WlDataSource,
    ) {
        println!("Finished");
        self.drag_sources.retain(|s| s.0.inner() != source);
        self.drag_surface = None;
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

impl PrimarySelectionDeviceHandler for DataDeviceWindow {
    fn selection(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        primary_device: &ZwpPrimarySelectionDeviceV1,
    ) {
        let primary_device = self
            .seat_objects
            .iter()
            .find(|seat| seat.primary_device.as_ref().map(|p| p.inner()) == Some(primary_device))
            .unwrap()
            .primary_device
            .as_ref()
            .unwrap();
        if let Some(offer) = primary_device.data().selection_offer() {
            offer.with_mime_types(|mimes| {
                println!("Received primary selection offer with mime types:");
                for mime in mimes {
                    println!("\t{mime}");
                }
            });

            // Add a new offer.
            self.primary_selection_offers.push((offer.clone(), Vec::new(), None));

            let mime_type = match offer.with_mime_types(pick_mime) {
                Some(mime) => mime,
                None => return,
            };

            let read_pipe = match offer.receive(mime_type) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to receive the offer: {:?}", e);
                    return;
                }
            };
            if let Ok(token) = self.loop_handle.insert_source(read_pipe, move |_, f, state| {
                let offer = match state.primary_selection_offers.iter().position(|of| of.0 == offer)
                {
                    Some(s) => state.primary_selection_offers.remove(s),
                    None => return PostAction::Continue,
                };
                let (offer, mut data, token) = match offer {
                    (o, d, Some(t)) => (o, d, t),
                    _ => return PostAction::Continue,
                };
                // SAFETY: it's safe as long as we don't close the underlying file.
                let f: &mut fs::File = unsafe { f.get_mut() };
                let mut reader = BufReader::new(f);
                let consumed = match reader.fill_buf() {
                    Ok(buf) => {
                        if buf.is_empty() {
                            println!(
                                "primary selection data: {:?}",
                                String::from_utf8(data.clone())
                            );
                            state.primary_selection_offers.push((offer, Vec::new(), None));
                            return PostAction::Remove;
                        } else {
                            data.extend_from_slice(buf);
                            state.primary_selection_offers.push((offer, data, Some(token)));
                        }
                        buf.len()
                    }
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::Interrupted) => {
                        state.primary_selection_offers.push((offer, data, Some(token)));
                        return PostAction::Continue;
                    }
                    Err(e) => {
                        eprintln!("Error reading selection data: {}", e);
                        state.primary_selection_offers.push((offer, Vec::new(), None));

                        return PostAction::Remove;
                    }
                };
                reader.consume(consumed);
                PostAction::Continue
            }) {
                self.primary_selection_offers.last_mut().unwrap().2 = Some(token);
            }
        }
    }
}

impl PrimarySelectionSourceHandler for DataDeviceWindow {
    fn send_request(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        source: &ZwpPrimarySelectionSourceV1,
        mime: String,
        write_pipe: WritePipe,
    ) {
        let fd = OwnedFd::from(write_pipe);
        if self
            .selection_sources
            .iter_mut()
            .any(|s| s.inner() == source && SUPPORTED_MIME_TYPES.contains(&mime.as_str()))
        {
            let mut f = File::from(fd);
            writeln!(f, "Copied from primary selection via sctk").unwrap();
        }
    }

    fn cancelled(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        source: &ZwpPrimarySelectionSourceV1,
    ) {
        self.selection_sources.retain(|s| s.inner() == source);
    }
}

impl ProvidesRegistryState for DataDeviceWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

struct SeatObject {
    seat: WlSeat,
    keyboard: Option<WlKeyboard>,
    pointer: Option<WlPointer>,
    data_device: DataDevice,
    primary_device: Option<PrimarySelectionDevice>,
}

delegate_compositor!(DataDeviceWindow);
delegate_output!(DataDeviceWindow);
delegate_shm!(DataDeviceWindow);

delegate_seat!(DataDeviceWindow);
delegate_keyboard!(DataDeviceWindow);
delegate_pointer!(DataDeviceWindow);

delegate_xdg_shell!(DataDeviceWindow);
delegate_xdg_window!(DataDeviceWindow);

delegate_data_device!(DataDeviceWindow);

delegate_primary_selection!(DataDeviceWindow);

delegate_registry!(DataDeviceWindow);

const SUPPORTED_MIME_TYPES: &[&str; 6] = &[
    "text/plain;charset=utf-8",
    "text/plain;charset=UTF-8",
    "UTF8_STRING",
    "STRING",
    "text/plain",
    "TEXT",
];
fn pick_mime(mime_types: &[String]) -> Option<String> {
    for mime in mime_types {
        if SUPPORTED_MIME_TYPES.contains(&mime.as_str()) {
            return Some(mime.clone());
        }
    }

    None
}
