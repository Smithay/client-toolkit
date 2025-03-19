//! An example demonstrating tablets

use std::collections::{HashSet, HashMap};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_keyboard,
    delegate_registry, delegate_tablet, delegate_seat, delegate_shm, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        tablet::{
            TabletState,
            seat::TabletSeatHandler,
            tablet::{TabletHandler, TabletDescription},
            tool::{
                ToolType, ToolCapability,
                TabletToolInitEvent, TabletToolInitEventList,
                TabletToolEventFrame,
                TabletToolEvent, TabletToolHandler,
            },
        },
        Capability, SeatHandler, SeatState,
    },
    shell::{
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{slot::{SlotPool, Buffer}, Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_keyboard, wl_region, wl_seat, wl_shm, wl_surface},
    Connection, Dispatch, QueueHandle,
    Proxy,
};
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_seat_v2::ZwpTabletSeatV2,
    zwp_tablet_tool_v2::ZwpTabletToolV2,
    zwp_tablet_v2::ZwpTabletV2,
    // zwp_tablet_pad_v2::ZwpTabletPadV2,
    // zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2,
    // zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2,
    // zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2,
};

const RED: raqote::SolidSource = raqote::SolidSource { r: 221, g: 0, b: 0, a: 255 };
const GREEN: raqote::SolidSource = raqote::SolidSource { r: 0, g: 170, b: 0, a: 255 };
const SOLID_RED: raqote::Source = raqote::Source::Solid(RED);
const SOLID_GREEN: raqote::Source = raqote::Source::Solid(GREEN);

const WHITE: raqote::SolidSource = raqote::SolidSource { r: 255, g: 255, b: 255, a: 255 };
const BLACK: raqote::SolidSource = raqote::SolidSource { r: 0, g: 0, b: 0, a: 255 };
const SOLID_BLACK: raqote::Source = raqote::Source::Solid(BLACK);

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let font = font_kit::source::SystemSource::new()
        .select_best_match(
            &[font_kit::family_name::FamilyName::SansSerif],
            &font_kit::properties::Properties::new(),
        )
        .unwrap()
        .load()
        .unwrap();

    let compositor_state = CompositorState::bind(&globals, &qh)
                    .expect("wl_compositor not available");
    let shm_state = Shm::bind(&globals, &qh).expect("wl_shm not available");
    let xdg_shell_state = XdgShell::bind(&globals, &qh).expect("xdg shell not available");

    let surface = compositor_state.create_surface(&qh);

    let window = xdg_shell_state.create_window(surface, WindowDecorations::ServerDefault, &qh);

    window.set_title("A wayland window");
    window.set_app_id("io.github.smithay.client-toolkit.Tablet");
    window.set_min_size(Some((256, 256)));

    window.commit();

    let width = 256;
    let height = 256;
    // Initial size, but it grows automatically as needed.
    let pool = SlotPool::new(width as usize * height as usize * 4, &shm_state)
        .expect("Failed to create pool");

    let mut simple_window = SimpleWindow {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor_state,
        shm_state,
        xdg_shell_state,
        tablet_state: TabletState::bind(&globals, &qh),

        exit: false,
        width,
        height,
        window,
        keyboard: None,
        keyboard_focus: false,
        tablet_seat: None,
        tablets: HashMap::new(),
        tools: HashMap::new(),
        pool,
        mode: Mode::Points,
        font,
    };

    while !simple_window.exit {
        event_queue.blocking_dispatch(&mut simple_window).unwrap();
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Button {
    serial: u32,
    button: u32,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TabletToolCapabilities {
    tilt: bool,
    pressure: bool,
    distance: bool,
    rotation: bool,
    slider: bool,
    wheel: bool,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TabletToolInfo {
    tool_type: ToolType,
    hardware_serial: Option<(u32, u32)>,
    hardware_id_wacom: Option<(u32, u32)>,
    capabilities: TabletToolCapabilities,
}

/// The current state of the tool.
///
/// This covers everything, and may, for some applications,
/// be the most practical way of perceiving it;
/// but button is more likely to be desired as events,
/// and wheel is fundamentally a delta thing,
/// so for at least them you probably want to consume the events.
///
/// Also you won’t get the last frame’s time, if you view it this way,
/// as a proximity_out event deletes the state.
#[derive(Debug)]
struct TabletToolState {
    // ProximityIn
    serial: u32,
    tablet: ZwpTabletV2,
    surface: wl_surface::WlSurface,
    // Down (cleared on Up), stores serial
    down: Option<u32>,
    // Motion
    x: f64,
    y: f64,
    // Pressure
    pressure: u16,
    // Distance
    distance: u16,
    // Tilt
    tilt_x: f64,
    tilt_y: f64,
    // Rotation
    rotation_degrees: f64,
    // Slider
    slider_position: i32,
    // Wheel
    wheel_degrees: f64,
    wheel_clicks: i32,
    // Button
    buttons: HashSet<Button>,
}

struct ToolInfoAndState {
    /// Static info about the tool and its capabilities.
    info: TabletToolInfo,
    /// The time the last frame was sent,
    /// or zero if no frames have come yet.
    last_frame_time: u32,
    /// The current state of the tool, if in proximity.
    state: Option<TabletToolState>,
}

impl ToolInfoAndState {
    /// Get the pressure according to the Web Pointer Events API:
    /// scaled in the range \[0, 1\],
    /// and set to 0.5 when down if pressure isn’t supported.
    fn pressure_web(&self) -> f64 {
        match (self.info.capabilities.pressure, &self.state) {
            (true, &Some(TabletToolState { pressure, .. })) => pressure as f64 / 65535.0,
            (false, Some(TabletToolState { down: Some(_), .. })) => 0.5,
            _ => 0.0,
        }
    }
}

struct SimpleWindow {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    xdg_shell_state: XdgShell,
    tablet_state: TabletState,

    exit: bool,
    width: u32,
    height: u32,
    window: Window,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    tablet_seat: Option<ZwpTabletSeatV2>,
    tablets: HashMap<ZwpTabletV2, TabletDescription>,
    tools: HashMap<ZwpTabletToolV2, ToolInfoAndState>,
    pool: SlotPool,
    mode: Mode,

    font: font_kit::loaders::freetype::Font,
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
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        self.width = configure.new_size.0.map(|v| v.get()).unwrap_or(256);
        self.height = configure.new_size.1.map(|v| v.get()).unwrap_or(256);
        if self.mode.is_sketch() {
            self.reset_sketch_mode();
        }
        self.draw(conn, qh);
    }
}

impl SeatHandler for SimpleWindow {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
        // TODO: I would have thought tablet seat initialisation should happen here,
        // but this doesn’t seem to be called?
        // I’m not at all sure I’m structuring this the right way.
        panic!("I thought new_seat didn’t get called?");
    }

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
                .get_keyboard(
                    qh,
                    &seat,
                    None,
                )
                .expect("Failed to create keyboard");

            self.keyboard = Some(keyboard);
        }
        if self.tablet_seat.is_none() {
            let tablet_seat = self.tablet_state.get_tablet_seat(&seat, qh).ok();
            if tablet_seat.is_some() {
                println!("Created tablet seat");
            } else {
                println!("Compositor does not support tablet events");
            }
            self.tablet_seat = tablet_seat;
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        // TODO: do we need to release tablet_seat, or will it sort itself out?
    }
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
        _: &[Keysym],
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
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        match event.keysym {
            Keysym::n => self.toggle_mode(),
            Keysym::r if self.mode.is_sketch() => self.reset_sketch_mode(),
            _ => (),
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
        _: u32,
        _: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
    }
}

impl TabletSeatHandler for SimpleWindow {
    fn tablet_added(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _tablet_seat: &ZwpTabletSeatV2,
        _seat: &wl_seat::WlSeat,
        tablet: ZwpTabletV2,
    ) {
        println!("Added tablet: {}", tablet.id());
    }

    fn tool_added(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _tablet_seat: &ZwpTabletSeatV2,
        _seat: &wl_seat::WlSeat,
        tool: ZwpTabletToolV2,
    ) {
        println!("Added tablet tool: {}", tool.id());
    }

    // fn pad_added(
    //     &mut self,
    //     _conn: &Connection,
    //     _qh: &QueueHandle<Self>,
    //     _tablet_seat: &ZwpTabletSeatV2,
    //     _seat: &wl_seat::WlSeat,
    //     pad: ZwpTabletPadV2,
    // ) {
    //     println!("Added tablet pad: {}", pad.id());
    // }
}

impl TabletHandler for SimpleWindow {
    fn init_done(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        tablet: &ZwpTabletV2,
        description: TabletDescription,
    ) {
        println!("Tablet {} initialised: {:#?}", tablet.id(), description);
        self.tablets.insert(tablet.clone(), description);
    }

    fn removed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        tablet: &ZwpTabletV2,
    ) {
        println!("Tablet {} removed", tablet.id());
        self.tablets.remove(tablet);
    }
}

impl TabletToolHandler for SimpleWindow {
    fn init_done(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        tablet_tool: &ZwpTabletToolV2,
        events: TabletToolInitEventList,
    ) {
        let mut tool_type = None;
        let mut hardware_serial = None;
        let mut hardware_id_wacom = None;
        let mut capabilities = TabletToolCapabilities {
            tilt: false,
            pressure: false,
            distance: false,
            rotation: false,
            slider: false,
            wheel: false,
        };
        for event in events {
            match event {
                TabletToolInitEvent::Type { tool_type: t } => tool_type = Some(t),
                TabletToolInitEvent::HardwareSerial { hardware_serial_hi, hardware_serial_lo } => hardware_serial = Some((hardware_serial_hi, hardware_serial_lo)),
                TabletToolInitEvent::HardwareIdWacom { hardware_id_hi, hardware_id_lo } => hardware_id_wacom = Some((hardware_id_hi, hardware_id_lo)),
                TabletToolInitEvent::Capability { capability: ToolCapability::Tilt } => capabilities.tilt = true,
                TabletToolInitEvent::Capability { capability: ToolCapability::Pressure } => capabilities.pressure = true,
                TabletToolInitEvent::Capability { capability: ToolCapability::Distance } => capabilities.distance = true,
                TabletToolInitEvent::Capability { capability: ToolCapability::Rotation } => capabilities.rotation = true,
                TabletToolInitEvent::Capability { capability: ToolCapability::Slider } => capabilities.slider = true,
                TabletToolInitEvent::Capability { capability: ToolCapability::Wheel } => capabilities.wheel = true,
                TabletToolInitEvent::Capability { capability: _ } => (),
            }
        }
        let info = TabletToolInfo {
            tool_type: tool_type.expect("zwp_tablet_tool_v2.type event missing"),
            hardware_serial,
            hardware_id_wacom,
            capabilities,
        };
        println!("Tablet tool {} initialised: {:#?}", tablet_tool.id(), info);
        self.tools.insert(tablet_tool.clone(), ToolInfoAndState {
            info,
            last_frame_time: 0,
            state: None,
        });
    }

    fn removed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        tablet_tool: &ZwpTabletToolV2,
    ) {
        println!("Tablet tool {} removed", tablet_tool.id());
    }

    fn tablet_tool_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        tablet_tool: &ZwpTabletToolV2,
        frame: TabletToolEventFrame,
    ) {
        println!("Tablet tool {} frame: {:#?}", tablet_tool.id(), frame);
        let TabletToolEventFrame { time, events } = frame;
        let mut events = events.into_iter();
        let tias = self.tools.get_mut(tablet_tool).unwrap();
        let state = tias.state.get_or_insert_with(|| {
            let Some(TabletToolEvent::ProximityIn { serial, tablet, surface }) = events.next()
            else {
                panic!("First zwp_tablet_tool_v2 frame didn’t start with a proximity_in event");
            };
            TabletToolState {
                // ProximityIn
                serial,
                tablet,
                surface,
                // Down (cleared on Up)
                down: None,
                // Motion
                x: 0.0,
                y: 0.0,
                // Pressure
                pressure: 0,
                // Distance
                distance: 0,
                // Tilt
                tilt_x: 0.0,
                tilt_y: 0.0,
                // Rotation
                rotation_degrees: 0.0,
                // Slider
                slider_position: 0,
                // Wheel
                wheel_degrees: 0.0,
                wheel_clicks: 0,
                // Button
                buttons: HashSet::new(),
            }
        });
        tias.last_frame_time = time;

        for event in events {
            match event {
                TabletToolEvent::ProximityIn { serial, tablet, surface } => {
                    state.serial = serial;
                    state.tablet = tablet;
                    state.surface = surface;
                },
                TabletToolEvent::ProximityOut => {
                    tias.state = None;
                    // Given that a frame is supposed to represent a single hardware event,
                    // I think you can fairly say it’d be mad to proximity_out and
                    // immediately proximity_in in the same frame.
                    // So I think we’re OK to just break.
                    break;
                },
                TabletToolEvent::Down { serial } => {
                    state.down = Some(serial);
                },
                TabletToolEvent::Up => {
                    state.down = None;
                },
                TabletToolEvent::Motion { x, y } => {
                    state.x = x;
                    state.y = y;
                },
                TabletToolEvent::Pressure { pressure } => {
                    state.pressure = pressure;
                },
                TabletToolEvent::Distance { distance } => {
                    state.distance = distance;
                },
                TabletToolEvent::Tilt { tilt_x, tilt_y } => {
                    state.tilt_x = tilt_x;
                    state.tilt_y = tilt_y;
                },
                TabletToolEvent::Rotation { degrees } => {
                    state.rotation_degrees = degrees;
                },
                TabletToolEvent::Slider { position } => {
                    state.slider_position = position;
                },
                TabletToolEvent::Wheel { degrees, clicks } => {
                    // These ones use += because they’re deltas, unlike the rest.
                    state.wheel_degrees += degrees;
                    state.wheel_clicks += clicks;
                },
                TabletToolEvent::Button { serial, button, pressed } => {
                    if pressed {
                        state.buttons.insert(Button { serial, button });
                    } else {
                        state.buttons.remove(&Button { serial, button });
                    }
                },
            }
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
        match self.mode {
            Mode::Points => self.draw_point(conn, qh),
            Mode::Sketch { .. } => self.draw_sketch(conn, qh),
        }
    }

    pub fn draw_point(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;
        let stride = self.width as i32 * 4;

        let buffer = self.pool
            .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Xrgb8888)
            .expect("create buffer")
            .0;

        let mut dt = raqote::DrawTarget::from_backing(
            width as i32,
            height as i32,
            bytemuck::cast_slice_mut(self.pool.canvas(&buffer).unwrap()),
        );
        dt.clear(WHITE);
        let mut y = 16.;
        if self.tablets.is_empty() {
            dt.draw_text(
                &self.font,
                14.,
                "No tablets found",
                raqote::Point::new(2., y),
                &SOLID_RED,
                &raqote::DrawOptions::new(),
            );
        } else {
            for (tablet, description) in &self.tablets {
                let text = match &description.name {
                    Some(name) => name,
                    None => &*tablet.id().to_string(),
                };
                dt.draw_text(
                    &self.font,
                    14.,
                    text,
                    raqote::Point::new(2., y),
                    &SOLID_BLACK,
                    &raqote::DrawOptions::new(),
                );
                y += 16.0;
            }
        }

        for tias in self.tools.values() {
            if let Some(state) = &tias.state {
                let mut pb = raqote::PathBuilder::new();
                let pressure = tias.pressure_web();
                pb.arc(
                    state.x as f32,// * self.width as f32,
                    state.y as f32,// * self.height as f32,
                    10.0 + 30.0 * pressure as f32,
                    0.,
                    2. * std::f32::consts::PI,
                );
                pb.close();
                dt.fill(
                    &pb.finish(),
                    &if state.down.is_some() { SOLID_GREEN } else { SOLID_RED },
                    &raqote::DrawOptions::new(),
                );
            }
        }

        // Damage the entire window
        let surface = self.window.wl_surface();
        surface.damage_buffer(0, 0, self.width as i32, self.height as i32);

        // Request our next frame
        surface.frame(qh, surface.clone());

        // Attach and commit to present.
        buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
    }

    fn draw_sketch(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        let Mode::Sketch { buffer, busy_buffer } = &mut self.mode
        else { unreachable!() };

        println!("draw_sketch");
        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                // I don’t know if I’m using this wrong,
                // but I want to copy from one buffer to another buffer,
                // which I understand is allowed (see below),
                // and I suppose they should both go in the same SlotPool,
                // but the use of &mut prevents me accessing both at once,
                // and… and… and… meh, I’m just going to do something stupid for now.
                // TODO find out from someone who actually knows what they’re doing,
                // what the appropriate solution is.
                let old_canvas = unsafe {
                    std::mem::transmute::<&'_ mut [u8], &'static [u8]>(
                        self.pool.raw_data_mut(&buffer.slot())
                    )
                };

                // This should be rare
                // (TODO: the rest of this ssentence is found in other examples here,
                // but I seem to hit it immediatley in Sway, so I dunno if it’s even true),
                // but if the compositor has not released the previous
                // buffer, we need double-buffering.
                let (mut second_buffer, canvas) = self
                    .pool
                    .create_buffer(
                        self.width as i32,
                        self.height as i32,
                        self.width as i32 * 4,
                        wl_shm::Format::Xrgb8888,
                    )
                    .expect("create buffer");
                // Now, we copy from the busy buffer to the new one.
                // <https://lists.freedesktop.org/archives/wayland-devel/2020-June/041490.html#:~:text=you%20can%20play%20tricks,readbacks%20may%20have%20issues>
                // tells me that this is safe, but that there may be issues.
                // I’m out of my depth here.
                // Maybe that’s why there isn’t a &Buffer → &[u8] method?
                // Anyway, for now I’ll just do this and hope for the best.
                canvas.copy_from_slice(old_canvas);
                // And we swap them. We’ll later need to mark the damage
                // regions that the busy buffer needs to copy back from this one.
                std::mem::swap(buffer, &mut second_buffer);
                *busy_buffer = Some((second_buffer, vec![]));
                // … but y’know what? For now I’m just going to trash it,
                // instead of implementing that.
                // Consider this a stub for a possibly bad idea.
                *busy_buffer = None;
                canvas
            }
        };
        let mut dt = raqote::DrawTarget::from_backing(
            self.width as i32,
            self.height as i32,
            //bytemuck::cast_slice_mut(self.pool.canvas(&buffer).unwrap()),
            bytemuck::cast_slice_mut(canvas),
        );

        let surface = self.window.wl_surface();

        for tool in self.tools.values() {
            if let Some(state @ TabletToolState { down: Some(_), .. }) = &tool.state {
                let mut pb = raqote::PathBuilder::new();
                let pressure = tool.pressure_web();

                let radius = 10.0 * pressure as f32;
                pb.arc(
                    state.x as f32,
                    state.y as f32,
                    radius,
                    0.,
                    2. * std::f32::consts::PI,
                );
                pb.close();
                dt.fill(
                    &pb.finish(),
                    &raqote::Source::Solid(raqote::SolidSource {
                        r: (state.tilt_x + 90.0 / 180.0 * 255.0) as u8,
                        g: (state.tilt_y + 90.0 / 180.0 * 255.0) as u8,
                        b: (state.rotation_degrees / 360.0 * 255.0 % 255.0) as u8,
                        a: if tool.info.capabilities.slider {
                            ((state.slider_position + 65535) as f64 / 131071.0 * 255.0) as u8
                        } else {
                            // Sure, 0 is the neutraal position and all that,
                            // but semitransparent looks a bit odd,
                            // so sans slider support, we’ll just go opaque.
                            // (b being 0 if rotation is not supported is fine.)
                            255
                        },
                    }),
                    &raqote::DrawOptions::new(),
                );

                surface.damage_buffer(
                    ((state.x as f32) - radius).floor() as i32,
                    ((state.y as f32) - radius).floor() as i32,
                    ((state.x as f32) + radius).ceil() as i32,
                    ((state.y as f32) + radius).ceil() as i32,
                );
            }
        }

        // Request our next frame
        surface.frame(qh, surface.clone());

        // Attach and commit to present.
        buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
    }

    fn reset_sketch_mode(&mut self) {
        let buffer = self.pool
            .create_buffer(
                self.width as i32,
                self.height as i32,
                self.width as i32 * 4,
                wl_shm::Format::Xrgb8888,
            )
            .expect("create buffer")
            .0;

        // Now we actually clear the buffer, making everything opaque white.
        for x in buffer.canvas(&mut self.pool).unwrap() {
            *x = 255;
        }
        self.window.wl_surface().damage_buffer(0, 0, i32::MAX, i32::MAX);
        self.mode = Mode::Sketch { buffer, busy_buffer: None };
    }

    fn toggle_mode(&mut self) {
        println!("Switching mode to {:?}", self.mode);
        match self.mode {
            Mode::Points => self.reset_sketch_mode(),
            Mode::Sketch { .. } => self.mode = Mode::Points,
        }
    }
}

#[derive(Debug)]
pub enum Mode {
    Points,
    Sketch {
        /// The buffer that gets drawn to and attached to the surface.
        buffer: Buffer,
        /// If the first buffer is not released, we swap to this buffer.
        /// The buffer that gets copied to after drawing, and 
        busy_buffer: Option<(Buffer, Vec<(i32, i32, i32, i32)>)>,
    },
}

impl Mode {
    fn is_points(&self) -> bool {
        matches!(self, Mode::Points)
    }

    fn is_sketch(&self) -> bool {
        matches!(self, Mode::Sketch { .. })
    }
}

delegate_compositor!(SimpleWindow);
delegate_output!(SimpleWindow);
delegate_shm!(SimpleWindow);

delegate_seat!(SimpleWindow);
delegate_keyboard!(SimpleWindow);
delegate_tablet!(SimpleWindow);

delegate_xdg_shell!(SimpleWindow);
delegate_xdg_window!(SimpleWindow);

delegate_registry!(SimpleWindow);

impl ProvidesRegistryState for SimpleWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

impl Dispatch<wl_region::WlRegion, ()> for SimpleWindow {
    fn event(
        _: &mut Self,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<SimpleWindow>,
    ) {
    }
}
