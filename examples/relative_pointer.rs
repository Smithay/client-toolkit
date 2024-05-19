//! An example demonstrating relative pointer and (if supported) pointer constraints

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_pointer, delegate_pointer_constraints,
    delegate_registry, delegate_relative_pointer, delegate_seat, delegate_shm, delegate_xdg_shell,
    delegate_xdg_window,
    globals::ProvidesBoundGlobal,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        pointer_constraints::{PointerConstraintsHandler, PointerConstraintsState},
        relative_pointer::{RelativeMotionEvent, RelativePointerHandler, RelativePointerState},
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
};
use wayland_protocols::wp::{
    pointer_constraints::zv1::client::{
        zwp_confined_pointer_v1, zwp_locked_pointer_v1, zwp_pointer_constraints_v1,
    },
    relative_pointer::zv1::client::zwp_relative_pointer_v1,
};

const WHITE: raqote::SolidSource = raqote::SolidSource { r: 255, g: 255, b: 255, a: 255 };
const BLACK: raqote::SolidSource = raqote::SolidSource { r: 0, g: 0, b: 0, a: 255 };
const GREY: raqote::SolidSource = raqote::SolidSource { r: 192, g: 192, b: 192, a: 255 };
const SOLID_WHITE: raqote::Source = raqote::Source::Solid(WHITE);
const SOLID_BLACK: raqote::Source = raqote::Source::Solid(BLACK);
const SPEED: f32 = 0.001;

enum Constraint {
    Confine(zwp_confined_pointer_v1::ZwpConfinedPointerV1),
    ConfineRegion(zwp_confined_pointer_v1::ZwpConfinedPointerV1),
    Lock(zwp_locked_pointer_v1::ZwpLockedPointerV1),
}

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
        relative_pointer_state: RelativePointerState::bind(&globals, &qh),
        pointer_constraint_state: PointerConstraintsState::bind(&globals, &qh),

        exit: false,
        width: 256,
        height: 256,
        window: None,
        pointer: None,
        relative_pointer: None,
        constraint: None,
        constraint_active: false,
        pos: (0.5, 0.5),
        font,
    };

    let surface = simple_window.compositor_state.create_surface(&qh);

    let window =
        simple_window.xdg_shell_state.create_window(surface, WindowDecorations::ServerDefault, &qh);

    window.set_title("A wayland window");
    window.set_app_id("io.github.smithay.client-toolkit.RelativePointer");
    window.set_min_size(Some((256, 256)));

    window.commit();

    simple_window.window = Some(window);

    while !simple_window.exit {
        event_queue.blocking_dispatch(&mut simple_window).unwrap();
    }
}

struct SimpleWindow {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    xdg_shell_state: XdgShell,
    relative_pointer_state: RelativePointerState,
    pointer_constraint_state: PointerConstraintsState,

    exit: bool,
    width: u32,
    height: u32,
    window: Option<Window>,
    pointer: Option<wl_pointer::WlPointer>,
    relative_pointer: Option<zwp_relative_pointer_v1::ZwpRelativePointerV1>,
    constraint: Option<Constraint>,
    constraint_active: bool,
    pos: (f32, f32),
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

        if let Some(Constraint::ConfineRegion(confine)) = &self.constraint {
            let region = self.compositor_state.wl_compositor().create_region(qh, ());
            region.add(
                self.width as i32 / 4,
                self.height as i32 / 4,
                self.width as i32 / 2,
                self.height as i32 / 2,
            );
            confine.set_region(Some(&region));
        }

        self.draw(conn, qh);
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
        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self.seat_state.get_pointer(qh, &seat).expect("Failed to create pointer");
            let relative_pointer =
                self.relative_pointer_state.get_relative_pointer(&pointer, qh).ok();
            if relative_pointer.is_some() {
                println!("Created relative pointer");
            } else {
                println!("Compositor does not support relative pointer events");
            }
            self.pointer = Some(pointer);
            self.relative_pointer = relative_pointer;
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
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

impl PointerConstraintsHandler for SimpleWindow {
    fn confined(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _confined_pointer: &zwp_confined_pointer_v1::ZwpConfinedPointerV1,
        _surface: &wl_surface::WlSurface,
        _pointer: &wl_pointer::WlPointer,
    ) {
        println!("Confined");
        self.constraint_active = true;
    }

    fn unconfined(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _confined_pointer: &zwp_confined_pointer_v1::ZwpConfinedPointerV1,
        _surface: &wl_surface::WlSurface,
        _pointer: &wl_pointer::WlPointer,
    ) {
        println!("Unconfined");
        self.constraint_active = false;
    }

    fn locked(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _locked_pointer: &zwp_locked_pointer_v1::ZwpLockedPointerV1,
        _surface: &wl_surface::WlSurface,
        _pointer: &wl_pointer::WlPointer,
    ) {
        println!("Locked");
        self.constraint_active = false;
        self.constraint_active = true;
    }

    fn unlocked(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _locked_pointer: &zwp_locked_pointer_v1::ZwpLockedPointerV1,
        _surface: &wl_surface::WlSurface,
        _pointer: &wl_pointer::WlPointer,
    ) {
        println!("Unlocked");
        self.constraint_active = false;
    }
}

impl RelativePointerHandler for SimpleWindow {
    fn relative_pointer_motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _relative_pointer: &zwp_relative_pointer_v1::ZwpRelativePointerV1,
        _pointer: &wl_pointer::WlPointer,
        event: RelativeMotionEvent,
    ) {
        println!("{event:?}");
        self.pos.0 = (self.pos.0 + event.delta.0 as f32 * SPEED).rem_euclid(1.);
        self.pos.1 = (self.pos.1 + event.delta.1 as f32 * SPEED).rem_euclid(1.);
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
            if let Some(Constraint::ConfineRegion(_)) = &self.constraint {
                dt.clear(GREY);
                dt.fill_rect(
                    (width / 4) as f32,
                    (height / 4) as f32,
                    (width / 2) as f32,
                    (height / 2) as f32,
                    &SOLID_WHITE,
                    &raqote::DrawOptions::new(),
                );
            } else {
                dt.clear(WHITE);
            }
            let mut pb = raqote::PathBuilder::new();
            pb.arc(
                self.pos.0 * self.width as f32,
                self.pos.1 * self.height as f32,
                5.,
                0.,
                2. * std::f32::consts::PI,
            );
            pb.close();
            dt.stroke(
                &pb.finish(),
                &SOLID_BLACK,
                &raqote::StrokeStyle::default(),
                &raqote::DrawOptions::new(),
            );
            dt.draw_text(
                &self.font,
                14.,
                self.constraint_label(),
                raqote::Point::new(2., 16.),
                &SOLID_BLACK,
                &raqote::DrawOptions::new(),
            );
            dt.draw_text(
                &self.font,
                14.,
                "Click to change mode",
                raqote::Point::new(2., 32.),
                &SOLID_BLACK,
                &raqote::DrawOptions::new(),
            );

            // Damage the entire window
            window.wl_surface().damage_buffer(0, 0, self.width as i32, self.height as i32);

            // Request our next frame
            window.wl_surface().frame(qh, window.wl_surface().clone());

            // Attach and commit to present.
            buffer.attach_to(window.wl_surface()).expect("buffer attach");
            window.wl_surface().commit();
        }
    }

    // Text for label describing current pointer constraint mode
    fn constraint_label(&self) -> &str {
        if self.pointer_constraint_state.bound_global().is_err() {
            return "Pointer constraints not supported by compositor";
        }
        match &self.constraint {
            None => "Pointer unconstrained",
            Some(Constraint::Confine(_)) => {
                if self.constraint_active {
                    "Pointer confined to window"
                } else {
                    "Pointer confined to window (inactive)"
                }
            }
            Some(Constraint::ConfineRegion(_)) => {
                if self.constraint_active {
                    "Pointer confined to region"
                } else {
                    "Pointer confined to region (inactive)"
                }
            }
            Some(Constraint::Lock(_)) => {
                if self.constraint_active {
                    "Pointer locked in place"
                } else {
                    "Pointer locked in place (inactive)"
                }
            }
        }
    }

    // Swap between constraint modes
    fn change_constraint(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        if self.pointer_constraint_state.bound_global().is_err() {
            return;
        }
        let pointer = if let Some(pointer) = self.pointer.as_ref() {
            pointer
        } else {
            return;
        };
        let surface = if let Some(window) = &self.window {
            window.wl_surface()
        } else {
            return;
        };

        self.constraint = match self.constraint.take() {
            None => Some(Constraint::Confine(
                self.pointer_constraint_state
                    .confine_pointer(
                        surface,
                        pointer,
                        None,
                        zwp_pointer_constraints_v1::Lifetime::Persistent,
                        qh,
                    )
                    .unwrap(),
            )),
            Some(Constraint::Confine(confine)) => {
                let region = self.compositor_state.wl_compositor().create_region(qh, ());
                region.add(
                    self.width as i32 / 4,
                    self.height as i32 / 4,
                    self.width as i32 / 2,
                    self.height as i32 / 2,
                );
                confine.set_region(Some(&region));
                Some(Constraint::ConfineRegion(confine))
            }
            Some(Constraint::ConfineRegion(confine)) => {
                confine.destroy();
                Some(Constraint::Lock(
                    self.pointer_constraint_state
                        .lock_pointer(
                            surface,
                            pointer,
                            None,
                            zwp_pointer_constraints_v1::Lifetime::Persistent,
                            qh,
                        )
                        .unwrap(),
                ))
            }
            Some(Constraint::Lock(lock)) => {
                lock.destroy();
                None
            }
        };

        self.draw(conn, qh);
    }
}

delegate_compositor!(SimpleWindow);
delegate_output!(SimpleWindow);
delegate_shm!(SimpleWindow);

delegate_seat!(SimpleWindow);
delegate_pointer!(SimpleWindow);
delegate_pointer_constraints!(SimpleWindow);
delegate_relative_pointer!(SimpleWindow);

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
