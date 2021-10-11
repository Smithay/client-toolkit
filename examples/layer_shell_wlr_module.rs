use smithay_client_toolkit::{
    default_environment,
    environment::SimpleGlobal,
    new_default_environment,
    output::{with_output_info, OutputInfo},
    reexports::{
        calloop,
        client::protocol::{wl_output, wl_shm},
        protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1,
    },
    shell::layer,
    shm::AutoMemPool,
    WaylandSource,
};

use std::cell::RefCell;
use std::rc::Rc;

default_environment!(Env,
    fields = [
        layer_shell: SimpleGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    ],
    singles = [
        zwlr_layer_shell_v1::ZwlrLayerShellV1 => layer_shell
    ],
);

struct Surface {
    surface: layer::LayerSurface,
    pool: AutoMemPool,
}

impl Surface {
    fn new(surface: layer::LayerSurface, pool: AutoMemPool) -> Self {
        surface.surface.commit();

        Self { pool, surface }
    }

    /// Handles any events that have occurred since the last call, redrawing if needed.
    /// Returns true if the surface should be dropped.
    fn handle_events(&mut self) -> bool {
        match self.surface.render_event.take() {
            Some(layer::RenderEvent::Closed) => true,
            Some(layer::RenderEvent::Configure { width, height }) => {
                self.surface.dimensions = (width, height);
                self.draw();
                false
            }
            None => false,
        }
    }

    fn draw(&mut self) {
        let stride = 4 * self.surface.dimensions.0 as i32;
        let width = self.surface.dimensions.0 as i32;
        let height = self.surface.dimensions.1 as i32;

        // Note: unwrap() is only used here in the interest of simplicity of the example.
        // A "real" application should handle the case where both pools are still in use by the
        // compositor.
        let (canvas, buffer) =
            self.pool.buffer(width, height, stride, wl_shm::Format::Argb8888).unwrap();

        for dst_pixel in canvas.chunks_exact_mut(4) {
            let pixel = 0x24021bu32.to_ne_bytes();
            dst_pixel[0] = pixel[0];
            dst_pixel[1] = pixel[1];
            dst_pixel[2] = pixel[2];
            dst_pixel[3] = pixel[3];
        }

        // Attach the buffer to the surface and mark the entire surface as damaged
        self.surface.surface.attach(Some(&buffer), 0, 0);
        self.surface.surface.damage_buffer(0, 0, width as i32, height as i32);

        // Finally, commit the surface
        self.surface.surface.commit();
    }
}

fn main() {
    let (env, display, queue) =
        new_default_environment!(Env, fields = [layer_shell: SimpleGlobal::new(),])
            .expect("Initial roundtrip failed!");

    let surfaces = Rc::new(RefCell::new(Vec::new()));

    let layer_shell = env.require_global::<zwlr_layer_shell_v1::ZwlrLayerShellV1>();

    let env_handle = env.clone();
    let surfaces_handle = Rc::clone(&surfaces);
    let output_handler = move |output: wl_output::WlOutput, info: &OutputInfo| {
        if info.obsolete {
            // an output has been removed, release it
            surfaces_handle.borrow_mut().retain(|(i, _)| *i != info.id);
            output.release();
        } else {
            // an output has been created, construct a surface for it
            let surface = env_handle.create_surface().detach();
            let pool = env_handle.create_auto_pool().expect("Failed to create a memory pool!");

            let wlr_shell = layer::LayerSurface::new(
                &output,
                surface,
                &layer_shell.clone(),
                layer::Layer::Background,
                layer::Anchor::Top,
                (info.modes[0].dimensions.0 as u32, info.modes[0].dimensions.1 as u32),
            );

            (*surfaces_handle.borrow_mut()).push((info.id, Surface::new(wlr_shell, pool)));
        }
    };

    // Process currently existing outputs
    for output in env.get_all_outputs() {
        if let Some(info) = with_output_info(&output, Clone::clone) {
            output_handler(output, &info);
        }
    }

    // Setup a listener for changes
    // The listener will live for as long as we keep this handle alive
    let _listner_handle =
        env.listen_for_outputs(move |output, info, _| output_handler(output, info));

    let mut event_loop = calloop::EventLoop::<()>::try_new().unwrap();

    WaylandSource::new(queue).quick_insert(event_loop.handle()).unwrap();

    loop {
        // This is ugly, let's hope that some version of drain_filter() gets stabilized soon
        // https://github.com/rust-lang/rust/issues/43244
        {
            let mut surfaces = surfaces.borrow_mut();
            let mut i = 0;
            while i != surfaces.len() {
                if surfaces[i].1.handle_events() {
                    surfaces.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        display.flush().unwrap();
        event_loop.dispatch(None, &mut ()).unwrap();
    }
}
