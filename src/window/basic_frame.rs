use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use byteorder::{NativeEndian, WriteBytesExt};

use wayland_client::commons::Implementation;
use wayland_client::protocol::{wl_buffer, wl_compositor, wl_pointer, wl_seat, wl_shm,
                               wl_subcompositor, wl_subsurface, wl_surface};
use wayland_client::Proxy;

use wayland_client::protocol::wl_buffer::RequestsTrait as BufferRequests;
use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use wayland_client::protocol::wl_pointer::RequestsTrait as PointerRequests;
use wayland_client::protocol::wl_seat::RequestsTrait as SeatRequests;
use wayland_client::protocol::wl_subcompositor::RequestsTrait as SubcompRequests;
use wayland_client::protocol::wl_subsurface::RequestsTrait as SubsurfaceRequests;
use wayland_client::protocol::wl_surface::RequestsTrait as SurfaceRequests;

use super::{Frame, FrameRequest};
use pointer::{AutoPointer, AutoThemer};
use utils::{DoubleMemPool, MemPool};

/*
 * Drawing theme definitions
 */

const DECORATION_SIZE: u32 = 8;
const DECORATION_TOP_SIZE: u32 = 32;

// defining the color scheme
const INACTIVE_BORDER: u32 = 0xFF606060;
const ACTIVE_BORDER: u32 = 0xFF808080;
const RED_BUTTON_REGULAR: u32 = 0xFFB04040;
const RED_BUTTON_HOVER: u32 = 0xFFFF4040;
const GREEN_BUTTON_REGULAR: u32 = 0xFF40B040;
const GREEN_BUTTON_HOVER: u32 = 0xFF40FF40;
const YELLOW_BUTTON_REGULAR: u32 = 0xFFB0B040;
const YELLOW_BUTTON_HOVER: u32 = 0xFFFFFF40;
const YELLOW_BUTTON_DISABLED: u32 = 0xFF808020;

/*
 * Utilities
 */

const TOP: usize = 0;
const BOTTOM: usize = 1;
const LEFT: usize = 2;
const RIGHT: usize = 3;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Location {
    None,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    TopLeft,
    TopBar,
    Button(UIButton),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum UIButton {
    Minimize,
    Maximize,
    Close,
}

struct Part {
    surface: Proxy<wl_surface::WlSurface>,
    subsurface: Proxy<wl_subsurface::WlSubsurface>,
}

impl Part {
    fn new(
        parent: &Proxy<wl_surface::WlSurface>,
        compositor: &Proxy<wl_compositor::WlCompositor>,
        subcompositor: &Proxy<wl_subcompositor::WlSubcompositor>,
    ) -> Part {
        let surface = compositor.create_surface().unwrap().implement(|_, _| {});
        let subsurface = subcompositor
            .get_subsurface(&surface, parent)
            .unwrap()
            .implement(|_, _| {});
        Part {
            surface,
            subsurface,
        }
    }
}

impl Drop for Part {
    fn drop(&mut self) {
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

struct PointerUserData {
    location: Location,
    position: (f64, f64),
    seat: Proxy<wl_seat::WlSeat>,
}

/*
 * The core frame
 */

struct Inner {
    parts: [Part; 4],
    size: Mutex<(u32, u32)>,
    resizable: Mutex<bool>,
    implem: Mutex<Box<Implementation<u32, FrameRequest> + Send>>,
    maximized: Mutex<bool>,
}

impl Inner {
    fn find_surface(&self, surface: &Proxy<wl_surface::WlSurface>) -> Location {
        if surface.equals(&self.parts[TOP].surface) {
            Location::Top
        } else if surface.equals(&self.parts[BOTTOM].surface) {
            Location::Bottom
        } else if surface.equals(&self.parts[LEFT].surface) {
            Location::Left
        } else if surface.equals(&self.parts[RIGHT].surface) {
            Location::Right
        } else {
            Location::None
        }
    }
}

fn precise_location(old: Location, width: u32, x: f64, y: f64) -> Location {
    match old {
        Location::Top
        | Location::TopRight
        | Location::TopLeft
        | Location::TopBar
        | Location::Button(_) => {
            // top surface
            if x <= DECORATION_SIZE as f64 {
                Location::TopLeft
            } else if x >= (width + DECORATION_SIZE) as f64 {
                Location::TopRight
            } else if y <= DECORATION_SIZE as f64 {
                Location::Top
            } else {
                find_button(x, y, width)
            }
        }
        Location::Bottom | Location::BottomLeft | Location::BottomRight => {
            if x <= DECORATION_SIZE as f64 {
                Location::BottomLeft
            } else if x >= (width + DECORATION_SIZE) as f64 {
                Location::BottomRight
            } else {
                Location::Bottom
            }
        }
        other => other,
    }
}

fn find_button(x: f64, y: f64, w: u32) -> Location {
    if (w >= 24) && (x > (w - 24) as f64) && (x <= w as f64)
        && (y <= (DECORATION_SIZE + 16) as f64)
    {
        Location::Button(UIButton::Close)
    } else if (w >= 56) && (x > (w - 56) as f64)
        && (x <= (w - 32) as f64)
        && (y <= (DECORATION_SIZE + 16) as f64)
    {
        Location::Button(UIButton::Maximize)
    } else if (w >= 88) && (x > (w - 88) as f64)
        && (x <= (w - 64) as f64)
        && (y <= (DECORATION_SIZE + 16) as f64)
    {
        Location::Button(UIButton::Minimize)
    } else {
        Location::TopBar
    }
}

/// A minimalistic set of decorations
///
/// This class draws minimalistic decorations, which are arguably not very
/// beautiful, but functional.
pub struct BasicFrame {
    inner: Arc<Inner>,
    pools: DoubleMemPool,
    buffers: Vec<Proxy<wl_buffer::WlBuffer>>,
    active: bool,
    hidden: bool,
    pointers: Vec<AutoPointer>,
    themer: AutoThemer,
    surface_version: u32,
}

impl Frame for BasicFrame {
    type Error = ::std::io::Error;
    fn init(
        base_surface: &Proxy<wl_surface::WlSurface>,
        compositor: &Proxy<wl_compositor::WlCompositor>,
        subcompositor: &Proxy<wl_subcompositor::WlSubcompositor>,
        shm: &Proxy<wl_shm::WlShm>,
        implementation: Box<Implementation<u32, FrameRequest> + Send>,
    ) -> Result<BasicFrame, ::std::io::Error> {
        let pools = DoubleMemPool::new(&shm)?;
        let parts = [
            Part::new(base_surface, compositor, subcompositor),
            Part::new(base_surface, compositor, subcompositor),
            Part::new(base_surface, compositor, subcompositor),
            Part::new(base_surface, compositor, subcompositor),
        ];
        Ok(BasicFrame {
            inner: Arc::new(Inner {
                parts: parts,
                size: Mutex::new((1, 1)),
                resizable: Mutex::new(true),
                implem: Mutex::new(implementation),
                maximized: Mutex::new(false),
            }),
            pools,
            buffers: Vec::new(),
            active: false,
            hidden: false,
            pointers: Vec::new(),
            themer: AutoThemer::init(None, compositor.clone(), shm.clone()),
            surface_version: compositor.version(),
        })
    }

    fn new_seat(&mut self, seat: &Proxy<wl_seat::WlSeat>) {
        use self::wl_pointer::Event;
        let inner = self.inner.clone();
        let pointer = self.themer.theme_pointer_with_impl(
            seat.get_pointer().unwrap(),
            move |event, pointer: AutoPointer| {
                let data = unsafe { &mut *(pointer.get_user_data() as *mut PointerUserData) };
                let (width, _) = *(inner.size.lock().unwrap());
                let resizable = *(inner.resizable.lock().unwrap());
                match event {
                    Event::Enter {
                        serial,
                        surface,
                        surface_x,
                        surface_y,
                    } => {
                        data.location = precise_location(
                            inner.find_surface(&surface),
                            width,
                            surface_x,
                            surface_y,
                        );
                        data.position = (surface_x, surface_y);
                        if resizable {
                            change_pointer(&pointer, data.location, Some(serial));
                        }
                    }
                    Event::Leave { serial, .. } => {
                        data.location = Location::None;
                        if resizable {
                            change_pointer(&pointer, data.location, Some(serial));
                        }
                    }
                    Event::Motion {
                        surface_x,
                        surface_y,
                        ..
                    } => {
                        data.position = (surface_x, surface_y);
                        let newpos = precise_location(data.location, width, surface_x, surface_y);
                        if newpos != data.location {
                            match (newpos, data.location) {
                                (Location::Button(_), _) | (_, Location::Button(_)) => {
                                    // pointer movement involves a button, request refresh
                                    inner
                                        .implem
                                        .lock()
                                        .unwrap()
                                        .receive(FrameRequest::Refresh, 0);
                                }
                                _ => (),
                            }
                            // we changed of part of the decoration, pointer image
                            // may need to be changed
                            data.location = newpos;
                            if resizable {
                                change_pointer(&pointer, data.location, None);
                            }
                        }
                    }
                    Event::Button {
                        serial,
                        button,
                        state,
                        ..
                    } => {
                        if state == wl_pointer::ButtonState::Pressed && button == 0x110 {
                            // left click
                            let req = request_for_location(
                                data.location,
                                &data.seat,
                                *(inner.maximized.lock().unwrap()),
                                resizable,
                            );
                            if let Some(req) = req {
                                inner.implem.lock().unwrap().receive(req, serial);
                            }
                        }
                    }
                    _ => {}
                }
            },
        );
        pointer.set_user_data(Box::into_raw(Box::new(PointerUserData {
            location: Location::None,
            position: (0.0, 0.0),
            seat: seat.clone(),
        })) as *mut ());
        self.pointers.push(pointer);
    }

    fn set_active(&mut self, active: bool) -> bool {
        if self.active != active {
            self.active = active;
            true
        } else {
            false
        }
    }

    fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    fn set_maximized(&mut self, maximized: bool) -> bool {
        let mut my_maximized = self.inner.maximized.lock().unwrap();
        if *my_maximized != maximized {
            *my_maximized = maximized;
            true
        } else {
            false
        }
    }

    fn set_resizable(&mut self, resizable: bool) {
        *(self.inner.resizable.lock().unwrap()) = resizable;
    }

    fn resize(&mut self, newsize: (u32, u32)) {
        *(self.inner.size.lock().unwrap()) = newsize;
    }

    fn redraw(&mut self) {
        if self.hidden {
            // don't draw the borders
            for p in &self.inner.parts {
                p.surface.attach(None, 0, 0);
                p.surface.commit();
            }
            return;
        }
        let (width, height) = *(self.inner.size.lock().unwrap());
        // destroy current pending buffers
        // TODO: do double-buffering
        for b in self.buffers.drain(..) {
            b.destroy();
        }

        {
            // grab the current pool
            let pool = self.pools.pool();
            // resize the pool as appropriate
            let pxcount = 2 * height * DECORATION_SIZE
                + (width + 2 * DECORATION_SIZE) * (DECORATION_SIZE + DECORATION_TOP_SIZE);
            pool.resize(4 * pxcount as usize)
                .expect("I/O Error while redrawing the borders");

            // Redraw the grey borders
            let color = if self.active {
                ACTIVE_BORDER
            } else {
                INACTIVE_BORDER
            };
            let _ = pool.seek(SeekFrom::Start(0));
            // draw the grey background
            {
                let mut writer = BufWriter::new(&mut *pool);
                // For every pixel in top border
                for y in 0..DECORATION_TOP_SIZE {
                    for _ in 0..DECORATION_SIZE {
                        let _ = writer.write_u32::<NativeEndian>(0x00_00_00_00);
                    }
                    for _ in 0..width {
                        let _ = writer.write_u32::<NativeEndian>(color);
                    }
                    for _ in 0..DECORATION_SIZE {
                        let _ = writer.write_u32::<NativeEndian>(0x00_00_00_00);
                    }
                }

                // For every pixel in the other borders
                for _ in DECORATION_TOP_SIZE * (width + 2 * DECORATION_SIZE)..pxcount {
                    let _ = writer.write_u32::<NativeEndian>(0x00_00_00_00);
                }

                draw_buttons(
                    &mut writer,
                    width,
                    true,
                    self.pointers
                        .iter()
                        .flat_map(|p| {
                            if p.is_alive() {
                                let data =
                                    unsafe { &mut *(p.get_user_data() as *mut PointerUserData) };
                                Some(data.location)
                            } else {
                                None
                            }
                        })
                        .collect(),
                );
                let _ = writer.flush();
            }

            // Create the buffers
            // -> top-subsurface
            let buffer = pool.buffer(
                0,
                (width + 2 * DECORATION_SIZE) as i32,
                DECORATION_TOP_SIZE as i32,
                4 * (width + 2 * DECORATION_SIZE) as i32,
                wl_shm::Format::Argb8888,
            ).implement(|_, _| {});
            self.inner.parts[TOP]
                .subsurface
                .set_position(-(DECORATION_SIZE as i32), -(DECORATION_TOP_SIZE as i32));
            self.inner.parts[TOP].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                self.inner.parts[TOP].surface.damage_buffer(
                    0,
                    0,
                    (width + 2 * DECORATION_SIZE) as i32,
                    DECORATION_TOP_SIZE as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                self.inner.parts[TOP].surface.damage(
                    0,
                    0,
                    (width + 2 * DECORATION_SIZE) as i32,
                    DECORATION_TOP_SIZE as i32,
                );
            }
            self.inner.parts[TOP].surface.commit();
            self.buffers.push(buffer);
            // -> bottom-subsurface
            let buffer = pool.buffer(
                4 * (DECORATION_TOP_SIZE * (width + 2 * DECORATION_SIZE)) as i32,
                (width + 2 * DECORATION_SIZE) as i32,
                DECORATION_SIZE as i32,
                4 * (width + 2 * DECORATION_SIZE) as i32,
                wl_shm::Format::Argb8888,
            ).implement(|_, _| {});
            self.inner.parts[BOTTOM]
                .subsurface
                .set_position(-(DECORATION_SIZE as i32), height as i32);
            self.inner.parts[BOTTOM].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                self.inner.parts[BOTTOM].surface.damage_buffer(
                    0,
                    0,
                    (width + 2 * DECORATION_SIZE) as i32,
                    DECORATION_SIZE as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                self.inner.parts[BOTTOM].surface.damage(
                    0,
                    0,
                    (width + 2 * DECORATION_SIZE) as i32,
                    DECORATION_SIZE as i32,
                );
            }
            self.inner.parts[BOTTOM].surface.commit();
            self.buffers.push(buffer);
            // -> left-subsurface
            let buffer = pool.buffer(
                4 * ((DECORATION_TOP_SIZE + DECORATION_SIZE) * (width + 2 * DECORATION_SIZE))
                    as i32,
                DECORATION_SIZE as i32,
                height as i32,
                4 * (DECORATION_SIZE as i32),
                wl_shm::Format::Argb8888,
            ).implement(|_, _| {});
            self.inner.parts[LEFT]
                .subsurface
                .set_position(-(DECORATION_SIZE as i32), 0);
            self.inner.parts[LEFT].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                self.inner.parts[LEFT].surface.damage_buffer(
                    0,
                    0,
                    DECORATION_SIZE as i32,
                    height as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                self.inner.parts[LEFT]
                    .surface
                    .damage(0, 0, DECORATION_SIZE as i32, height as i32);
            }
            self.inner.parts[LEFT].surface.commit();
            self.buffers.push(buffer);
            // -> right-subsurface
            let buffer = pool.buffer(
                4 * ((DECORATION_TOP_SIZE + DECORATION_SIZE) * (width + 2 * DECORATION_SIZE)
                    + DECORATION_SIZE * height) as i32,
                DECORATION_SIZE as i32,
                height as i32,
                4 * (DECORATION_SIZE as i32),
                wl_shm::Format::Argb8888,
            ).implement(|_, _| {});
            self.inner.parts[RIGHT]
                .subsurface
                .set_position(width as i32, 0);
            self.inner.parts[RIGHT].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                self.inner.parts[RIGHT].surface.damage_buffer(
                    0,
                    0,
                    DECORATION_SIZE as i32,
                    height as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                self.inner.parts[RIGHT]
                    .surface
                    .damage(0, 0, DECORATION_SIZE as i32, height as i32);
            }
            self.inner.parts[RIGHT].surface.commit();
            self.buffers.push(buffer);
        }
        // swap the pool
        self.pools.swap();
    }

    fn subtract_borders(&self, width: i32, height: i32) -> (i32, i32) {
        if self.hidden {
            (width, height)
        } else {
            (width, height - DECORATION_TOP_SIZE as i32)
        }
    }

    fn add_borders(&self, width: i32, height: i32) -> (i32, i32) {
        if self.hidden {
            (width, height)
        } else {
            (width, height + DECORATION_TOP_SIZE as i32)
        }
    }

    fn location(&self) -> (i32, i32) {
        if self.hidden { (0, 0) } else { (0, -(DECORATION_TOP_SIZE as i32)) }
    }
}

impl Drop for BasicFrame {
    fn drop(&mut self) {
        for ptr in self.pointers.drain(..) {
            let _data = unsafe { Box::from_raw(ptr.get_user_data() as *mut PointerUserData) };
            ptr.set_user_data(::std::ptr::null_mut());
            if ptr.version() >= 3 {
                ptr.release();
            }
        }
    }
}

fn change_pointer(pointer: &AutoPointer, location: Location, serial: Option<u32>) {
    let name = match location {
        Location::Top => "top_side",
        Location::TopRight => "top_right_corner",
        Location::Right => "right_side",
        Location::BottomRight => "bottom_right_corner",
        Location::Bottom => "bottom_side",
        Location::BottomLeft => "bottom_left_corner",
        Location::Left => "left_side",
        Location::TopLeft => "top_left_corner",
        _ => "left_ptr",
    };
    let _ = pointer.set_cursor(name, serial);
}

fn request_for_location(
    location: Location,
    seat: &Proxy<wl_seat::WlSeat>,
    maximized: bool,
    resizable: bool,
) -> Option<FrameRequest> {
    use wayland_protocols::xdg_shell::client::xdg_toplevel::ResizeEdge;
    match location {
        Location::Top if resizable => Some(FrameRequest::Resize(seat.clone(), ResizeEdge::Top)),
        Location::TopLeft if resizable => {
            Some(FrameRequest::Resize(seat.clone(), ResizeEdge::TopLeft))
        }
        Location::Left if resizable => Some(FrameRequest::Resize(seat.clone(), ResizeEdge::Left)),
        Location::BottomLeft if resizable => {
            Some(FrameRequest::Resize(seat.clone(), ResizeEdge::BottomLeft))
        }
        Location::Bottom if resizable => {
            Some(FrameRequest::Resize(seat.clone(), ResizeEdge::Bottom))
        }
        Location::BottomRight if resizable => {
            Some(FrameRequest::Resize(seat.clone(), ResizeEdge::BottomRight))
        }
        Location::Right if resizable => Some(FrameRequest::Resize(seat.clone(), ResizeEdge::Right)),
        Location::TopRight if resizable => {
            Some(FrameRequest::Resize(seat.clone(), ResizeEdge::TopRight))
        }
        Location::TopBar => Some(FrameRequest::Move(seat.clone())),
        Location::Button(UIButton::Close) => Some(FrameRequest::Close),
        Location::Button(UIButton::Maximize) => if maximized {
            Some(FrameRequest::UnMaximize)
        } else {
            Some(FrameRequest::Maximize)
        },
        Location::Button(UIButton::Minimize) => Some(FrameRequest::Minimize),
        _ => None,
    }
}

fn draw_buttons(
    pool: &mut BufWriter<&mut MemPool>,
    width: u32,
    maximizable: bool,
    mouses: Vec<Location>,
) {
    // draw up to 3 buttons, depending on the width of the window
    // color of the button depends on whether a pointer is on it, and the maximizable
    // button can be disabled
    // buttons are 24x16
    let ds = DECORATION_SIZE;

    if width >= 24 {
        // draw the red button
        let color = if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Close))
        {
            RED_BUTTON_HOVER
        } else {
            RED_BUTTON_REGULAR
        };
        let _ = pool.seek(SeekFrom::Start(
            4 * ((width + 2 * ds) * ds + width - 24) as u64,
        ));
        for _ in 0..16 {
            for _ in 0..24 {
                let _ = pool.write_u32::<NativeEndian>(color);
            }
            let _ = pool.seek(SeekFrom::Current(4 * (width + 2 * ds - 24) as i64));
        }
    }

    if width >= 56 {
        // draw the yellow button
        let color = if !maximizable {
            YELLOW_BUTTON_DISABLED
        } else if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Maximize))
        {
            YELLOW_BUTTON_HOVER
        } else {
            YELLOW_BUTTON_REGULAR
        };
        let _ = pool.seek(SeekFrom::Start(
            4 * ((width + 2 * ds) * ds + width - 56) as u64,
        ));
        for _ in 0..16 {
            for _ in 0..24 {
                let _ = pool.write_u32::<NativeEndian>(color);
            }
            let _ = pool.seek(SeekFrom::Current(4 * (width + 2 * ds - 24) as i64));
        }
    }

    if width >= 88 {
        // draw the green button
        let color = if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Minimize))
        {
            GREEN_BUTTON_HOVER
        } else {
            GREEN_BUTTON_REGULAR
        };
        let _ = pool.seek(SeekFrom::Start(
            4 * ((width + 2 * ds) * ds + width - 88) as u64,
        ));
        for _ in 0..16 {
            for _ in 0..24 {
                let _ = pool.write_u32::<NativeEndian>(color);
            }
            let _ = pool.seek(SeekFrom::Current(4 * (width + 2 * ds - 24) as i64));
        }
    }
}
