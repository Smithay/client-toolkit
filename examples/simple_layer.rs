//! This example is horrible. Please make a better one soon.

use std::convert::TryInto;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Modifiers},
        pointer::{PointerHandler, PointerScroll},
        Capability, SeatHandler, SeatState,
    },
    shell::layer::{
        Anchor, KeyboardInteractivity, Layer, LayerHandler, LayerState, LayerSurface,
        LayerSurfaceConfigure,
    },
    shm::{pool::slot::SlotPool, ShmHandler, ShmState},
};
use wayland_client::{
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let registry = display.get_registry(&qh, ()).unwrap();

    let mut simple_layer = SimpleLayer {
        registry_state: RegistryState::new(registry),
        seat_state: SeatState::new(),
        output_state: OutputState::new(),
        compositor_state: CompositorState::new(),
        shm_state: ShmState::new(),
        layer_state: LayerState::new(),

        exit: false,
        first_configure: true,
        pool: None,
        width: 256,
        height: 256,
        layer: None,
        keyboard: None,
        keyboard_focus: false,
        pointer: None,
        pointer_focus: false,
    };

    event_queue.blocking_dispatch(&mut simple_layer).unwrap();
    // event_queue.blocking_dispatch(&mut simple_layer).unwrap();

    let pool = simple_layer
        .shm_state
        .new_slot_pool(simple_layer.width as usize * simple_layer.height as usize * 4, &qh, ())
        .expect("Failed to create pool");
    simple_layer.pool = Some(pool);

    let surface = simple_layer.compositor_state.create_surface(&qh).unwrap();

    let layer = LayerSurface::builder()
        .size((256, 256))
        .anchor(Anchor::BOTTOM)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .namespace("sample_layer")
        .map(&qh, &mut simple_layer.layer_state, surface, Layer::Top)
        .expect("layer surface creation");

    simple_layer.layer = Some(layer);

    // We don't draw immediately, the configure will notify us when to first draw.

    loop {
        event_queue.blocking_dispatch(&mut simple_layer).unwrap();

        if simple_layer.exit {
            println!("exiting example");
            break;
        }
    }
}

struct SimpleLayer {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: ShmState,
    layer_state: LayerState,

    exit: bool,
    first_configure: bool,
    pool: Option<SlotPool>,
    width: u32,
    height: u32,
    layer: Option<LayerSurface>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    pointer: Option<wl_pointer::WlPointer>,
    pointer_focus: bool,
}

impl CompositorHandler for SimpleLayer {
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
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(qh);
    }
}

impl OutputHandler for SimpleLayer {
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

impl LayerHandler for SimpleLayer {
    fn layer_state(&mut self) -> &mut LayerState {
        &mut self.layer_state
    }

    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        if configure.new_size.0 == 0 || configure.new_size.1 == 0 {
            self.width = 256;
            self.height = 256;
        } else {
            self.width = configure.new_size.0;
            self.height = configure.new_size.1;
        }

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(qh);
        }
    }
}

impl SeatHandler for SimpleLayer {
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

        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self.seat_state.get_pointer(qh, &seat).expect("Failed to create pointer");
            self.pointer = Some(pointer);
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

impl KeyboardHandler for SimpleLayer {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        keysyms: &[u32],
    ) {
        if self.layer.as_ref().map(LayerSurface::wl_surface) == Some(surface) {
            println!("Keyboard focus on window with pressed syms: {:?}", keysyms);
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
        if self.layer.as_ref().map(LayerSurface::wl_surface) == Some(surface) {
            println!("Release keyboard focus on window");
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
        println!("Key press: {:?}", event);
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key release: {:?}", event);
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
    ) {
        println!("Update modifiers: {:?}", modifiers);
    }
}

impl PointerHandler for SimpleLayer {
    fn pointer_focus(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        surface: &wl_surface::WlSurface,
        entered: (f64, f64),
        _serial: u32,
    ) {
        if self.layer.as_ref().map(LayerSurface::wl_surface) == Some(surface) {
            println!("Pointer focus on layer, entering at {:?}", entered);
            self.pointer_focus = true;
        }
    }

    fn pointer_release_focus(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        if self.layer.as_ref().map(LayerSurface::wl_surface) == Some(surface) {
            println!("Release pointer focus on layer");
            self.pointer_focus = false;
        }
    }

    fn pointer_motion(
        &mut self,
        _conn: &Connection,
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
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        time: u32,
        button: u32,
        _serial: u32,
    ) {
        if self.pointer_focus {
            println!("Pointer press button: {:?} @ {}", button, time);
        }
    }

    fn pointer_release_button(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        time: u32,
        button: u32,
        _serial: u32,
    ) {
        if self.pointer_focus {
            println!("Pointer release button: {:?} @ {}", button, time);
        }
    }

    fn pointer_axis(
        &mut self,
        _: &Connection,
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

impl ShmHandler for SimpleLayer {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
    }
}

impl SimpleLayer {
    pub fn draw(&mut self, qh: &QueueHandle<Self>) {
        if let Some(window) = self.layer.as_ref() {
            let width = self.width;
            let height = self.height;
            let stride = self.width as i32 * 4;
            let pool = self.pool.as_mut().unwrap();

            let (buffer, canvas) = pool
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
                .expect("create buffer");

            // Draw to the window:
            {
                canvas.chunks_exact_mut(4).enumerate().for_each(|(index, chunk)| {
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

            // Damage the entire window
            window.wl_surface().damage_buffer(0, 0, width as i32, height as i32);

            // Request our next frame
            window.wl_surface().frame(qh, window.wl_surface().clone()).expect("create callback");

            // Attach and commit to present.
            buffer.attach_to(window.wl_surface()).expect("buffer attach");
            window.wl_surface().commit();

            // TODO save and reuse buffer when the window size is unchanged.  This is especially
            // useful if you do damage tracking, since you don't need to redraw the undamaged parts
            // of the canvas.
        }
    }
}

delegate_compositor!(SimpleLayer);
delegate_output!(SimpleLayer);
delegate_shm!(SimpleLayer);

delegate_seat!(SimpleLayer);
delegate_keyboard!(SimpleLayer);
delegate_pointer!(SimpleLayer);

delegate_layer!(SimpleLayer);

delegate_registry!(SimpleLayer: [
    CompositorState,
    OutputState,
    ShmState,
    SeatState,
    LayerState,
]);

impl ProvidesRegistryState for SimpleLayer {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
}
