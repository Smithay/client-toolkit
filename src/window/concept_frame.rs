use std::cmp::max;
use std::io::Read;
use std::sync::{Arc, Mutex};

use andrew::line;
use andrew::shapes::rectangle;
use andrew::text;
use andrew::text::fontconfig;
use andrew::{Canvas, Endian};

use wayland_client::protocol::{
    wl_compositor, wl_pointer, wl_seat, wl_shm, wl_subcompositor, wl_subsurface, wl_surface,
};
use wayland_client::NewProxy;

use super::{ButtonState, Frame, FrameRequest, Theme};
use crate::surface;
use pointer::{AutoPointer, AutoThemer};
use utils::DoubleMemPool;

/*
 * Drawing theme definitions
 */

const BORDER_SIZE: u32 = 12;
const HEADER_SIZE: u32 = 30;
const BUTTON_SPACE: u32 = 10;

// Defining the theme
struct DefaultTheme;

impl Theme for DefaultTheme {
    // Used for header color
    fn get_primary_color(&self, active: bool) -> [u8; 4] {
        if active {
            [0xFF, 0xE6, 0xE6, 0xE6]
        } else {
            [0xFF, 0xDC, 0xDC, 0xDC]
        }
    }

    // Used for division line
    fn get_secondary_color(&self, active: bool) -> [u8; 4] {
        if active {
            [0xFF, 0x1E, 0x1E, 0x1E]
        } else {
            [0xFF, 0x78, 0x78, 0x78]
        }
    }

    fn get_close_button_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => [0xFF, 0xD9, 0x43, 0x52],
            _ => [0x00, 0x00, 0x00, 0x00],
        }
    }

    fn get_maximize_button_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => [0xFF, 0x2D, 0xCB, 0x70],
            _ => [0x00, 0x00, 0x00, 0x00],
        }
    }

    fn get_minimize_button_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => [0xFF, 0x3C, 0xAD, 0xE8],
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
    surface: wl_surface::WlSurface,
    subsurface: wl_subsurface::WlSubsurface,
}

impl Part {
    fn new(
        parent: &wl_surface::WlSurface,
        compositor: &wl_compositor::WlCompositor,
        subcompositor: &wl_subcompositor::WlSubcompositor,
    ) -> Part {
        let surface = compositor
            .create_surface(NewProxy::implement_dummy)
            .unwrap();
        let subsurface = subcompositor
            .get_subsurface(&surface, parent, NewProxy::implement_dummy)
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
    seat: wl_seat::WlSeat,
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
    fn find_surface(&self, surface: &wl_surface::WlSurface) -> Location {
        if surface.as_ref().equals(&self.parts[HEAD].surface.as_ref()) {
            Location::Head
        } else if surface.as_ref().equals(&self.parts[TOP].surface.as_ref()) {
            Location::Top
        } else if surface
            .as_ref()
            .equals(&self.parts[BOTTOM].surface.as_ref())
        {
            Location::Bottom
        } else if surface.as_ref().equals(&self.parts[LEFT].surface.as_ref()) {
            Location::Left
        } else if surface.as_ref().equals(&self.parts[RIGHT].surface.as_ref()) {
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
        && (x >= f64::from(w - HEADER_SIZE))
        && (x <= f64::from(w))
        && (y <= f64::from(HEADER_SIZE))
        && (y >= f64::from(0))
    {
        Location::Button(UIButton::Close)
    } else if (w >= 56 + 2 * BUTTON_SPACE)
        && (x >= f64::from(w - 2 * HEADER_SIZE))
        && (x <= f64::from(w - HEADER_SIZE))
        && (y <= f64::from(HEADER_SIZE))
        && (y >= f64::from(0))
    {
        Location::Button(UIButton::Maximize)
    } else if (w >= 88 + 2 * BUTTON_SPACE)
        && (x >= f64::from(w - 3 * HEADER_SIZE))
        && (x <= f64::from(w - 2 * HEADER_SIZE))
        && (y <= f64::from(HEADER_SIZE))
        && (y >= f64::from(0))
    {
        Location::Button(UIButton::Minimize)
    } else {
        Location::Head
    }
}

/// A clean, modern and stylish set of decorations
///
/// This class draws clean and modern decorations with
/// buttons inspired by breeze, material hover shade and
/// a white header background
pub struct ConceptFrame {
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
    base_surface: wl_surface::WlSurface,
}

impl Frame for ConceptFrame {
    type Error = ::std::io::Error;
    fn init(
        base_surface: &wl_surface::WlSurface,
        compositor: &wl_compositor::WlCompositor,
        subcompositor: &wl_subcompositor::WlSubcompositor,
        shm: &wl_shm::WlShm,
        implementation: Box<FnMut(FrameRequest, u32) + Send>,
    ) -> Result<ConceptFrame, ::std::io::Error> {
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
        Ok(ConceptFrame {
            inner,
            pools,
            active: false,
            hidden: false,
            pointers: Vec::new(),
            themer: AutoThemer::init(None, compositor.clone(), &shm),
            surface_version: compositor.as_ref().version(),
            theme: Box::new(DefaultTheme),
            title: None,
            font_data: None,
            base_surface: base_surface.clone(),
        })
    }

    fn new_seat(&mut self, seat: &wl_seat::WlSeat) {
        use self::wl_pointer::Event;
        let inner = self.inner.clone();
        let base_surface = self.base_surface.clone();
        let pointer = self.themer.theme_pointer_with_impl(
            seat,
            move |event, pointer: AutoPointer| {
                let data: &Mutex<PointerUserData> = pointer.as_ref().user_data().unwrap();
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
                            let scale =
                                surface::try_get_dpi_factor(&base_surface).unwrap_or(1) as u32;
                            change_pointer(&pointer, scale, data.location, Some(serial))
                        }
                    }
                    Event::Leave { serial, .. } => {
                        data.location = Location::None;
                        let scale = surface::try_get_dpi_factor(&base_surface).unwrap_or(1) as u32;
                        change_pointer(&pointer, scale, data.location, Some(serial));
                        (&mut *inner.implem.lock().unwrap())(FrameRequest::Refresh, 0);
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
                                let scale =
                                    surface::try_get_dpi_factor(&base_surface).unwrap_or(1) as u32;
                                change_pointer(&pointer, scale, data.location, None)
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

        let scale = surface::try_get_dpi_factor(&self.base_surface).unwrap_or(1);

        // Update dpi scaling factor.
        for p in &self.inner.parts {
            p.surface.set_buffer_scale(scale as i32);
        }

        let (width, height) = *(self.inner.size.lock().unwrap());

        let scale = scale as u32;

        let scaled_header = HEADER_SIZE * scale;
        let scaled_border = BORDER_SIZE * scale;
        let scaled_width = scale * width;
        let scaled_height = scale * height;

        {
            // grab the current pool
            let pool = match self.pools.pool() {
                Some(pool) => pool,
                None => return,
            };
            // resize the pool as appropriate
            let pxcount = (scaled_header * scaled_width)
                + max(
                    (scaled_width + 2 * scaled_border) * scaled_border,
                    (scaled_height + scaled_header) * scaled_border,
                );

            pool.resize(4 * pxcount as usize)
                .expect("I/O Error while redrawing the borders");

            // draw the white header bar
            {
                let mmap = pool.mmap();
                {
                    let color = self.theme.get_primary_color(self.active);

                    let mut header_canvas = Canvas::new(
                        &mut mmap[0..scaled_header as usize * scaled_width as usize * 4],
                        scaled_width as usize,
                        scaled_header as usize,
                        scaled_width as usize * 4,
                        Endian::native(),
                    );
                    header_canvas.clear();

                    let header_bar = rectangle::Rectangle::new(
                        (0, 0),
                        (scaled_width as usize, scaled_header as usize),
                        None,
                        Some(color),
                    );
                    header_canvas.draw(&header_bar);

                    draw_buttons(
                        &mut header_canvas,
                        width,
                        scale,
                        true,
                        &self
                            .pointers
                            .iter()
                            .flat_map(|p| {
                                if p.as_ref().is_alive() {
                                    let data: &Mutex<PointerUserData> =
                                        p.as_ref().user_data().unwrap();
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
                                .filter_map(|p| match p.extension() {
                                    Some(e) if e == "ttf" => Some(p),
                                    _ => None,
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
                                (0, (HEADER_SIZE as usize / 2 - 8) * scale as usize),
                                [0, 0, 0, 255],
                                font_data,
                                17.0 * scale as f32,
                                1.0 * scale as f32,
                                title,
                            );

                            // Check if text is bigger then the avaliable width
                            if (width as isize - 88 - 4 * BUTTON_SPACE as isize)
                                > (title_text.get_width() + BUTTON_SPACE as usize) as isize
                            {
                                title_text.pos.0 = scale as usize
                                    * ((width as usize) / 2 - (title_text.get_width() / 2));
                                // Adjust position for buttons if both compete for space
                                if (width as usize) / 2 + (title_text.get_width() / 2)
                                    > (width - 88 - 2 * 2 * BUTTON_SPACE) as usize
                                {
                                    title_text.pos.0 -= scale as usize
                                        * (((width as usize) / 2 + (title_text.get_width() / 2))
                                            - (width - 88 - 2 * 2 * BUTTON_SPACE) as usize);
                                }
                                header_canvas.draw(&title_text);
                            }
                        }
                    }
                }

                // For each pixel in borders
                {
                    for b in &mut mmap[scaled_header as usize * scaled_width as usize * 4..] {
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
                scaled_width as i32,
                scaled_header as i32,
                4 * scaled_width as i32,
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
                    scaled_width as i32,
                    scaled_header as i32,
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
                4 * (scaled_width * scaled_header) as i32,
                (scaled_width + 2 * scaled_border) as i32,
                scaled_border as i32,
                4 * (scaled_width + 2 * scaled_border) as i32,
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
                    (scaled_width + 2 * scaled_border) as i32,
                    scaled_border as i32,
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
                4 * (scaled_width * scaled_header) as i32,
                (scaled_width + 2 * scaled_border) as i32,
                scaled_border as i32,
                4 * (scaled_width + 2 * scaled_border) as i32,
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
                    (scaled_width + 2 * scaled_border) as i32,
                    scaled_border as i32,
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
                4 * (scaled_width * scaled_header) as i32,
                scaled_border as i32,
                (scaled_height + scaled_header) as i32,
                4 * (scaled_border as i32),
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
                    scaled_border as i32,
                    (scaled_height + scaled_header) as i32,
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
                4 * (scaled_width * scaled_header) as i32,
                scaled_border as i32,
                (scaled_height + scaled_header) as i32,
                4 * (scaled_border as i32),
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
                    scaled_border as i32,
                    (scaled_height + scaled_height) as i32,
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

impl Drop for ConceptFrame {
    fn drop(&mut self) {
        for ptr in self.pointers.drain(..) {
            if ptr.as_ref().version() >= 3 {
                ptr.release();
            }
        }
    }
}

fn change_pointer(pointer: &AutoPointer, scale: u32, location: Location, serial: Option<u32>) {
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
    if pointer.set_cursor_with_scale(name, scale, serial).is_err() {
        eprintln!("[SCTK] Basic frame: failed to set cursor");
    }
}

fn request_for_location(
    location: Location,
    seat: &wl_seat::WlSeat,
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
    scale: u32,
    maximizable: bool,
    mouses: &[Location],
    theme: &Theme,
) {
    let scale = scale as usize;

    // Draw seperator between header and window contents

    for i in 1..=scale {
        let y = HEADER_SIZE as usize * scale - i;
        let division_line = line::Line::new(
            (0, y),
            (width as usize * scale, y),
            theme.get_secondary_color(false),
            false,
        );
        canvas.draw(&division_line);
    }

    if width >= HEADER_SIZE {
        // Draw the red button
        let mut button_color = theme.get_close_button_icon_color(ButtonState::Idle);
        if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Close))
        {
            // Draw a red shading around close button if hovered over
            let red_shade = theme.get_close_button_color(ButtonState::Hovered);
            // Change the button color (yet to be drawn) to the hovered version
            button_color = theme.get_close_button_icon_color(ButtonState::Hovered);
            let red_hover = rectangle::Rectangle::new(
                ((width - HEADER_SIZE) as usize * scale, 0),
                (HEADER_SIZE as usize * scale, HEADER_SIZE as usize * scale),
                None,
                Some(red_shade),
            );
            canvas.draw(&red_hover);

            for i in 1..=scale {
                let y = HEADER_SIZE as usize * scale - i;
                let red_division_line = line::Line::new(
                    ((width - HEADER_SIZE) as usize * scale, y),
                    ((width) as usize * scale, y),
                    [
                        red_shade[0],
                        red_shade[1].saturating_sub(50),
                        red_shade[2].saturating_sub(50),
                        red_shade[3].saturating_sub(50),
                    ],
                    false,
                );
                canvas.draw(&red_division_line);
            }
        } else {
            // draw shading if around close button when idle
            let red_shade = theme.get_close_button_color(ButtonState::Idle);
            let red_hover = rectangle::Rectangle::new(
                ((width - HEADER_SIZE) as usize * scale, 0),
                (HEADER_SIZE as usize * scale, HEADER_SIZE as usize * scale),
                None,
                Some(red_shade),
            );
            canvas.draw(&red_hover);
            for i in 1..=scale {
                let y = HEADER_SIZE as usize * scale - i;
                let red_division_line = line::Line::new(
                    ((width - HEADER_SIZE) as usize * scale, y),
                    ((width) as usize * scale, y),
                    [
                        red_shade[0],
                        red_shade[1].saturating_sub(50),
                        red_shade[2].saturating_sub(50),
                        red_shade[3].saturating_sub(50),
                    ],
                    false,
                );
                canvas.draw(&red_division_line);
            }
        };

        // Draw cross to represent the close button
        for i in 0..2 * scale {
            let diagonal_line = line::Line::new(
                (
                    (width - HEADER_SIZE / 2 - 4) as usize * scale + i,
                    (HEADER_SIZE / 2 - 4) as usize * scale,
                ),
                (
                    (width - HEADER_SIZE / 2 + 4) as usize * scale,
                    (HEADER_SIZE / 2 + 4) as usize * scale - i,
                ),
                button_color,
                true,
            );
            canvas.draw(&diagonal_line);
            let diagonal_line = line::Line::new(
                (
                    (width - HEADER_SIZE / 2 - 4) as usize * scale,
                    (HEADER_SIZE / 2 - 4) as usize * scale + i,
                ),
                (
                    (width - HEADER_SIZE / 2 + 4) as usize * scale - i,
                    (HEADER_SIZE / 2 + 4) as usize * scale,
                ),
                button_color,
                true,
            );
            canvas.draw(&diagonal_line);
            let diagonal_line = line::Line::new(
                (
                    (width - HEADER_SIZE / 2 + 4) as usize * scale - i,
                    (HEADER_SIZE / 2 - 4) as usize * scale,
                ),
                (
                    (width - HEADER_SIZE / 2 - 4) as usize * scale,
                    (HEADER_SIZE / 2 + 4) as usize * scale - i,
                ),
                button_color,
                true,
            );
            canvas.draw(&diagonal_line);
            let diagonal_line = line::Line::new(
                (
                    (width - HEADER_SIZE / 2 + 4) as usize * scale,
                    (HEADER_SIZE / 2 - 4) as usize * scale + i,
                ),
                (
                    (width - HEADER_SIZE / 2 - 4) as usize * scale + i,
                    (HEADER_SIZE / 2 + 4) as usize * scale,
                ),
                button_color,
                true,
            );
            canvas.draw(&diagonal_line);
        }
    }

    if width >= 2 * HEADER_SIZE {
        // Draw the green button
        let mut button_color = theme.get_maximize_button_icon_color(ButtonState::Idle);
        if !maximizable {
        } else if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Maximize))
        {
            // Draw a green shading around maximize button if hovered over
            let green_shade = theme.get_maximize_button_color(ButtonState::Hovered);
            // Change the button color (yet to be drawn) to the hovered version
            button_color = theme.get_maximize_button_icon_color(ButtonState::Hovered);
            let green_hover = rectangle::Rectangle::new(
                ((width - 2 * HEADER_SIZE) as usize * scale, 0),
                (HEADER_SIZE as usize * scale, HEADER_SIZE as usize * scale),
                None,
                Some(green_shade),
            );
            canvas.draw(&green_hover);
            for i in 1..=scale {
                let y = HEADER_SIZE as usize * scale - i;
                let green_division_line = line::Line::new(
                    ((width - 2 * HEADER_SIZE) as usize * scale, y),
                    ((width - HEADER_SIZE) as usize * scale, y),
                    [
                        green_shade[0],
                        green_shade[1].saturating_sub(50),
                        green_shade[2].saturating_sub(50),
                        green_shade[3].saturating_sub(50),
                    ],
                    false,
                );

                canvas.draw(&green_division_line);
            }
        } else {
            // Draw a green shading around maximize button if idle
            let green_shade = theme.get_maximize_button_color(ButtonState::Idle);
            let green_hover = rectangle::Rectangle::new(
                ((width - 2 * HEADER_SIZE) as usize * scale, 0),
                (HEADER_SIZE as usize * scale, HEADER_SIZE as usize * scale),
                None,
                Some(green_shade),
            );

            canvas.draw(&green_hover);

            for i in 1..=scale {
                let y = HEADER_SIZE as usize * scale - i;
                let green_division_line = line::Line::new(
                    ((width - 2 * HEADER_SIZE) as usize * scale, y),
                    ((width - HEADER_SIZE) as usize * scale, y),
                    [
                        green_shade[0],
                        green_shade[1].saturating_sub(50),
                        green_shade[2].saturating_sub(50),
                        green_shade[3].saturating_sub(50),
                    ],
                    false,
                );
                canvas.draw(&green_division_line);
            }
        };

        for i in 0..3 * scale {
            let left_diagional = line::Line::new(
                (
                    (width - HEADER_SIZE - HEADER_SIZE / 2 - 4) as usize * scale - i,
                    (HEADER_SIZE / 2 + 2) as usize * scale,
                ),
                (
                    (width - HEADER_SIZE - HEADER_SIZE / 2) as usize * scale,
                    (HEADER_SIZE / 2 - 2) as usize * scale - i,
                ),
                button_color,
                true,
            );
            canvas.draw(&left_diagional);
            let right_diagional = line::Line::new(
                (
                    (width - HEADER_SIZE - HEADER_SIZE / 2 + 4) as usize * scale + i,
                    (HEADER_SIZE / 2 + 2) as usize * scale,
                ),
                (
                    (width - HEADER_SIZE - HEADER_SIZE / 2) as usize * scale,
                    (HEADER_SIZE / 2 - 2) as usize * scale - i,
                ),
                button_color,
                true,
            );
            canvas.draw(&right_diagional);
        }
    }

    if width >= 3 * HEADER_SIZE {
        // Draw the blue button
        let mut button_color = theme.get_minimize_button_icon_color(ButtonState::Idle);
        if mouses
            .iter()
            .any(|&l| l == Location::Button(UIButton::Minimize))
        {
            // Draw a blue shading around minimize button if hovered over
            let blue_shade = theme.get_minimize_button_color(ButtonState::Hovered);
            // Change the button color (yet to be drawn) to the hovered version
            button_color = theme.get_minimize_button_icon_color(ButtonState::Hovered);
            let blue_hover = rectangle::Rectangle::new(
                ((width - 3 * HEADER_SIZE) as usize * scale, 0),
                (HEADER_SIZE as usize * scale, HEADER_SIZE as usize * scale),
                None,
                Some(blue_shade),
            );
            canvas.draw(&blue_hover);
            for i in 1..=scale {
                let y = HEADER_SIZE as usize * scale - i;
                let blue_division_line = line::Line::new(
                    ((width - 3 * HEADER_SIZE) as usize * scale, y),
                    ((width - 2 * HEADER_SIZE) as usize * scale, y),
                    [
                        blue_shade[0],
                        blue_shade[1].saturating_sub(50),
                        blue_shade[2].saturating_sub(50),
                        blue_shade[3].saturating_sub(50),
                    ],
                    false,
                );
                canvas.draw(&blue_division_line);
            }
        } else {
            // Draw a blue shading around minimize button if idle
            let blue_shade = theme.get_minimize_button_color(ButtonState::Idle);
            let blue_hover = rectangle::Rectangle::new(
                ((width - 3 * HEADER_SIZE) as usize * scale, 0),
                (HEADER_SIZE as usize * scale, HEADER_SIZE as usize * scale),
                None,
                Some(blue_shade),
            );
            canvas.draw(&blue_hover);
            for i in 1..=scale {
                let y = HEADER_SIZE as usize * scale - i;
                let blue_division_line = line::Line::new(
                    ((width - 3 * HEADER_SIZE) as usize * scale, y),
                    ((width - 2 * HEADER_SIZE) as usize * scale, y),
                    [
                        blue_shade[0],
                        blue_shade[1].saturating_sub(50),
                        blue_shade[2].saturating_sub(50),
                        blue_shade[3].saturating_sub(50),
                    ],
                    false,
                );
                canvas.draw(&blue_division_line);
            }
        }

        for i in 0..3 * scale {
            let left_diagional = line::Line::new(
                (
                    (width - 2 * HEADER_SIZE - HEADER_SIZE / 2 - 4) as usize * scale - i,
                    (HEADER_SIZE / 2 - 3) as usize * scale,
                ),
                (
                    (width - 2 * HEADER_SIZE - HEADER_SIZE / 2) as usize * scale,
                    (HEADER_SIZE / 2 + 1) as usize * scale + i,
                ),
                button_color,
                true,
            );
            canvas.draw(&left_diagional);
            let right_diagional = line::Line::new(
                (
                    (width - 2 * HEADER_SIZE - HEADER_SIZE / 2 + 4) as usize * scale + i,
                    (HEADER_SIZE / 2 - 3) as usize * scale,
                ),
                (
                    (width - 2 * HEADER_SIZE - HEADER_SIZE / 2) as usize * scale,
                    (HEADER_SIZE / 2 + 1) as usize * scale + i,
                ),
                button_color,
                true,
            );
            canvas.draw(&right_diagional);
        }
    }
}
