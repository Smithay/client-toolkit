use smithay_client_toolkit::{
    environment,
    environment::{Environment, SimpleGlobal},
    output::{with_output_info, OutputHandler, OutputHandling, OutputInfo, OutputStatusListener},
    reexports::{
        calloop,
        client::protocol::{wl_compositor, wl_output, wl_shm, wl_surface},
        client::{Attached, Display, Main, Proxy},
        protocols::wlr::unstable::layer_shell::v1::client::{
            zwlr_layer_shell_v1, zwlr_layer_surface_v1,
        },
    },
    shm::{DoubleMemPool, ShmHandler},
    WaylandSource,
};

use byteorder::{NativeEndian, WriteBytesExt};

use std::cell::{Cell, RefCell};
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::rc::Rc;

struct Env {
    compositor: SimpleGlobal<wl_compositor::WlCompositor>,
    layer_shell: SimpleGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    shm: ShmHandler,
    outputs: OutputHandler,
}

impl OutputHandling for Env {
    fn listen<F: FnMut(wl_output::WlOutput, &OutputInfo) + 'static>(
        &mut self,
        f: F,
    ) -> OutputStatusListener {
        self.outputs.listen(f)
    }
}

environment!(Env,
    singles = [
        wl_compositor::WlCompositor => compositor,
        zwlr_layer_shell_v1::ZwlrLayerShellV1 => layer_shell,
        wl_shm::WlShm => shm,
    ],
    multis = [
        wl_output::WlOutput => outputs
    ]
);

#[derive(PartialEq, Copy, Clone)]
enum RenderEvent {
    Configure { width: u32, height: u32 },
    Closed,
}

struct Surface {
    surface: wl_surface::WlSurface,
    layer_surface: Main<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    next_render_event: Rc<Cell<Option<RenderEvent>>>,
    pools: DoubleMemPool,
    dimensions: (u32, u32),
}

impl Surface {
    fn new(
        output: &wl_output::WlOutput,
        surface: wl_surface::WlSurface,
        layer_shell: &Attached<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
        pools: DoubleMemPool,
    ) -> Self {
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            Some(&output),
            zwlr_layer_shell_v1::Layer::Overlay,
            "example".to_owned(),
        );

        layer_surface.set_size(32, 32);
        // Anchor to the top left corner of the output
        layer_surface
            .set_anchor(zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left);

        let next_render_event = Rc::new(Cell::new(None::<RenderEvent>));
        let next_render_event_handle = Rc::clone(&next_render_event);
        layer_surface.quick_assign(move |layer_surface, event, _| {
            match (event, next_render_event_handle.get()) {
                (zwlr_layer_surface_v1::Event::Closed, _) => {
                    next_render_event_handle.set(Some(RenderEvent::Closed));
                }
                (
                    zwlr_layer_surface_v1::Event::Configure {
                        serial,
                        width,
                        height,
                    },
                    next,
                ) if next != Some(RenderEvent::Closed) => {
                    layer_surface.ack_configure(serial);
                    next_render_event_handle.set(Some(RenderEvent::Configure { width, height }));
                }
                (_, _) => {}
            }
        });

        // Commit so that the server will send a configure event
        surface.commit();

        Self {
            surface,
            layer_surface,
            next_render_event,
            pools,
            dimensions: (0, 0),
        }
    }

    /// Handles any events that have occurred since the last call, redrawing if needed.
    /// Returns true if the surface should be dropped.
    fn handle_events(&mut self) -> bool {
        match self.next_render_event.take() {
            Some(RenderEvent::Closed) => true,
            Some(RenderEvent::Configure { width, height }) => {
                self.dimensions = (width, height);
                self.draw();
                false
            }
            None => false,
        }
    }

    fn draw(&mut self) {
        let pool = self.pools.pool().unwrap();

        let stride = 4 * self.dimensions.0 as i32;
        let width = self.dimensions.0 as i32;
        let height = self.dimensions.1 as i32;

        // First make sure the pool is the right size
        pool.resize((stride * height) as usize).unwrap();

        // Create a new buffer from the pool
        let buffer = pool.buffer(0, width, height, stride, wl_shm::Format::Argb8888);

        // Write the color to all bytes of the pool
        pool.seek(SeekFrom::Start(0)).unwrap();
        {
            let mut writer = BufWriter::new(&mut *pool);
            for _ in 0..(width * height) {
                writer.write_u32::<NativeEndian>(0xff00ff00).unwrap();
            }
            writer.flush().unwrap();
        }

        // Attach the buffer to the surface and mark the entire surface as damaged
        self.surface.attach(Some(&buffer), 0, 0);
        self.surface
            .damage_buffer(0, 0, width as i32, height as i32);

        // Finally, commit the surface
        self.surface.commit();
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}

fn main() {
    let (env, display, queue) = {
        let display = Display::connect_to_env().unwrap();
        let mut queue = display.create_event_queue();
        let env = Environment::init(
            &Proxy::clone(&display).attach(queue.token()),
            Env {
                compositor: SimpleGlobal::new(),
                layer_shell: SimpleGlobal::new(),
                shm: ShmHandler::new(),
                outputs: OutputHandler::new(),
            },
        );
        let ret = queue.sync_roundtrip(&mut (), |_, _, _| unreachable!());
        ret.and_then(|_| queue.sync_roundtrip(&mut (), |_, _, _| unreachable!()))
            .expect("Error during initial setup");

        (env, display, queue)
    };

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
            let surface = env_handle.create_surface();
            let pools = env_handle
                .create_double_pool(|_| {})
                .expect("Failed to create a memory pool!");
            (*surfaces_handle.borrow_mut()).push((
                info.id,
                Surface::new(&output, surface, &layer_shell.clone(), pools),
            ));
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
    let _listner_handle = env.listen_for_outputs(output_handler);

    let mut event_loop = calloop::EventLoop::<()>::new().unwrap();

    let _source_queue = event_loop
        .handle()
        .insert_source(WaylandSource::new(queue), |ret, _| {
            if let Err(e) = ret {
                panic!("Wayland connection lost: {:?}", e);
            }
        })
        .unwrap();

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
