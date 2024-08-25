use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop, LoopHandle,
        },
        calloop_wayland_source::WaylandSource,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
    shm::{raw::RawPool, Shm, ShmHandler},
};
use std::time::Duration;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};

struct AppData {
    loop_handle: LoopHandle<'static, Self>,
    conn: Connection,
    compositor_state: CompositorState,
    output_state: OutputState,
    registry_state: RegistryState,
    shm: Shm,
    session_lock_state: SessionLockState,
    session_lock: Option<SessionLock>,
    lock_surfaces: Vec<SessionLockSurface>,
    exit: bool,
}

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let (globals, event_queue) = registry_queue_init(&conn).unwrap();
    let qh: QueueHandle<AppData> = event_queue.handle();
    let mut event_loop: EventLoop<AppData> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");

    let mut app_data = AppData {
        loop_handle: event_loop.handle(),
        conn: conn.clone(),
        compositor_state: CompositorState::bind(&globals, &qh).unwrap(),
        output_state: OutputState::new(&globals, &qh),
        registry_state: RegistryState::new(&globals),
        shm: Shm::bind(&globals, &qh).unwrap(),
        session_lock_state: SessionLockState::new(&globals, &qh),
        session_lock: None,
        lock_surfaces: Vec::new(),
        exit: false,
    };

    app_data.session_lock =
        Some(app_data.session_lock_state.lock(&qh).expect("ext-session-lock not supported"));

    // After locking the session, we're expected to create a lock surface for each output.
    // As soon as all lock surfaces are created, `SessionLockHandler::locked` will be called
    // and the every surface receives a `SessionLockHandler::configure` call.
    let qh: QueueHandle<AppData> = event_queue.handle();
    for output in app_data.output_state().outputs() {
        let session_lock = app_data.session_lock.as_ref().unwrap();
        let surface = app_data.compositor_state.create_surface(&qh);

        // It's important to keep the `SessionLockSurface` returned here around, as the
        // surface will be destroyed when the `SessionLockSurface` is dropped.
        let lock_surface = session_lock.create_lock_surface(surface, &output, &qh);
        app_data.lock_surfaces.push(lock_surface);
    }

    WaylandSource::new(conn.clone(), event_queue).insert(event_loop.handle()).unwrap();

    loop {
        event_loop.dispatch(Duration::from_millis(16), &mut app_data).unwrap();

        if app_data.exit {
            break;
        }
    }
}

impl SessionLockHandler for AppData {
    fn locked(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {
        println!("Locked");

        // After 5 seconds, destroy lock
        self.loop_handle
            .insert_source(Timer::from_duration(Duration::from_secs(5)), |_, _, app_data| {
                // Unlock the lock
                app_data.session_lock.take().unwrap().unlock();
                // Sync connection to make sure compostor receives destroy
                app_data.conn.roundtrip().unwrap();
                // Then we can exit
                app_data.exit = true;
                TimeoutAction::Drop
            })
            .unwrap();
    }

    fn finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
        println!("Session could not be locked");
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        session_lock_surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;

        let mut pool = RawPool::new(width as usize * height as usize * 4, &self.shm).unwrap();
        let canvas = pool.mmap();
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
        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            width as i32 * 4,
            wl_shm::Format::Argb8888,
            (),
            qh,
        );

        session_lock_surface.wl_surface().attach(Some(&buffer), 0, 0);
        session_lock_surface.wl_surface().commit();

        buffer.destroy();
    }
}

impl CompositorHandler for AppData {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
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

impl OutputHandler for AppData {
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

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState,];
}

impl ShmHandler for AppData {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

smithay_client_toolkit::delegate_compositor!(AppData);
smithay_client_toolkit::delegate_output!(AppData);
smithay_client_toolkit::delegate_session_lock!(AppData);
smithay_client_toolkit::delegate_shm!(AppData);
smithay_client_toolkit::delegate_registry!(AppData);
wayland_client::delegate_noop!(AppData: ignore wl_buffer::WlBuffer);
