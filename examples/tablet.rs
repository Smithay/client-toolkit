//! An example demonstrating tablets

use std::collections::{HashSet, HashMap};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_pointer,
    delegate_registry, delegate_tablet, delegate_seat, delegate_shm, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        tablet::{
            ToolType,
            ToolCapability,
            TabletState,
            TabletSeatHandler,
            TabletEvent, TabletEventList, TabletHandler,
            TabletToolInitEvent, TabletToolInitEventList,
            TabletToolEventFrame,
            TabletToolEvent, TabletToolHandler,
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
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_pointer, wl_region, wl_seat, wl_shm, wl_surface},
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

    let mut simple_window = SimpleWindow {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor_state: CompositorState::bind(&globals, &qh)
            .expect("wl_compositor not available"),
        shm_state: Shm::bind(&globals, &qh).expect("wl_shm not available"),
        xdg_shell_state: XdgShell::bind(&globals, &qh).expect("xdg shell not available"),
        tablet_state: TabletState::bind(&globals, &qh),

        exit: false,
        width: 256,
        height: 256,
        window: None,
        tablet_seat: None,
        tablets: HashMap::new(),
        tools: HashMap::new(),
        font,
    };

    let surface = simple_window.compositor_state.create_surface(&qh);

    let window =
        simple_window.xdg_shell_state.create_window(surface, WindowDecorations::ServerDefault, &qh);

    window.set_title("A wayland window");
    window.set_app_id("io.github.smithay.client-toolkit.Tablet");
    window.set_min_size(Some((256, 256)));

    window.commit();

    simple_window.window = Some(window);

    while !simple_window.exit {
        event_queue.blocking_dispatch(&mut simple_window).unwrap();
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TabletMetadata {
    /// The descriptive name of the tablet device.
    name: Option<String>,
    /// The USB vendor and product IDs for the tablet device.
    id: Option<(u32, u32)>,
    /// System-specific device paths for the tablet.
    ///
    /// Path format is unspecified.
    /// Clients must figure out what to do with them, if they care.
    paths: Vec<String>,
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
    window: Option<Window>,
    tablet_seat: Option<ZwpTabletSeatV2>,

    tools: HashMap<ZwpTabletToolV2, ToolInfoAndState>,
    tablets: HashMap<ZwpTabletV2, TabletMetadata>,

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
        _capability: Capability,
    ) {
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

impl PointerHandler for SimpleWindow {
    fn pointer_frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if let PointerEventKind::Release { .. } = event.kind {
                self.change_constraint(conn, qh);
            }
        }
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
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        tablet: &ZwpTabletV2,
        events: TabletEventList,
    ) {
        let mut metadata = TabletMetadata::default();
        for event in events {
            match event {
                TabletEvent::Name { name } => metadata.name = Some(name),
                TabletEvent::Id { vid, pid } => metadata.id = Some((vid, pid)),
                TabletEvent::Path { path } => metadata.paths.push(path),
            }
        }
        println!("Tablet {} initialised: {:#?}", tablet.id(), metadata);
        self.tablets.insert(tablet.clone(), metadata);
    }

    fn removed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
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
    pub fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        if let Some(window) = self.window.as_ref() {
            let width = self.width;
            let height = self.height;
            let stride = self.width as i32 * 4;

            let mut pool = SlotPool::new(width as usize * height as usize * 4, &self.shm_state)
                .expect("Failed to create pool");

            let buffer = pool
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Xrgb8888)
                .expect("create buffer")
                .0;

            let mut dt = raqote::DrawTarget::from_backing(
                width as i32,
                height as i32,
                bytemuck::cast_slice_mut(pool.canvas(&buffer).unwrap()),
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
                for (id, tablet_metadata) in &self.tablets {
                    let text = match &tablet_metadata.name {
                        Some(name) => name,
                        None => &*id.id().to_string(),
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
            window.wl_surface().damage_buffer(0, 0, self.width as i32, self.height as i32);

            // Request our next frame
            window.wl_surface().frame(qh, window.wl_surface().clone());

            // Attach and commit to present.
            buffer.attach_to(window.wl_surface()).expect("buffer attach");
            window.wl_surface().commit();
        }
    }
}

delegate_compositor!(SimpleWindow);
delegate_output!(SimpleWindow);
delegate_shm!(SimpleWindow);

delegate_seat!(SimpleWindow);
delegate_pointer!(SimpleWindow);
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
