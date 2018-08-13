use std::cmp::max;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use wayland_client::commons::Implementation;
use wayland_client::protocol::{
    wl_compositor, wl_pointer, wl_seat, wl_shm, wl_subcompositor, wl_subsurface, wl_surface,
};
use wayland_client::Proxy;

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

const BORDER_SIZE: u32 = 12;
const HEADER_SIZE: u32 = 32;
const BUTTON_SPACE: u32 = 10;
const ROUNDING_SIZE: u32 = 3;

// defining the color scheme
#[cfg(target_endian = "little")]
mod colors {
    pub const INACTIVE_BORDER: &[u8] = &[0x60, 0x60, 0x60, 0xFF];
    pub const ACTIVE_BORDER: &[u8] = &[0x80, 0x80, 0x80, 0xFF];
    pub const RED_BUTTON_REGULAR: &[u8] = &[0x40, 0x40, 0xB0, 0xFF];
    pub const RED_BUTTON_HOVER: &[u8] = &[0x40, 0x40, 0xFF, 0xFF];
    pub const GREEN_BUTTON_REGULAR: &[u8] = &[0x40, 0xB0, 0x40, 0xFF];
    pub const GREEN_BUTTON_HOVER: &[u8] = &[0x40, 0xFF, 0x40, 0xFF];
    pub const YELLOW_BUTTON_REGULAR: &[u8] = &[0x40, 0xB0, 0xB0, 0xFF];
    pub const YELLOW_BUTTON_HOVER: &[u8] = &[0x40, 0xFF, 0xFF, 0xFF];
    pub const YELLOW_BUTTON_DISABLED: &[u8] = &[0x20, 0x80, 0x80, 0xFF];
}
#[cfg(target_endian = "big")]
mod colors {
    pub const INACTIVE_BORDER: &[u8] = &[0xFF, 0x60, 0x60, 0x60];
    pub const ACTIVE_BORDER: &[u8] = &[0xFF, 0x80, 0x80, 0x80];
    pub const RED_BUTTON_REGULAR: &[u8] = &[0xFF, 0xB0, 0x40, 0x40];
    pub const RED_BUTTON_HOVER: &[u8] = &[0xFF, 0xFF, 0x40, 0x40];
    pub const GREEN_BUTTON_REGULAR: &[u8] = &[0xFF, 0x40, 0xB0, 0x40];
    pub const GREEN_BUTTON_HOVER: &[u8] = &[0xFF, 0x40, 0xFF, 0x40];
    pub const YELLOW_BUTTON_REGULAR: &[u8] = &[0xFF, 0xB0, 0xB0, 0x40];
    pub const YELLOW_BUTTON_HOVER: &[u8] = &[0xFF, 0xFF, 0xFF, 0x40];
    pub const YELLOW_BUTTON_DISABLED: &[u8] = &[0xFF, 0x80, 0x80, 0x20];
}

/*
 * Utilities
 */

const HEAD: usize = 0;
const TOP: usize = 1;
const BOTTOM: usize = 2;
const LEFT: usize = 3;
const RIGHT: usize = 4;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Location {
    None,
    Head,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    TopLeft,
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
    parts: [Part; 5],
    size: Mutex<(u32, u32)>,
    resizable: Arc<Mutex<bool>>,
    implem: Mutex<Box<Implementation<u32, FrameRequest> + Send>>,
    maximized: Arc<Mutex<bool>>,
}

impl Inner {
    fn find_surface(&self, surface: &Proxy<wl_surface::WlSurface>) -> Location {
        if surface.equals(&self.parts[HEAD].surface) {
            Location::Head
        } else if surface.equals(&self.parts[TOP].surface) {
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
        Location::Head | Location::Button(_) => find_button(x, y, width),

        Location::Top | Location::TopLeft | Location::TopRight => {
            if x <= f64::from(BORDER_SIZE) {
                Location::TopLeft
            } else if x >= f64::from(width + BORDER_SIZE) {
                Location::TopRight
            } else {
                Location::Top
            }
        }

        Location::Bottom | Location::BottomLeft | Location::BottomRight => {
            if x <= f64::from(BORDER_SIZE) {
                Location::BottomLeft
            } else if x >= f64::from(width + BORDER_SIZE) {
                Location::BottomRight
            } else {
                Location::Bottom
            }
        }

        other => other,
    }
}

fn find_button(x: f64, y: f64, w: u32) -> Location {
    if (w >= 24 + 2 * BUTTON_SPACE)
        && (x >= f64::from(w - 24 - BUTTON_SPACE))
        && (x <= f64::from(w - BUTTON_SPACE))
        && (y <= f64::from(HEADER_SIZE) / 2.0 + 8.0)
        && (y >= f64::from(HEADER_SIZE) / 2.0 - 8.0)
    {
        Location::Button(UIButton::Close)
    } else if (w >= 56 + 2 * BUTTON_SPACE)
        && (x >= f64::from(w - 56 - BUTTON_SPACE))
        && (x <= f64::from(w - 32 - BUTTON_SPACE))
        && (y <= f64::from(HEADER_SIZE) / 2.0 + 8.0)
        && (y >= f64::from(HEADER_SIZE) / 2.0 - 8.0)
    {
        Location::Button(UIButton::Maximize)
    } else if (w >= 88 + 2 * BUTTON_SPACE)
        && (x >= f64::from(w - 88 - BUTTON_SPACE))
        && (x <= f64::from(w - 64 - BUTTON_SPACE))
        && (y <= f64::from(HEADER_SIZE) / 2.0 + 8.0)
        && (y >= f64::from(HEADER_SIZE) / 2.0 - 8.0)
    {
        Location::Button(UIButton::Minimize)
    } else {
        Location::Head
    }
}

/// A minimalistic set of decorations
///
/// This class draws minimalistic decorations, which are arguably not very
/// beautiful, but functional.
pub struct BasicFrame {
    inner: Arc<Inner>,
    pools: DoubleMemPool,
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
        let parts = [
            Part::new(base_surface, compositor, subcompositor),
            Part::new(base_surface, compositor, subcompositor),
            Part::new(base_surface, compositor, subcompositor),
            Part::new(base_surface, compositor, subcompositor),
            Part::new(base_surface, compositor, subcompositor),
        ];
        let inner = Arc::new(Inner {
            parts,
            size: Mutex::new((1, 1)),
            resizable: Arc::new(Mutex::new(true)),
            implem: Mutex::new(implementation),
            maximized: Arc::new(Mutex::new(false)),
        });
        let my_inner = inner.clone();
        // Send a Refresh request on callback from DoubleMemPool as it will be fired when
        // None was previously returned from `pool()` and the draw was postponed
        let pools = DoubleMemPool::new(&shm, move |_, _| {
            my_inner
                .implem
                .lock()
                .unwrap()
                .receive(FrameRequest::Refresh, 0);
        })?;
        Ok(BasicFrame {
            inner,
            pools,
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

        {
            // grab the current pool
            let pool = match self.pools.pool() {
                Some(pool) => pool,
                None => return,
            };
            // resize the pool as appropriate
            let pxcount = (HEADER_SIZE * width)
                + max(
                    (width + 2 * BORDER_SIZE) * BORDER_SIZE,
                    (height + HEADER_SIZE) * BORDER_SIZE,
                );

            pool.resize(4 * pxcount as usize)
                .expect("I/O Error while redrawing the borders");

            // Redraw the grey borders
            let color = if self.active {
                colors::ACTIVE_BORDER
            } else {
                colors::INACTIVE_BORDER
            };

            let _ = pool.seek(SeekFrom::Start(0));
            // draw the grey background
            {
                let mut writer = BufWriter::new(&mut *pool);

                // For every pixel in header
                for y in 0..HEADER_SIZE {
                    if y < ROUNDING_SIZE && !*self.inner.maximized.lock().unwrap() {
                        // Calculate the circle width at y using trigonometry and pythagoras theorem
                        let circle_width = ROUNDING_SIZE
                            - ((ROUNDING_SIZE as f32).powi(2)
                                - ((ROUNDING_SIZE - y) as f32).powi(2))
                                .sqrt() as u32;

                        for x in 0..width {
                            if x >= circle_width && x < width - circle_width {
                                let _ = writer.write(color);
                            } else {
                                let _ = writer.write(&[0x00, 0x00, 0x00, 0x00]);
                            }
                        }
                    } else {
                        for _ in 0..width {
                            let _ = writer.write(color);
                        }
                    }
                }

                // For every pixel in borders
                for _ in BORDER_SIZE * width..pxcount {
                    let _ = writer.write(&[0x00, 0x00, 0x00, 0x00]);
                }

                draw_buttons(
                    &mut writer,
                    width,
                    true,
                    &self
                        .pointers
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
            // -> head-subsurface
            let buffer = pool.buffer(
                0,
                width as i32,
                HEADER_SIZE as i32,
                4 * width as i32,
                wl_shm::Format::Argb8888,
            );
            self.inner.parts[HEAD]
                .subsurface
                .set_position(0, -(HEADER_SIZE as i32));
            self.inner.parts[HEAD].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                self.inner.parts[HEAD].surface.damage_buffer(
                    0,
                    0,
                    width as i32,
                    HEADER_SIZE as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                self.inner.parts[HEAD]
                    .surface
                    .damage(0, 0, width as i32, HEADER_SIZE as i32);
            }
            self.inner.parts[HEAD].surface.commit();

            // -> top-subsurface
            let buffer = pool.buffer(
                4 * (width * HEADER_SIZE) as i32,
                (width + 2 * BORDER_SIZE) as i32,
                BORDER_SIZE as i32,
                4 * (width + 2 * BORDER_SIZE) as i32,
                wl_shm::Format::Argb8888,
            );
            self.inner.parts[TOP].subsurface.set_position(
                -(BORDER_SIZE as i32),
                -(HEADER_SIZE as i32 + BORDER_SIZE as i32),
            );
            self.inner.parts[TOP].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                self.inner.parts[TOP].surface.damage_buffer(
                    0,
                    0,
                    (width + 2 * BORDER_SIZE) as i32,
                    BORDER_SIZE as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                self.inner.parts[TOP].surface.damage(
                    0,
                    0,
                    (width + 2 * BORDER_SIZE) as i32,
                    BORDER_SIZE as i32,
                );
            }
            self.inner.parts[TOP].surface.commit();

            // -> bottom-subsurface
            let buffer = pool.buffer(
                4 * (width * HEADER_SIZE) as i32,
                (width + 2 * BORDER_SIZE) as i32,
                BORDER_SIZE as i32,
                4 * (width + 2 * BORDER_SIZE) as i32,
                wl_shm::Format::Argb8888,
            );
            self.inner.parts[BOTTOM]
                .subsurface
                .set_position(-(BORDER_SIZE as i32), height as i32);
            self.inner.parts[BOTTOM].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                self.inner.parts[BOTTOM].surface.damage_buffer(
                    0,
                    0,
                    (width + 2 * BORDER_SIZE) as i32,
                    BORDER_SIZE as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                self.inner.parts[BOTTOM].surface.damage(
                    0,
                    0,
                    (width + 2 * BORDER_SIZE) as i32,
                    BORDER_SIZE as i32,
                );
            }
            self.inner.parts[BOTTOM].surface.commit();

            // -> left-subsurface
            let buffer = pool.buffer(
                4 * (width * HEADER_SIZE) as i32,
                BORDER_SIZE as i32,
                (height + HEADER_SIZE) as i32,
                4 * (BORDER_SIZE as i32),
                wl_shm::Format::Argb8888,
            );
            self.inner.parts[LEFT]
                .subsurface
                .set_position(-(BORDER_SIZE as i32), -(HEADER_SIZE as i32));
            self.inner.parts[LEFT].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                self.inner.parts[LEFT].surface.damage_buffer(
                    0,
                    0,
                    BORDER_SIZE as i32,
                    (height + HEADER_SIZE) as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                self.inner.parts[LEFT].surface.damage(
                    0,
                    0,
                    BORDER_SIZE as i32,
                    (height + HEADER_SIZE) as i32,
                );
            }
            self.inner.parts[LEFT].surface.commit();

            // -> right-subsurface
            let buffer = pool.buffer(
                4 * (width * HEADER_SIZE) as i32,
                BORDER_SIZE as i32,
                (height + HEADER_SIZE) as i32,
                4 * (BORDER_SIZE as i32),
                wl_shm::Format::Argb8888,
            );
            self.inner.parts[RIGHT]
                .subsurface
                .set_position(width as i32, -(HEADER_SIZE as i32));
            self.inner.parts[RIGHT].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                self.inner.parts[RIGHT].surface.damage_buffer(
                    0,
                    0,
                    BORDER_SIZE as i32,
                    (height + HEADER_SIZE) as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                self.inner.parts[RIGHT].surface.damage(
                    0,
                    0,
                    BORDER_SIZE as i32,
                    (height + HEADER_SIZE) as i32,
                );
            }
            self.inner.parts[RIGHT].surface.commit();
        }
    }

    fn subtract_borders(&self, width: i32, height: i32) -> (i32, i32) {
        if self.hidden {
            (width, height)
        } else {
            (width, height - HEADER_SIZE as i32)
        }
    }

    fn add_borders(&self, width: i32, height: i32) -> (i32, i32) {
        if self.hidden {
            (width, height)
        } else {
            (width, height + HEADER_SIZE as i32)
        }
    }

    fn location(&self) -> (i32, i32) {
        if self.hidden {
            (0, 0)
        } else {
            (0, -(HEADER_SIZE as i32))
        }
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
        Location::Head => Some(FrameRequest::Move(seat.clone())),
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
    mouses: &Vec<Location>,
) {
    // draw up to 3 buttons, depending on the width of the window
    // color of the button depends on whether a pointer is on it, and the maximizable
    // button can be disabled
    // buttons are 24x16
    if width >= 24 + 2 * BUTTON_SPACE {
        // draw the red button
        let color = if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Close))
        {
            colors::RED_BUTTON_HOVER
        } else {
            colors::RED_BUTTON_REGULAR
        };
        let _ = pool.seek(SeekFrom::Start(
            4 * u64::from(width * (HEADER_SIZE / 2 - 8) + width - 24 - BUTTON_SPACE),
        ));
        for _ in 0..16 {
            for _ in 0..24 {
                let _ = pool.write(color);
            }
            let _ = pool.seek(SeekFrom::Current(4 * i64::from(width - 24)));
        }
    }

    if width >= 56 + 2 * BUTTON_SPACE {
        // draw the yellow button
        let color = if !maximizable {
            colors::YELLOW_BUTTON_DISABLED
        } else if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Maximize))
        {
            colors::YELLOW_BUTTON_HOVER
        } else {
            colors::YELLOW_BUTTON_REGULAR
        };
        let _ = pool.seek(SeekFrom::Start(
            4 * u64::from(width * (HEADER_SIZE / 2 - 8) + width - 56 - BUTTON_SPACE),
        ));
        for _ in 0..16 {
            for _ in 0..24 {
                let _ = pool.write(color);
            }
            let _ = pool.seek(SeekFrom::Current(4 * i64::from(width - 24)));
        }
    }

    if width >= 88 + 2 * BUTTON_SPACE {
        // draw the green button
        let color = if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Minimize))
        {
            colors::GREEN_BUTTON_HOVER
        } else {
            colors::GREEN_BUTTON_REGULAR
        };
        let _ = pool.seek(SeekFrom::Start(
            4 * u64::from(width * (HEADER_SIZE / 2 - 8) + width - 88 - BUTTON_SPACE),
        ));
        for _ in 0..16 {
            for _ in 0..24 {
                let _ = pool.write(color);
            }
            let _ = pool.seek(SeekFrom::Current(4 * i64::from(width - 24)));
        }
    }
}
