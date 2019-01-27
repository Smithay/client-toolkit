use std::cmp::max;
use std::io::Read;
use std::sync::{Arc, Mutex};

use andrew::shapes::rectangle;
use andrew::text;
use andrew::text::fontconfig;
use andrew::{Canvas, Endian};

use wayland_client::protocol::{
    wl_compositor, wl_pointer, wl_seat, wl_shm, wl_subcompositor, wl_subsurface, wl_surface,
};
use wayland_client::Proxy;

use wayland_client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use wayland_client::protocol::wl_pointer::RequestsTrait as PointerRequests;
use wayland_client::protocol::wl_subcompositor::RequestsTrait as SubcompRequests;
use wayland_client::protocol::wl_subsurface::RequestsTrait as SubsurfaceRequests;
use wayland_client::protocol::wl_surface::RequestsTrait as SurfaceRequests;

use super::{ButtonState, Frame, FrameRequest, Theme};
use pointer::{AutoPointer, AutoThemer};
use utils::DoubleMemPool;

/*
 * Drawing theme definitions
 */

const BORDER_SIZE: u32 = 12;
const HEADER_SIZE: u32 = 32;
const BUTTON_SPACE: u32 = 10;
const ROUNDING_SIZE: u32 = 5;

// Defining the theme
struct DefaultTheme;

impl Theme for DefaultTheme {
    // Used for header color
    fn get_primary_color(&self, active: bool) -> [u8; 4] {
        if active {
            [0xFF, 0x80, 0x80, 0x80]
        } else {
            [0xFF, 0x60, 0x60, 0x60]
        }
    }

    fn get_secondary_color(&self, _active: bool) -> [u8; 4] {
        [0x00, 0x00, 0x00, 0x00]
    }

    fn get_close_button_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => [0xFF, 0xFF, 0x40, 0x40],
            ButtonState::Idle => [0xFF, 0xB0, 0x40, 0x40],
            _ => [0x00, 0x00, 0x00, 0x00],
        }
    }

    fn get_maximize_button_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => [0xFF, 0xFF, 0xFF, 0x40],
            ButtonState::Idle => [0xFF, 0xB0, 0xB0, 0x40],
            ButtonState::Disabled => [0xFF, 0x80, 0x80, 0x20],
        }
    }

    fn get_minimize_button_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => [0xFF, 0x40, 0xFF, 0x40],
            ButtonState::Idle => [0xFF, 0x40, 0xB0, 0x40],
            _ => [0x00, 0x00, 0x00, 0x00],
        }
    }
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
        let surface = compositor
            .create_surface(|surface| surface.implement(|_, _| {}, ()))
            .unwrap();
        let subsurface = subcompositor
            .get_subsurface(&surface, parent, |subsurface| {
                subsurface.implement(|_, _| {}, ())
            })
            .unwrap();
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
    implem: Mutex<Box<FnMut(FrameRequest, u32) + Send>>,
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
    theme: Box<Theme>,
    title: Option<String>,
    font_data: Option<Vec<u8>>,
}

impl Frame for BasicFrame {
    type Error = ::std::io::Error;
    fn init(
        base_surface: &Proxy<wl_surface::WlSurface>,
        compositor: &Proxy<wl_compositor::WlCompositor>,
        subcompositor: &Proxy<wl_subcompositor::WlSubcompositor>,
        shm: &Proxy<wl_shm::WlShm>,
        implementation: Box<FnMut(FrameRequest, u32) + Send>,
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
        let pools = DoubleMemPool::new(&shm, move || {
            (&mut *my_inner.implem.lock().unwrap())(FrameRequest::Refresh, 0);
        })?;
        Ok(BasicFrame {
            inner,
            pools,
            active: false,
            hidden: false,
            pointers: Vec::new(),
            themer: AutoThemer::init(None, compositor.clone(), &shm),
            surface_version: compositor.version(),
            theme: Box::new(DefaultTheme),
            title: None,
            font_data: None,
        })
    }

    fn new_seat(&mut self, seat: &Proxy<wl_seat::WlSeat>) {
        use self::wl_pointer::Event;
        let inner = self.inner.clone();
        let pointer = self.themer.theme_pointer_with_impl(
            seat,
            move |event, pointer: AutoPointer| {
                let data: &Mutex<PointerUserData> = pointer.user_data().unwrap();
                let data = &mut *data.lock().unwrap();
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
                                    (&mut *inner.implem.lock().unwrap())(FrameRequest::Refresh, 0);
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
                                (&mut *inner.implem.lock().unwrap())(req, serial);
                            }
                        }
                    }
                    _ => {}
                }
            },
            Mutex::new(PointerUserData {
                location: Location::None,
                position: (0.0, 0.0),
                seat: seat.clone(),
            }),
        );
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

            // draw the grey header bar
            {
                let mmap = pool.mmap();
                {
                    let color = self.theme.get_primary_color(self.active);

                    let mut header_canvas = Canvas::new(
                        &mut mmap[0..HEADER_SIZE as usize * width as usize * 4],
                        width as usize,
                        HEADER_SIZE as usize,
                        width as usize * 4,
                        Endian::native(),
                    );
                    header_canvas.clear();

                    let header_bar = rectangle::Rectangle::new(
                        (0, 0),
                        (width as usize - 1, HEADER_SIZE as usize - 1),
                        Some((
                            HEADER_SIZE as usize,
                            color,
                            rectangle::Sides::TOP,
                            Some(ROUNDING_SIZE as usize),
                        )),
                        None,
                    );
                    header_canvas.draw(&header_bar);

                    draw_buttons(
                        &mut header_canvas,
                        width,
                        true,
                        &self
                            .pointers
                            .iter()
                            .flat_map(|p| {
                                if p.is_alive() {
                                    let data: &Mutex<PointerUserData> = p.user_data().unwrap();
                                    Some(data.lock().unwrap().location)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<Location>>(),
                        &*self.theme,
                    );

                    if let Some(title) = self.title.clone() {
                        // If theres no stored font data, find the first ttf regular sans font and
                        // store it
                        if self.font_data.is_none() {
                            if let Some(font) = fontconfig::FontConfig::new()
                                .unwrap()
                                .get_regular_family_fonts("sans")
                                .unwrap()
                                .iter()
                                .filter_map(|p| {
                                    if p.extension().unwrap() == "ttf" {
                                        Some(p)
                                    } else {
                                        None
                                    }
                                })
                                .nth(0)
                            {
                                let mut font_data = Vec::new();
                                if let Ok(mut file) = ::std::fs::File::open(font) {
                                    match file.read_to_end(&mut font_data) {
                                        Ok(_) => self.font_data = Some(font_data),
                                        Err(err) => eprintln!("Could not read font file: {}", err),
                                    }
                                }
                            }
                        }

                        // Create text from stored title and font data
                        if let Some(ref font_data) = self.font_data {
                            let mut title_text = text::Text::new(
                                (0, HEADER_SIZE as usize / 2 - 8),
                                [0, 0, 0, 255],
                                font_data,
                                17.0,
                                1.0,
                                title,
                            );

                            // Check if text is bigger then the avaliable width
                            if (width as isize - 88 - 4 * BUTTON_SPACE as isize)
                                > (title_text.get_width() + BUTTON_SPACE as usize) as isize
                            {
                                title_text.pos.0 =
                                    (width as usize) / 2 - (title_text.get_width() / 2);
                                // Adjust position for buttons if both compete for space
                                if (width as usize) / 2 + (title_text.get_width() / 2)
                                    > (width - 88 - 2 * 2 * BUTTON_SPACE) as usize
                                {
                                    title_text.pos.0 -= ((width as usize) / 2
                                        + (title_text.get_width() / 2))
                                        - (width - 88 - 2 * 2 * BUTTON_SPACE) as usize;
                                }
                                header_canvas.draw(&title_text);
                            }
                        }
                    }
                }

                // For each pixel in borders
                {
                    for b in &mut mmap[HEADER_SIZE as usize * width as usize * 4..] {
                        *b = 0x00;
                    }
                }
                if let Err(err) = mmap.flush() {
                    eprintln!(
                        "[SCTK] Basic frame: failed to flush frame memory map: {}",
                        err
                    );
                }
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

    fn set_theme<T: Theme>(&mut self, theme: T) {
        self.theme = Box::new(theme)
    }

    fn set_title(&mut self, title: String) {
        self.title = Some(title);
    }
}

impl Drop for BasicFrame {
    fn drop(&mut self) {
        for ptr in self.pointers.drain(..) {
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
    if pointer.set_cursor(name, serial).is_err() {
        eprintln!("[SCTK] Basic frame: failed to set cursor");
    }
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
        Location::Button(UIButton::Maximize) => {
            if maximized {
                Some(FrameRequest::UnMaximize)
            } else {
                Some(FrameRequest::Maximize)
            }
        }
        Location::Button(UIButton::Minimize) => Some(FrameRequest::Minimize),
        _ => None,
    }
}

fn draw_buttons(
    canvas: &mut Canvas,
    width: u32,
    maximizable: bool,
    mouses: &[Location],
    theme: &Theme,
) {
    // draw up to 3 buttons, depending on the width of the window
    // color of the button depends on whether a pointer is on it, and the maximizable
    // button can be disabled
    // buttons are 24x16
    if width >= 24 + 2 * BUTTON_SPACE {
        // draw the red button
        let button_state = if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Close))
        {
            ButtonState::Hovered
        } else {
            ButtonState::Idle
        };
        let color = theme.get_close_button_color(button_state);
        let red_button = rectangle::Rectangle::new(
            (
                (width - 24 - BUTTON_SPACE) as usize,
                (HEADER_SIZE / 2 - 8) as usize,
            ),
            (24, 16),
            None,
            Some(color),
        );
        canvas.draw(&red_button);
    }

    if width >= 56 + 2 * BUTTON_SPACE {
        // draw the yellow button
        let button_state = if !maximizable {
            ButtonState::Disabled
        } else if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Maximize))
        {
            ButtonState::Hovered
        } else {
            ButtonState::Idle
        };
        let color = theme.get_maximize_button_color(button_state);

        let yellow_button = rectangle::Rectangle::new(
            (
                (width - 56 - BUTTON_SPACE) as usize,
                (HEADER_SIZE / 2 - 8) as usize,
            ),
            (24, 16),
            None,
            Some(color),
        );
        canvas.draw(&yellow_button);
    }

    if width >= 88 + 2 * BUTTON_SPACE {
        // draw the green button
        let button_state = if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Minimize))
        {
            ButtonState::Hovered
        } else {
            ButtonState::Idle
        };
        let color = theme.get_minimize_button_color(button_state);
        let green_button = rectangle::Rectangle::new(
            (
                (width - 88 - BUTTON_SPACE) as usize,
                (HEADER_SIZE / 2 - 8) as usize,
            ),
            (24, 16),
            None,
            Some(color),
        );
        canvas.draw(&green_button);
    }
}
