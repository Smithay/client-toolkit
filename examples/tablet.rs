//! An example demonstrating tablets

use std::collections::HashMap;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Surface},
    delegate_compositor, delegate_output, delegate_keyboard,
    delegate_registry, delegate_tablet, delegate_seat, delegate_shm, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        tablet_seat,
        tablet,
        tablet_tool,
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

const TWO_PI: f32 = 2. * std::f32::consts::PI;

const BLACK: raqote::Source = raqote::Source::Solid(raqote::SolidSource { r: 0, g: 0, b: 0, a: 255 });
const WHITE: raqote::Source = raqote::Source::Solid(raqote::SolidSource { r: 255, g: 255, b: 255, a: 255 });
const DARK_GREEN: raqote::Source = raqote::Source::Solid(raqote::SolidSource { r: 0, g: 102, b: 0, a: 255 });
const DARK_RED: raqote::Source = raqote::Source::Solid(raqote::SolidSource { r: 153, g: 0, b: 0, a: 255 });
const HALF_WHITE: raqote::Source = raqote::Source::Solid(raqote::SolidSource { r: 127, g: 127, b: 127, a: 127 });

const NO_TIME: &'static str = "               ";

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)
                    .expect("wl_compositor not available");
    let shm_state = Shm::bind(&globals, &qh).expect("wl_shm not available");
    let xdg_shell_state = XdgShell::bind(&globals, &qh).expect("xdg shell not available");

    let surface = compositor_state.create_surface(&qh);

    let window = xdg_shell_state.create_window(surface, WindowDecorations::ServerDefault, &qh);

    window.set_title("Tablet drawing");
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
        _xdg_shell_state: xdg_shell_state,

        exit: false,
        width,
        height,
        window,
        keyboard: None,
        keyboard_focus: false,
        tablet_seat: None,
        tablets: HashMap::new(),
        tools: HashMap::new(),
        buffer: None,
        queued_circles: Vec::new(),
        redraw_queued: false,
        pool,
    };

    while !simple_window.exit {
        event_queue.blocking_dispatch(&mut simple_window).unwrap();
    }
}

struct Tool {
    info: tablet_tool::Info,
    state: tablet_tool::State,
    cursor_surface: Option<Surface>,
}

struct SimpleWindow {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    _xdg_shell_state: XdgShell,

    exit: bool,
    width: u32,
    height: u32,
    window: Window,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    tablet_seat: Option<ZwpTabletSeatV2>,
    tablets: HashMap<ZwpTabletV2, tablet::Info>,
    tools: HashMap<ZwpTabletToolV2, Tool>,
    pool: SlotPool,
    buffer: Option<Buffer>,
    queued_circles: Vec<Circle>,
    redraw_queued: bool,
}

struct Circle {
    x: f32,
    y: f32,
    radius: f32,
    color: raqote::SolidSource,
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
        surface: &wl_surface::WlSurface,
        time: u32,
    ) {
        println!("[33m[t={time:10}][m [35mdraw[m Frame callback, {} circles to draw", self.queued_circles.len());
        if surface == self.window.wl_surface() {
            self.redraw_queued = false;
            self.draw_cursors(conn, qh);
            self.draw(conn, qh, false);
        }
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
        let new_width = configure.new_size.0.map(|v| v.get()).unwrap_or(256);
        let new_height = configure.new_size.1.map(|v| v.get()).unwrap_or(256);
        if self.width != new_width || self.height != new_height || self.buffer.is_none() {
            self.width = new_width;
            self.height = new_height;
            self.init_canvas();
        }
        self.draw(conn, qh, true);
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
            self.keyboard = self.seat_state.get_keyboard(qh, &seat, None).ok();
        }
        // FIXME: this doesn’t seem like the right place to put this.
        // Where *should* it go?
        if self.tablet_seat.is_none() {
            self.tablet_seat = self.seat_state.get_tablet_seat(qh, &seat).ok();
            if self.tablet_seat.is_some() {
                println!("[35mCreated tablet_seat[m");
            } else {
                println!("[31mCompositor does not support tablet events[m");
            }
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
            Keysym::Delete => {
                self.clear_canvas();
                self.queued_circles.clear();
                if let Some(buffer) = &self.buffer {
                    let surface = self.window.wl_surface();
                    buffer.attach_to(surface).expect("buffer attach");
                    surface.commit();
                }
            },
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

impl tablet_seat::Handler for SimpleWindow {}

impl tablet::Handler for SimpleWindow {
    fn info(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        tablet: &ZwpTabletV2,
        info: tablet::Info,
    ) {
        println!("{NO_TIME}[36mtablet.done[m {}: {:?}", tablet.id(), info);
        self.tablets.insert(tablet.clone(), info);
    }

    fn removed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        tablet: &ZwpTabletV2,
    ) {
        println!("{NO_TIME}[32mtablet.removed[m {}", tablet.id());
        self.tablets.remove(tablet);
    }
}

impl tablet_tool::Handler for SimpleWindow {
    fn info(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        wtool: &ZwpTabletToolV2,
        info: tablet_tool::Info,
    ) {
        println!("{NO_TIME}[36mtablet_tool.done[m {}: {:?}", wtool.id(), info);
        self.tools.insert(wtool.clone(), Tool {
            info,
            state: tablet_tool::State::new(),
            cursor_surface: None,
        });
    }

    fn removed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        wtool: &ZwpTabletToolV2,
    ) {
        println!("{NO_TIME}[36mtablet_tool.removed[m {}", wtool.id());
        self.tools.remove(wtool);
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        wtool: &ZwpTabletToolV2,
        events: &[tablet_tool::Event],
    ) {
        let tool = self.tools.get_mut(wtool).expect("got frame for unknown tool");
        tool.state.ingest_frame(events);

        print!("[33m[t={:10}][m [32mtablet_tool.frame[m ", tool.state.time);
        if tool.state.is_in_proximity() {
            if tool.state.is_down() {
                let pressure = tool.state.pressure_web(&tool.info);
                let radius = 2.0 * pressure as f32;
                self.queued_circles.push(Circle {
                    x: tool.state.x as f32,
                    y: tool.state.y as f32,
                    radius,
                    color: raqote::SolidSource {
                        r: (tool.state.tilt_x + 90.0 / 180.0 * 255.0) as u8,
                        g: (tool.state.tilt_y + 90.0 / 180.0 * 255.0) as u8,
                        b: (tool.state.rotation_degrees / 360.0 * 255.0 % 255.0) as u8,
                        a: if tool.info.supports_slider() {
                            ((tool.state.slider_position + 65535) as f64 / 131071.0 * 255.0) as u8
                        } else {
                            // Sure, 0 is the neutraal position and all that,
                            // but semitransparent looks a bit odd,
                            // so sans slider support, we’ll just go opaque.
                            // (b being 0 if rotation is not supported is fine.)
                            255
                        },
                    },
                });
            }

            print!("{} x={:7.2} y={:7.2}",
                if tool.state.is_down() { "down" } else { "up  " },
                tool.state.x,
                tool.state.y);
            if tool.info.supports_pressure() {
                print!(" pressure={:5}", tool.state.pressure);
            }
            if tool.info.supports_tilt() {
                print!(" tilt_x={:5.2} tilt_y={:5.2}", tool.state.tilt_x, tool.state.tilt_y);
            }
            if tool.info.supports_distance() {
                print!(" distance={:5}", tool.state.distance);
            }
            if tool.info.supports_rotation() {
                print!(" rotation={:6.2}", tool.state.rotation_degrees);
            }
            if tool.info.supports_slider() {
                print!(" slider={:6}", tool.state.slider_position);
            }
            if tool.info.supports_wheel() {
                print!(" wheel={:6.2}", tool.state.wheel_degrees);
            }
            if tool.state.stylus_button_1_pressed {
                print!(" button:1");
            }
            if tool.state.stylus_button_2_pressed {
                print!(" button:2");
            }
            if tool.state.stylus_button_3_pressed {
                print!(" button:3");
            }
            println!();
        } else {
            println!("left proximity");
        }

        // Even if the main window has nothing to redraw,
        // the cursors probably do,
        // and we’re doing only coarse reactivity here,
        // so just queue a general redraw.
        self.queue_redraw(qh);
    }
}

impl ShmHandler for SimpleWindow {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl SimpleWindow {
    fn queue_redraw(&mut self, qh: &QueueHandle<Self>) {
        if !self.redraw_queued {
            let surface = self.window.wl_surface();
            // In theory, it might be better to do frame callbacks on cursor surfaces; don’t know.
            // But in practice, doing it on the window surface is plenty good enough.
            surface.frame(qh, surface.clone());
            // Have to commit to make the frame request.
            surface.commit();
            self.redraw_queued = true;
        }
    }

    pub fn draw(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, force: bool) {
        if self.queued_circles.is_empty() && !force {
            println!("{NO_TIME}[35mdraw[m Nothing to draw in the window");
            // Nothing needs updating.
            // (It was presumably the cursors needing to be updated.)
            return;
        }

        let buffer = self.buffer.as_ref().unwrap();
        let canvas = self.pool.canvas(buffer).expect("buffer is still active");
        let mut dt = raqote::DrawTarget::from_backing(
            self.width as i32,
            self.height as i32,
            bytemuck::cast_slice_mut(canvas),
        );

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for circle in self.queued_circles.drain(..) {
            let mut pb = raqote::PathBuilder::new();
            pb.arc(
                circle.x,
                circle.y,
                circle.radius,
                0.,
                TWO_PI,
            );
            pb.close();
            dt.fill(
                &pb.finish(),
                &raqote::Source::Solid(circle.color),
                &raqote::DrawOptions::new(),
            );
            min_x = min_x.min((circle.x - circle.radius).floor() as i32);
            min_y = min_y.min((circle.y - circle.radius).floor() as i32);
            max_x = max_x.max((circle.x + circle.radius).ceil() as i32);
            max_y = max_y.max((circle.y + circle.radius).ceil() as i32);
        }

        let surface = self.window.wl_surface();
        if let (Some(width), Some(height)) = (max_x.checked_sub(min_x), max_y.checked_sub(min_y)) {
            surface.damage_buffer(min_x, min_y, width, height);
        }
        buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
        println!("{NO_TIME}[35mdraw[m Finished drawing frame");
    }

    pub fn draw_cursors(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        for (wtool, tool) in &mut self.tools {
            let Some(tablet_tool::Proximity { serial: proximity_in_serial, .. }) = tool.state.proximity
            else { continue };
            let width = 58;
            let height = 33;
            let (buffer, canvas) = self.pool.create_buffer(
                width as i32,
                height as i32,
                width as i32 * 4,
                wl_shm::Format::Argb8888
            ).expect("create buffer");
            // https://github.com/Smithay/client-toolkit/issues/488 workaround.
            let canvas = &mut canvas[..width as usize * height as usize * 4];

            let mut dt = raqote::DrawTarget::from_backing(
                width as i32,
                height as i32,
                bytemuck::cast_slice_mut(canvas),
            );
            let o = &raqote::DrawOptions::new();
            dt.clear(raqote::SolidSource { r: 0, g: 0, b: 0, a: 0 });

            // Draw crosshairs, varyinig with pressure and contact state.
            {
                let mut pb = raqote::PathBuilder::new();
                let radius = 4.0 + 4.0 * tool.state.pressure_web(&tool.info) as f32;
                pb.move_to(16.5         ,  1.5         );
                pb.line_to(16.5         , 16.5 - radius);
                pb.move_to(16.5         , 16.5 + radius);
                pb.line_to(16.5         , 31.5         );
                pb.move_to( 1.5         , 16.5         );
                pb.line_to(16.5 - radius, 16.5         );
                pb.move_to(16.5 + radius, 16.5         );
                pb.line_to(31.5         , 16.5         );
                pb.arc(16.5, 16.5, radius, 0.0, TWO_PI);
                let path = pb.finish();
                let mut stroke_style = raqote::StrokeStyle {
                    width: 3.0,
                    cap: raqote::LineCap::Square,
                    ..Default::default()
                };
                dt.stroke(&path, &HALF_WHITE, &stroke_style, o);
                stroke_style.width = 1.0;
                dt.stroke(&path, &if tool.state.is_down() { DARK_GREEN } else { DARK_RED }, &stroke_style, o);
            }

            // Draw button states, ’cos why not.
            {
                let y = 27.0;
                let mut x = 30.0;
                let width = 8.0;
                let height = 6.0;
                let dx = 10.0;
                for pressed in [
                    tool.state.stylus_button_1_pressed,
                    tool.state.stylus_button_2_pressed,
                    tool.state.stylus_button_3_pressed,
                ] {
                    dt.fill_rect(x, y, width, height, &BLACK, o);
                    if !pressed {
                        dt.fill_rect(x + 1.0, y + 1.0, width - 2.0, height - 2.0, &WHITE, o);
                    }
                    x += dx;
                }
            }

            // Could draw more, but you get the idea.

            let cursor_surface = Surface::new(&self.compositor_state, qh).unwrap();
            let cursor_wl_surface = cursor_surface.wl_surface();
            wtool.set_cursor(proximity_in_serial, Some(cursor_wl_surface), 16, 16);
            buffer.attach_to(cursor_wl_surface).expect("buffer attach");
            cursor_wl_surface.damage_buffer(0, 0, width as i32, height as i32);
            cursor_wl_surface.commit();
            tool.cursor_surface = Some(cursor_surface);
        }
    }

    /// Initialise the canvas buffer, damaging but not attaching/committing.
    ///
    /// This should be called whenever the window is resized, too.
    fn init_canvas(&mut self) {
        let (buffer, canvas) = self.pool
            .create_buffer(
                self.width as i32,
                self.height as i32,
                self.width as i32 * 4,
                wl_shm::Format::Xrgb8888,
            )
            .expect("create buffer");
        // Make everything white.
        canvas.fill(0xff);
        self.buffer = Some(buffer);
        let surface = self.window.wl_surface();
        surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
    }

    /// Clear the canvas to white, damaging but not attaching/committing.
    fn clear_canvas(&mut self) {
        if let Some(buffer) = &self.buffer {
            let canvas = self.pool.canvas(buffer).expect("buffer is still active");
            // Make everything white.
            canvas.fill(0xff);
            let surface = self.window.wl_surface();
            surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
        }
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
