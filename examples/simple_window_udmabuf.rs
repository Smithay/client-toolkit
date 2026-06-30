// Requires compositor advertising dmabuf protocol, and user with access to `/dev/udmabuf`

use std::{
    array,
    convert::TryInto,
    io,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd},
    time::Duration,
};

use memmap2::MmapMut;
use rustix::{
    fs::{fcntl_add_seals, ftruncate, memfd_create, MemfdFlags, Mode, OFlags, SealFlags},
    ioctl::{ioctl, opcode, Ioctl, Opcode, Setter},
    param::page_size,
};
use smithay_client_toolkit::activation::RequestData;
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::{
    activation::{ActivationHandler, ActivationState},
    compositor::{CompositorHandler, CompositorState, FrameCallbackData},
    delegate_registry,
    dmabuf::{DmabufFeedback, DmabufHandler, DmabufState},
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, QueueHandle,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1, zwp_linux_dmabuf_feedback_v1,
};

const SWAPCHAIN_SIZE: usize = 3;

fn main() {
    env_logger::init();

    // All Wayland apps start by connecting the compositor (server).
    let conn = Connection::connect_to_env().unwrap();

    // Enumerate the list of globals to get the protocols the server implements.
    let (globals, event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();
    let mut event_loop: EventLoop<SimpleWindow> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn.clone(), event_queue).insert(loop_handle).unwrap();

    // The compositor (not to be confused with the server which is commonly called the compositor) allows
    // configuring surfaces to be presented.
    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    // For desktop platforms, the XDG shell is the standard protocol for creating desktop windows.
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg shell is not available");
    // If the compositor supports xdg-activation it probably wants us to use it to get focus
    let xdg_activation = ActivationState::bind(&globals, &qh).ok();

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

    // To request focus, we first need to request a token
    if let Some(activation) = xdg_activation.as_ref() {
        activation.request_token(
            &qh,
            RequestData {
                seat_and_serial: None,
                surface: Some(window.wl_surface().clone()),
                app_id: Some(String::from("io.github.smithay.client-toolkit.SimpleWindow")),
                udata: (),
            },
        )
    }

    let udmabuf_dev =
        rustix::fs::open("/dev/udmabuf", OFlags::RDONLY | OFlags::CLOEXEC, Mode::empty()).unwrap();

    let mut simple_window = SimpleWindow {
        // Seats and outputs may be hotplugged at runtime, therefore we need to setup a registry state to
        // listen for seats and outputs.
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        dmabuf_state: DmabufState::new(&globals, &qh),
        xdg_activation,
        qh,
        udmabuf_dev,

        exit: false,
        first_configure: true,
        width: 256,
        height: 256,
        shift: None,
        buffers: None,
        window,
        keyboard: None,
        keyboard_focus: false,
        pointer: None,
        loop_handle: event_loop.handle(),
    };

    // We don't draw immediately, the configure will notify us when to first draw.
    loop {
        event_loop.dispatch(Duration::from_millis(16), &mut simple_window).unwrap();

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
    dmabuf_state: DmabufState,
    xdg_activation: Option<ActivationState>,
    qh: QueueHandle<Self>,
    udmabuf_dev: OwnedFd,

    exit: bool,
    first_configure: bool,
    width: u32,
    height: u32,
    shift: Option<u32>,
    buffers: Option<[Dmabuf; SWAPCHAIN_SIZE]>,
    window: Window,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    pointer: Option<wl_pointer::WlPointer>,
    loop_handle: LoopHandle<'static, SimpleWindow>,
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
        println!("Window configured to: {:?}", configure);

        self.buffers = None;
        self.width = configure.new_size.0.map(|v| v.get()).unwrap_or(256);
        self.height = configure.new_size.1.map(|v| v.get()).unwrap_or(256);

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh);
        }
    }
}

impl ActivationHandler for SimpleWindow {
    type RequestUdata = ();

    fn new_token(&mut self, token: String, _data: &RequestData<()>) {
        self.xdg_activation
            .as_ref()
            .unwrap()
            .activate::<SimpleWindow>(self.window.wl_surface(), token);
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
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|_state, _wl_kbd, event| {
                        println!("Repeat: {:?} ", event);
                    }),
                )
                .expect("Failed to create keyboard");

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

impl KeyboardHandler for SimpleWindow {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        keysyms: &[Keysym],
    ) {
        if self.window.wl_surface() == surface {
            println!("Keyboard focus on window with pressed syms: {keysyms:?}");
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
        println!("Key press: {event:?}");
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key repeat: {event:?}");
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key release: {event:?}");
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
        println!("Update modifiers: {modifiers:?}");
    }
}

impl PointerHandler for SimpleWindow {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            // Ignore events for other surfaces
            if &event.surface != self.window.wl_surface() {
                continue;
            }

            match event.kind {
                Enter { .. } => {
                    println!("Pointer entered @{:?}", event.position);
                }
                Leave { .. } => {
                    println!("Pointer left");
                }
                Motion { .. } => {}
                Press { button, .. } => {
                    println!("Press {:x} @ {:?}", button, event.position);
                    self.shift = self.shift.xor(Some(0));
                }
                Release { button, .. } => {
                    println!("Release {:x} @ {:?}", button, event.position);
                }
                Axis { horizontal, vertical, .. } => {
                    println!("Scroll H:{horizontal:?}, V:{vertical:?}");
                }
            }
        }
    }
}

impl DmabufHandler for SimpleWindow {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_feedback(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _proxy: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        _feedback: DmabufFeedback,
    ) {
    }

    fn created(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _params: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
        _buffer: wl_buffer::WlBuffer,
    ) {
    }

    fn failed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _params: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
    ) {
    }

    fn released(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        buffer: &wl_buffer::WlBuffer,
    ) {
        if let Some(buffer) = self.buffers.iter_mut().flatten().find(|b| b.wl_buffer == *buffer) {
            buffer.released = true;
        }
    }
}

impl SimpleWindow {
    pub fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;

        if self.buffers.is_none() {
            self.buffers = Some(array::from_fn(|_| {
                self.allocate_dmabuf(width as u64, height as u64).unwrap()
            }));
        }
        let buffer = &mut self.buffers.as_mut().unwrap()[0];
        // TODO wait for release
        assert!(buffer.released);

        ioctl_dma_buf_sync(
            buffer.dmabuf_fd.as_fd(),
            DmabufSyncFlags::START | DmabufSyncFlags::WRITE,
        );

        let stride = buffer.stride;

        // Draw to the window:
        {
            let shift = self.shift.unwrap_or(0);
            buffer.mmap.chunks_exact_mut(4).enumerate().for_each(|(index, chunk)| {
                let x = (index % (stride as usize / 4)) as u32;
                let y = (index / (stride as usize / 4)) as u32;
                if x >= width || y >= height {
                    return;
                }

                let x = (x + shift) % width;

                let a = 0xFF;
                let r = u32::min(((width - x) * 0xFF) / width, ((height - y) * 0xFF) / height);
                let g = u32::min((x * 0xFF) / width, ((height - y) * 0xFF) / height);
                let b = u32::min(((width - x) * 0xFF) / width, (y * 0xFF) / height);
                let color = [b as u8, g as u8, r as u8, a as u8];

                let array: &mut [u8; 4] = chunk.try_into().unwrap();
                *array = color;
            });

            if let Some(shift) = &mut self.shift {
                *shift = (*shift + 1) % width;
            }
        }

        ioctl_dma_buf_sync(buffer.dmabuf_fd.as_fd(), DmabufSyncFlags::END | DmabufSyncFlags::WRITE);

        // Damage the entire window
        self.window.wl_surface().damage_buffer(0, 0, self.width as i32, self.height as i32);

        // Request our next frame
        self.window.wl_surface().frame(qh, FrameCallbackData(self.window.wl_surface().clone()));

        // Attach and commit to present.
        self.window.wl_surface().attach(Some(&buffer.wl_buffer), 0, 0);
        self.window.commit();
        buffer.released = false;
        self.buffers.as_mut().unwrap().rotate_left(1);
    }

    fn allocate_dmabuf(&self, width: u64, height: u64) -> io::Result<Dmabuf> {
        let stride_align = 256; // WIP can error with less than this on AMD?
        let stride = (width * 4).next_multiple_of(stride_align);
        let size = (height * stride).next_multiple_of(page_size() as u64);
        let mem_fd = memfd_create("udmabuf", MemfdFlags::ALLOW_SEALING | MemfdFlags::CLOEXEC)?;
        ftruncate(&mem_fd, size)?;
        fcntl_add_seals(&mem_fd, SealFlags::SHRINK)?;

        let dmabuf_fd = udmabuf_from_memfd(self.udmabuf_dev.as_fd(), mem_fd.as_fd(), 0, size)?;

        const DRM_FOURCC_LINEAR: u64 = 0;
        const DRM_FOURCC_ARGB8888: u32 = 875713089;
        let params = self.dmabuf_state.create_params(&self.qh).unwrap();
        params.add(dmabuf_fd.as_fd(), 0, 0, stride as u32, DRM_FOURCC_LINEAR);
        let wl_buffer = params
            .create_immed(
                width as i32,
                height as i32,
                DRM_FOURCC_ARGB8888,
                zwp_linux_buffer_params_v1::Flags::empty(),
                &self.qh,
            )
            .0;

        let mmap = unsafe { MmapMut::map_mut(&dmabuf_fd)? };
        Ok(Dmabuf { dmabuf_fd, wl_buffer, stride, mmap, released: true })
    }
}

struct Dmabuf {
    dmabuf_fd: OwnedFd,
    wl_buffer: wl_buffer::WlBuffer,
    stride: u64,
    mmap: MmapMut,
    released: bool,
}

fn udmabuf_from_memfd(
    fd: BorrowedFd,
    mem_fd: BorrowedFd,
    offset: u64,
    size: u64,
) -> io::Result<OwnedFd> {
    #[repr(C)]
    #[derive(Debug)]
    struct udmabuf_create {
        memfd: u32,
        flags: u32,
        offset: u64,
        size: u64,
    }
    const UDMABUF_FLAGS_CLOEXEC: u32 = 0x01;

    unsafe impl Ioctl for udmabuf_create {
        type Output = RawFd;

        const IS_MUTATING: bool = false;

        fn opcode(&self) -> Opcode {
            opcode::write::<udmabuf_create>(b'u', 0x42)
        }

        fn as_ptr(&mut self) -> *mut rustix::ffi::c_void {
            self as *mut Self as *mut _
        }

        unsafe fn output_from_ptr(
            out: rustix::ioctl::IoctlOutput,
            _extract_output: *mut rustix::ffi::c_void,
        ) -> rustix::io::Result<Self::Output> {
            if out < 0 {
                Err(rustix::io::Errno::from_raw_os_error(!out))
            } else {
                Ok(out)
            }
        }
    }

    let args = udmabuf_create {
        memfd: mem_fd.as_raw_fd() as u32,
        flags: UDMABUF_FLAGS_CLOEXEC,
        offset,
        size,
    };

    unsafe {
        ioctl(fd, args)
            .map(|fd| OwnedFd::from_raw_fd(fd))
            .map_err(|err| io::Error::from_raw_os_error(err.raw_os_error()))
    }
}

bitflags::bitflags! {
    /// Flags for the [`Dmabuf::sync_plane`](Dmabuf::sync_plane) operation
    #[derive(Copy, Clone)]
    pub struct DmabufSyncFlags: std::ffi::c_ulonglong {
        /// Read from the dmabuf
        const READ = 1 << 0;
        /// Write to the dmabuf
        #[allow(clippy::identity_op)]
        const WRITE = 2 << 0;
        /// Start of read/write
        const START = 0 << 2;
        /// End of read/write
        const END = 1 << 2;
    }
}

fn ioctl_dma_buf_sync(fd: BorrowedFd, flags: DmabufSyncFlags) {
    #[repr(C)]
    #[allow(non_camel_case_types)]
    struct dma_buf_sync {
        flags: DmabufSyncFlags,
    }

    const DMA_BUF_SYNC: rustix::ioctl::Opcode =
        rustix::ioctl::opcode::write::<dma_buf_sync>(b'b', 0);

    unsafe {
        rustix::ioctl::ioctl(fd, Setter::<DMA_BUF_SYNC, _>::new(dma_buf_sync { flags })).unwrap()
    };
}

delegate_registry!(SimpleWindow);

impl ProvidesRegistryState for SimpleWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

smithay_client_toolkit::delegate_dispatch2!(SimpleWindow);
