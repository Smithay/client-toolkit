use std::cell::RefCell;
use std::cmp::max;
use std::io::Read;
use std::rc::Rc;

use andrew::line;
use andrew::shapes::rectangle;
use andrew::text;
use andrew::text::fontconfig;
use andrew::{Canvas, Endian};

use wayland_client::protocol::{
    wl_compositor, wl_pointer, wl_seat, wl_shm, wl_subcompositor, wl_subsurface, wl_surface,
};
use wayland_client::{Attached, DispatchData};

use super::{
    ARGBColor, ButtonColorSpec, ButtonState, ColorSpec, Frame, FrameRequest, State, WindowState,
};
use crate::seat::pointer::{ThemeManager, ThemeSpec, ThemedPointer};
use crate::shm::DoubleMemPool;

/*
 * Drawing theme definitions
 */

const BORDER_SIZE: u32 = 12;
const HEADER_SIZE: u32 = 30;

/// Configuration for ConceptFrame
#[derive(Clone, Debug)]
pub struct ConceptConfig {
    /// The primary color of the titlebar
    pub primary_color: ColorSpec,
    /// Secondary color of the theme
    ///
    /// Used for the division line between the titlebar and the content
    pub secondary_color: ColorSpec,
    /// Parameters of the "Close" (or "x") button
    ///
    /// (icon color, button color)
    ///
    /// if `None` the button will not be drawn
    pub close_button: Option<(ButtonColorSpec, ButtonColorSpec)>,
    /// Parameters of the "Maximize" (or "^") button
    ///
    /// (icon color, button color)
    ///
    /// if `None` the button will not be drawn
    pub maximize_button: Option<(ButtonColorSpec, ButtonColorSpec)>,
    /// Parameters of the "Minimize" (or "v") button
    ///
    /// (icon color, button color)
    ///
    /// if `None` the button will not be drawn
    pub minimize_button: Option<(ButtonColorSpec, ButtonColorSpec)>,
    /// Font configuration for the titlebar
    ///
    /// Font name and size. If set to `None`, the title is not drawn.
    pub title_font: Option<(String, f32)>,
    /// Color for drawing the title text
    pub title_color: ColorSpec,
}

impl Default for ConceptConfig {
    fn default() -> ConceptConfig {
        let icon_spec = ButtonColorSpec {
            idle: ColorSpec::identical([0xFF, 0x1E, 0x1E, 0x1E].into()),
            hovered: ColorSpec::identical([0xFF, 0x1E, 0x1E, 0x1E].into()),
            disabled: ColorSpec::invisible(),
        };

        ConceptConfig {
            primary_color: ColorSpec {
                active: [0xFF, 0xE6, 0xE6, 0xE6].into(),
                inactive: [0xFF, 0xDC, 0xDC, 0xDC].into(),
            },
            secondary_color: ColorSpec {
                active: [0xFF, 0x1E, 0x1E, 0x1E].into(),
                inactive: [0xFF, 0x78, 0x78, 0x78].into(),
            },
            close_button: Some((
                // icon
                icon_spec,
                // button background
                ButtonColorSpec {
                    idle: ColorSpec::invisible(),
                    hovered: ColorSpec::identical([0xFF, 0xD9, 0x43, 0x52].into()),
                    disabled: ColorSpec::invisible(),
                },
            )),
            maximize_button: Some((
                // icon
                icon_spec,
                // button background
                ButtonColorSpec {
                    idle: ColorSpec::invisible(),
                    hovered: ColorSpec::identical([0xFF, 0x2D, 0xCB, 0x70].into()),
                    disabled: ColorSpec::invisible(),
                },
            )),
            minimize_button: Some((
                // icon
                icon_spec,
                // button background
                ButtonColorSpec {
                    idle: ColorSpec::invisible(),
                    hovered: ColorSpec::identical([0xFF, 0x3C, 0xAD, 0xE8].into()),
                    disabled: ColorSpec::invisible(),
                },
            )),
            title_font: Some(("sans".into(), 17.0)),
            title_color: ColorSpec::identical([0xFF, 0x00, 0x00, 0x00].into()),
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
        compositor: &Attached<wl_compositor::WlCompositor>,
        subcompositor: &Attached<wl_subcompositor::WlSubcompositor>,
        inner: Option<Rc<RefCell<Inner>>>,
    ) -> Part {
        let surface = if let Some(inner) = inner {
            crate::surface::setup_surface(
                compositor.create_surface(),
                Some(move |dpi, surface: wl_surface::WlSurface, ddata: DispatchData| {
                    surface.set_buffer_scale(dpi);
                    surface.commit();
                    (&mut inner.borrow_mut().implem)(FrameRequest::Refresh, 0, ddata);
                }),
            )
        } else {
            crate::surface::setup_surface(
                compositor.create_surface(),
                Some(move |dpi, surface: wl_surface::WlSurface, _ddata: DispatchData| {
                    surface.set_buffer_scale(dpi);
                    surface.commit();
                }),
            )
        };

        let surface = surface.detach();

        let subsurface = subcompositor.get_subsurface(&surface, parent);

        Part { surface, subsurface: subsurface.detach() }
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
    parts: Vec<Part>,
    size: (u32, u32),
    resizable: bool,
    implem: Box<dyn FnMut(FrameRequest, u32, DispatchData)>,
    maximized: bool,
    buttons: (bool, bool, bool),
}

impl Inner {
    fn find_surface(&self, surface: &wl_surface::WlSurface) -> Location {
        if surface.as_ref().equals(&self.parts[HEAD].surface.as_ref()) {
            Location::Head
        } else if surface.as_ref().equals(&self.parts[TOP].surface.as_ref()) {
            Location::Top
        } else if surface.as_ref().equals(&self.parts[BOTTOM].surface.as_ref()) {
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

fn precise_location(
    old: Location,
    width: u32,
    x: f64,
    y: f64,
    buttons: (bool, bool, bool),
) -> Location {
    match old {
        Location::Head | Location::Button(_) => find_button(x, y, width, buttons),

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

fn find_button(x: f64, y: f64, w: u32, buttons: (bool, bool, bool)) -> Location {
    if (w >= HEADER_SIZE)
        && (x >= f64::from(w - HEADER_SIZE))
        && (x <= f64::from(w))
        && (y <= f64::from(HEADER_SIZE))
        && (y >= f64::from(0))
    {
        // first button
        match buttons {
            (true, _, _) => Location::Button(UIButton::Close),
            (false, true, _) => Location::Button(UIButton::Maximize),
            (false, false, true) => Location::Button(UIButton::Minimize),
            _ => Location::Head,
        }
    } else if (w >= 2 * HEADER_SIZE)
        && (x >= f64::from(w - 2 * HEADER_SIZE))
        && (x <= f64::from(w - HEADER_SIZE))
        && (y <= f64::from(HEADER_SIZE))
        && (y >= f64::from(0))
    {
        // second button
        match buttons {
            (true, true, _) => Location::Button(UIButton::Maximize),
            (false, true, true) => Location::Button(UIButton::Minimize),
            _ => Location::Head,
        }
    } else if (w >= 3 * HEADER_SIZE)
        && (x >= f64::from(w - 3 * HEADER_SIZE))
        && (x <= f64::from(w - 2 * HEADER_SIZE))
        && (y <= f64::from(HEADER_SIZE))
        && (y >= f64::from(0))
    {
        // third button
        match buttons {
            (true, true, true) => Location::Button(UIButton::Minimize),
            _ => Location::Head,
        }
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
    inner: Rc<RefCell<Inner>>,
    pools: DoubleMemPool,
    active: WindowState,
    hidden: bool,
    pointers: Vec<ThemedPointer>,
    themer: ThemeManager,
    surface_version: u32,
    config: ConceptConfig,
    title: Option<String>,
    font_data: Option<Vec<u8>>,
}

impl Frame for ConceptFrame {
    type Error = ::std::io::Error;
    type Config = ConceptConfig;
    fn init(
        base_surface: &wl_surface::WlSurface,
        compositor: &Attached<wl_compositor::WlCompositor>,
        subcompositor: &Attached<wl_subcompositor::WlSubcompositor>,
        shm: &Attached<wl_shm::WlShm>,
        implementation: Box<dyn FnMut(FrameRequest, u32, DispatchData)>,
    ) -> Result<ConceptFrame, ::std::io::Error> {
        let inner = Rc::new(RefCell::new(Inner {
            parts: vec![],
            size: (1, 1),
            resizable: true,
            implem: implementation,
            maximized: false,
            buttons: (true, true, true),
        }));

        inner.borrow_mut().parts = vec![
            Part::new(base_surface, compositor, subcompositor, Some(Rc::clone(&inner))),
            Part::new(base_surface, compositor, subcompositor, None),
            Part::new(base_surface, compositor, subcompositor, None),
            Part::new(base_surface, compositor, subcompositor, None),
            Part::new(base_surface, compositor, subcompositor, None),
        ];

        let my_inner = inner.clone();
        // Send a Refresh request on callback from DoubleMemPool as it will be fired when
        // None was previously returned from `pool()` and the draw was postponed
        let pools = DoubleMemPool::new(shm.clone(), move |ddata| {
            (&mut my_inner.borrow_mut().implem)(FrameRequest::Refresh, 0, ddata);
        })?;
        Ok(ConceptFrame {
            inner,
            pools,
            active: WindowState::Inactive,
            hidden: false,
            pointers: Vec::new(),
            themer: ThemeManager::init(ThemeSpec::System, compositor.clone(), shm.clone()),
            surface_version: compositor.as_ref().version(),
            config: ConceptConfig::default(),
            title: None,
            font_data: None,
        })
    }

    fn new_seat(&mut self, seat: &Attached<wl_seat::WlSeat>) {
        use self::wl_pointer::Event;
        let inner = self.inner.clone();
        let pointer = self.themer.theme_pointer_with_impl(
            seat,
            move |event, pointer: ThemedPointer, ddata: DispatchData| {
                let data: &RefCell<PointerUserData> = pointer.as_ref().user_data().get().unwrap();
                let mut data = data.borrow_mut();
                let mut inner = inner.borrow_mut();
                match event {
                    Event::Enter { serial, surface, surface_x, surface_y } => {
                        data.location = precise_location(
                            inner.find_surface(&surface),
                            inner.size.0,
                            surface_x,
                            surface_y,
                            inner.buttons,
                        );
                        data.position = (surface_x, surface_y);
                        if inner.resizable {
                            change_pointer(&pointer, data.location, Some(serial))
                        }
                    }
                    Event::Leave { serial, .. } => {
                        data.location = Location::None;
                        change_pointer(&pointer, data.location, Some(serial));
                        (&mut inner.implem)(FrameRequest::Refresh, 0, ddata);
                    }
                    Event::Motion { surface_x, surface_y, .. } => {
                        data.position = (surface_x, surface_y);
                        let newpos = precise_location(
                            data.location,
                            inner.size.0,
                            surface_x,
                            surface_y,
                            inner.buttons,
                        );
                        if newpos != data.location {
                            match (newpos, data.location) {
                                (Location::Button(_), _) | (_, Location::Button(_)) => {
                                    // pointer movement involves a button, request refresh
                                    (&mut inner.implem)(FrameRequest::Refresh, 0, ddata);
                                }
                                _ => (),
                            }
                            // we changed of part of the decoration, pointer image
                            // may need to be changed
                            data.location = newpos;
                            if inner.resizable {
                                change_pointer(&pointer, data.location, None)
                            }
                        }
                    }
                    Event::Button { serial, button, state, .. } => {
                        if state == wl_pointer::ButtonState::Pressed {
                            let request = match button {
                                // Left mouse button.
                                0x110 => request_for_location_on_lmb(
                                    &data,
                                    inner.maximized,
                                    inner.resizable,
                                ),
                                // Right mouse button.
                                0x111 => request_for_location_on_rmb(&data),
                                _ => None,
                            };

                            if let Some(request) = request {
                                (&mut inner.implem)(request, serial, ddata);
                            }
                        }
                    }
                    _ => {}
                }
            },
        );
        pointer.as_ref().user_data().set(|| {
            RefCell::new(PointerUserData {
                location: Location::None,
                position: (0.0, 0.0),
                seat: seat.detach(),
            })
        });
        self.pointers.push(pointer);
    }

    fn remove_seat(&mut self, seat: &wl_seat::WlSeat) {
        self.pointers.retain(|pointer| {
            let user_data = pointer.as_ref().user_data().get::<RefCell<PointerUserData>>().unwrap();
            let guard = user_data.borrow_mut();
            if &guard.seat == seat {
                pointer.release();
                false
            } else {
                true
            }
        });
    }

    fn set_states(&mut self, states: &[State]) -> bool {
        let mut inner = self.inner.borrow_mut();
        let mut need_redraw = false;
        // process active
        let new_active = if states.contains(&State::Activated) {
            WindowState::Active
        } else {
            WindowState::Inactive
        };
        need_redraw |= new_active != self.active;
        self.active = new_active;
        // process maximized
        let new_maximized = states.contains(&State::Maximized);
        need_redraw |= new_maximized != inner.maximized;
        inner.maximized = new_maximized;

        need_redraw
    }

    fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    fn set_resizable(&mut self, resizable: bool) {
        self.inner.borrow_mut().resizable = resizable;
    }

    fn resize(&mut self, newsize: (u32, u32)) {
        self.inner.borrow_mut().size = newsize;
    }

    fn redraw(&mut self) {
        let inner = self.inner.borrow_mut();

        if self.hidden {
            // don't draw the borders
            for p in &inner.parts {
                p.surface.attach(None, 0, 0);
                p.surface.commit();
            }
            return;
        }

        let scales: Vec<u32> = inner
            .parts
            .iter()
            .map(|part| crate::surface::get_surface_scale_factor(&part.surface) as u32)
            .collect();

        let (width, height) = inner.size;

        // Use header scale for all the thing.
        let header_scale = scales[HEAD];

        let scaled_header_height = HEADER_SIZE * header_scale;
        let scaled_header_width = width * header_scale;

        {
            // grab the current pool
            let pool = match self.pools.pool() {
                Some(pool) => pool,
                None => return,
            };
            let lr_surfaces_scale = max(scales[LEFT], scales[RIGHT]);
            let tp_surfaces_scale = max(scales[TOP], scales[BOTTOM]);

            // resize the pool as appropriate
            let pxcount = (scaled_header_height * scaled_header_width)
                + max(
                    (width + 2 * BORDER_SIZE) * BORDER_SIZE * tp_surfaces_scale * tp_surfaces_scale,
                    (height + HEADER_SIZE) * BORDER_SIZE * lr_surfaces_scale * lr_surfaces_scale,
                );

            pool.resize(4 * pxcount as usize).expect("I/O Error while redrawing the borders");

            // draw the white header bar
            {
                let mmap = pool.mmap();
                {
                    let color = self.config.primary_color.get_for(self.active).into();

                    let mut header_canvas = Canvas::new(
                        &mut mmap
                            [0..scaled_header_height as usize * scaled_header_width as usize * 4],
                        scaled_header_width as usize,
                        scaled_header_height as usize,
                        scaled_header_width as usize * 4,
                        Endian::native(),
                    );
                    header_canvas.clear();

                    let header_bar = rectangle::Rectangle::new(
                        (0, 0),
                        (scaled_header_width as usize, scaled_header_height as usize),
                        None,
                        Some(color),
                    );
                    header_canvas.draw(&header_bar);

                    draw_buttons(
                        &mut header_canvas,
                        width,
                        header_scale,
                        true,
                        self.active,
                        &self
                            .pointers
                            .iter()
                            .flat_map(|p| {
                                if p.as_ref().is_alive() {
                                    let data: &RefCell<PointerUserData> =
                                        p.as_ref().user_data().get().unwrap();
                                    Some(data.borrow().location)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<Location>>(),
                        &self.config,
                    );
                    if let Some((ref font_face, font_size)) = self.config.title_font {
                        if let Some(title) = self.title.clone() {
                            // If theres no stored font data, find the first ttf regular sans font and
                            // store it
                            if self.font_data.is_none() {
                                if let Some(font) = fontconfig::FontConfig::new()
                                    .unwrap()
                                    .get_regular_family_fonts(&font_face)
                                    .unwrap()
                                    .iter()
                                    .find(|p| p.extension().map(|e| e == "ttf").unwrap_or(false))
                                {
                                    let mut font_data = Vec::new();
                                    if let Ok(mut file) = ::std::fs::File::open(font) {
                                        match file.read_to_end(&mut font_data) {
                                            Ok(_) => self.font_data = Some(font_data),
                                            Err(err) => {
                                                log::error!("Could not read font file: {}", err)
                                            }
                                        }
                                    }
                                }
                            }

                            // Create text from stored title and font data
                            if let Some(ref font_data) = self.font_data {
                                let title_color = self.config.title_color.get_for(self.active);
                                let mut title_text = text::Text::new(
                                    (
                                        0,
                                        (HEADER_SIZE as usize / 2)
                                            .saturating_sub((font_size / 2.0).ceil() as usize)
                                            * header_scale as usize,
                                    ),
                                    title_color.into(),
                                    font_data,
                                    font_size * header_scale as f32,
                                    1.0,
                                    title,
                                );

                                let mut button_count = 0isize;
                                if self.config.close_button.is_some() {
                                    button_count += 1;
                                }
                                if self.config.maximize_button.is_some() {
                                    button_count += 1;
                                }
                                if self.config.minimize_button.is_some() {
                                    button_count += 1;
                                }

                                let scaled_button_size =
                                    HEADER_SIZE as isize * header_scale as isize;
                                let button_space = button_count * scaled_button_size;
                                let scaled_header_width = width as isize * header_scale as isize;

                                // Check if text is bigger then the available width
                                if (scaled_header_width - button_space)
                                    > (title_text.get_width() as isize + scaled_button_size)
                                {
                                    title_text.pos.0 =
                                        (scaled_header_width - button_space) as usize / 2
                                            - (title_text.get_width() / 2);
                                    header_canvas.draw(&title_text);
                                }
                            }
                        }
                    }
                }

                // For each pixel in borders
                {
                    for b in &mut mmap
                        [scaled_header_height as usize * scaled_header_width as usize * 4..]
                    {
                        *b = 0x00;
                    }
                }
                if let Err(err) = mmap.flush() {
                    log::error!("Failed to flush frame memory map: {}", err);
                }
            }

            // Create the buffers
            // -> head-subsurface
            let buffer = pool.buffer(
                0,
                scaled_header_width as i32,
                scaled_header_height as i32,
                4 * scaled_header_width as i32,
                wl_shm::Format::Argb8888,
            );
            inner.parts[HEAD].subsurface.set_position(0, -(HEADER_SIZE as i32));
            inner.parts[HEAD].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                inner.parts[HEAD].surface.damage_buffer(
                    0,
                    0,
                    scaled_header_width as i32,
                    scaled_header_height as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                inner.parts[HEAD].surface.damage(0, 0, width as i32, HEADER_SIZE as i32);
            }
            inner.parts[HEAD].surface.commit();

            // -> top-subsurface
            let buffer = pool.buffer(
                4 * (scaled_header_width * scaled_header_height) as i32,
                ((width + 2 * BORDER_SIZE) * scales[TOP]) as i32,
                (BORDER_SIZE * scales[TOP]) as i32,
                (4 * scales[TOP] * (width + 2 * BORDER_SIZE)) as i32,
                wl_shm::Format::Argb8888,
            );
            inner.parts[TOP]
                .subsurface
                .set_position(-(BORDER_SIZE as i32), -(HEADER_SIZE as i32 + BORDER_SIZE as i32));
            inner.parts[TOP].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                inner.parts[TOP].surface.damage_buffer(
                    0,
                    0,
                    ((width + 2 * BORDER_SIZE) * scales[TOP]) as i32,
                    (BORDER_SIZE * scales[TOP]) as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                inner.parts[TOP].surface.damage(
                    0,
                    0,
                    (width + 2 * BORDER_SIZE) as i32,
                    BORDER_SIZE as i32,
                );
            }
            inner.parts[TOP].surface.commit();

            // -> bottom-subsurface
            let buffer = pool.buffer(
                4 * (scaled_header_width * scaled_header_height) as i32,
                ((width + 2 * BORDER_SIZE) * scales[BOTTOM]) as i32,
                (BORDER_SIZE * scales[BOTTOM]) as i32,
                (4 * scales[BOTTOM] * (width + 2 * BORDER_SIZE)) as i32,
                wl_shm::Format::Argb8888,
            );
            inner.parts[BOTTOM].subsurface.set_position(-(BORDER_SIZE as i32), height as i32);
            inner.parts[BOTTOM].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                inner.parts[BOTTOM].surface.damage_buffer(
                    0,
                    0,
                    ((width + 2 * BORDER_SIZE) * scales[BOTTOM]) as i32,
                    (BORDER_SIZE * scales[BOTTOM]) as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                inner.parts[BOTTOM].surface.damage(
                    0,
                    0,
                    (width + 2 * BORDER_SIZE) as i32,
                    BORDER_SIZE as i32,
                );
            }
            inner.parts[BOTTOM].surface.commit();

            // -> left-subsurface
            let buffer = pool.buffer(
                4 * (scaled_header_width * scaled_header_height) as i32,
                (BORDER_SIZE * scales[LEFT]) as i32,
                ((height + HEADER_SIZE) * scales[LEFT]) as i32,
                4 * (BORDER_SIZE * scales[LEFT]) as i32,
                wl_shm::Format::Argb8888,
            );
            inner.parts[LEFT].subsurface.set_position(-(BORDER_SIZE as i32), -(HEADER_SIZE as i32));
            inner.parts[LEFT].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                inner.parts[LEFT].surface.damage_buffer(
                    0,
                    0,
                    (BORDER_SIZE * scales[LEFT]) as i32,
                    ((height + HEADER_SIZE) * scales[LEFT]) as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                inner.parts[LEFT].surface.damage(
                    0,
                    0,
                    BORDER_SIZE as i32,
                    (height + HEADER_SIZE) as i32,
                );
            }
            inner.parts[LEFT].surface.commit();

            // -> right-subsurface
            let buffer = pool.buffer(
                4 * (scaled_header_width * scaled_header_height) as i32,
                (BORDER_SIZE * scales[RIGHT]) as i32,
                ((height + HEADER_SIZE) * scales[RIGHT]) as i32,
                4 * (BORDER_SIZE * scales[RIGHT]) as i32,
                wl_shm::Format::Argb8888,
            );
            inner.parts[RIGHT].subsurface.set_position(width as i32, -(HEADER_SIZE as i32));
            inner.parts[RIGHT].surface.attach(Some(&buffer), 0, 0);
            if self.surface_version >= 4 {
                inner.parts[RIGHT].surface.damage_buffer(
                    0,
                    0,
                    (BORDER_SIZE * scales[RIGHT]) as i32,
                    ((height + HEADER_SIZE) * scales[RIGHT]) as i32,
                );
            } else {
                // surface is old and does not support damage_buffer, so we damage
                // in surface coordinates and hope it is not rescaled
                inner.parts[RIGHT].surface.damage(
                    0,
                    0,
                    BORDER_SIZE as i32,
                    (height + HEADER_SIZE) as i32,
                );
            }
            inner.parts[RIGHT].surface.commit();
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

    fn set_config(&mut self, config: ConceptConfig) {
        self.config = config;
        let mut inner = self.inner.borrow_mut();
        inner.buttons = (
            self.config.close_button.is_some(),
            self.config.maximize_button.is_some(),
            self.config.minimize_button.is_some(),
        );
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

fn change_pointer(pointer: &ThemedPointer, location: Location, serial: Option<u32>) {
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
        log::error!("Failed to set cursor");
    }
}

fn request_for_location_on_lmb(
    pointer_data: &PointerUserData,
    maximized: bool,
    resizable: bool,
) -> Option<FrameRequest> {
    use wayland_protocols::xdg_shell::client::xdg_toplevel::ResizeEdge;
    match pointer_data.location {
        Location::Top if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::Top))
        }
        Location::TopLeft if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::TopLeft))
        }
        Location::Left if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::Left))
        }
        Location::BottomLeft if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::BottomLeft))
        }
        Location::Bottom if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::Bottom))
        }
        Location::BottomRight if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::BottomRight))
        }
        Location::Right if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::Right))
        }
        Location::TopRight if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::TopRight))
        }
        Location::Head => Some(FrameRequest::Move(pointer_data.seat.clone())),
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

fn request_for_location_on_rmb(pointer_data: &PointerUserData) -> Option<FrameRequest> {
    match pointer_data.location {
        Location::Head | Location::Button(_) => Some(FrameRequest::ShowMenu(
            pointer_data.seat.clone(),
            pointer_data.position.0 as i32,
            // We must offset it by header size for precise position.
            pointer_data.position.1 as i32 - HEADER_SIZE as i32,
        )),
        _ => None,
    }
}

// average of the two colors, approximately taking into account gamma correction
// result is as transparent as the most transparent color
fn mix_colors(x: ARGBColor, y: ARGBColor) -> ARGBColor {
    #[inline]
    fn gamma_mix(x: u8, y: u8) -> u8 {
        let x = x as f32 / 255.0;
        let y = y as f32 / 255.0;
        let z = ((x * x + y * y) / 2.0).sqrt();
        (z * 255.0) as u8
    }

    ARGBColor {
        a: x.a.min(y.a),
        r: gamma_mix(x.r, y.r),
        g: gamma_mix(x.g, y.g),
        b: gamma_mix(x.b, y.b),
    }
}

fn draw_buttons(
    canvas: &mut Canvas,
    width: u32,
    scale: u32,
    maximizable: bool,
    state: WindowState,
    mouses: &[Location],
    config: &ConceptConfig,
) {
    let scale = scale as usize;

    // Draw seperator between header and window contents
    let line_color = config.secondary_color.get_for(state);
    for i in 1..=scale {
        let y = HEADER_SIZE as usize * scale - i;
        let division_line =
            line::Line::new((0, y), (width as usize * scale, y), line_color.into(), false);
        canvas.draw(&division_line);
    }

    let mut drawn_buttons = 0usize;

    if width >= HEADER_SIZE {
        if let Some((ref icon_config, ref btn_config)) = config.close_button {
            // Draw the close button
            let btn_state = if mouses.iter().any(|&l| l == Location::Button(UIButton::Close)) {
                ButtonState::Hovered
            } else {
                ButtonState::Idle
            };

            let icon_color = icon_config.get_for(btn_state).get_for(state);
            let button_color = btn_config.get_for(btn_state).get_for(state);

            draw_button(canvas, 0, scale, button_color, mix_colors(button_color, line_color));
            draw_icon(canvas, 0, scale, icon_color, Icon::Close);
            drawn_buttons += 1;
        }
    }

    if width as usize >= (drawn_buttons + 1) * HEADER_SIZE as usize {
        if let Some((ref icon_config, ref btn_config)) = config.maximize_button {
            let btn_state = if !maximizable {
                ButtonState::Disabled
            } else if mouses.iter().any(|&l| l == Location::Button(UIButton::Maximize)) {
                ButtonState::Hovered
            } else {
                ButtonState::Idle
            };

            let icon_color = icon_config.get_for(btn_state).get_for(state);
            let button_color = btn_config.get_for(btn_state).get_for(state);

            draw_button(
                canvas,
                drawn_buttons * HEADER_SIZE as usize,
                scale,
                button_color,
                mix_colors(button_color, line_color),
            );
            draw_icon(
                canvas,
                drawn_buttons * HEADER_SIZE as usize,
                scale,
                icon_color,
                Icon::Maximize,
            );
            drawn_buttons += 1;
        }
    }

    if width as usize >= (drawn_buttons + 1) * HEADER_SIZE as usize {
        if let Some((ref icon_config, ref btn_config)) = config.minimize_button {
            let btn_state = if mouses.iter().any(|&l| l == Location::Button(UIButton::Minimize)) {
                ButtonState::Hovered
            } else {
                ButtonState::Idle
            };

            let icon_color = icon_config.get_for(btn_state).get_for(state);
            let button_color = btn_config.get_for(btn_state).get_for(state);

            draw_button(
                canvas,
                drawn_buttons * HEADER_SIZE as usize,
                scale,
                button_color,
                mix_colors(button_color, line_color),
            );
            draw_icon(
                canvas,
                drawn_buttons * HEADER_SIZE as usize,
                scale,
                icon_color,
                Icon::Minimize,
            );
        }
    }
}

enum Icon {
    Close,
    Maximize,
    Minimize,
}

fn draw_button(
    canvas: &mut Canvas,
    x_offset: usize,
    scale: usize,
    btn_color: ARGBColor,
    line_color: ARGBColor,
) {
    let h = HEADER_SIZE as usize;
    let x_start = canvas.width / scale - h - x_offset;
    // main square
    canvas.draw(&rectangle::Rectangle::new(
        (x_start * scale, 0),
        (h * scale, (h - 1) * scale),
        None,
        Some(btn_color.into()),
    ));
    // separation line
    canvas.draw(&rectangle::Rectangle::new(
        (x_start * scale, (h - 1) * scale),
        (h * scale, scale),
        None,
        Some(line_color.into()),
    ));
}

fn draw_icon(
    canvas: &mut Canvas,
    x_offset: usize,
    scale: usize,
    icon_color: ARGBColor,
    icon: Icon,
) {
    let h = HEADER_SIZE as usize;
    let cx = canvas.width / scale - h / 2 - x_offset;
    let cy = h / 2;
    let s = scale;

    match icon {
        Icon::Close => {
            // Draw cross to represent the close button
            for i in 0..2 * scale {
                canvas.draw(&line::Line::new(
                    ((cx - 4) * s + i, (cy - 4) * s),
                    ((cx + 4) * s, (cy + 4) * s - i),
                    icon_color.into(),
                    true,
                ));
                canvas.draw(&line::Line::new(
                    ((cx - 4) * s, (cy - 4) * s + i),
                    ((cx + 4) * s - i, (cy + 4) * s),
                    icon_color.into(),
                    true,
                ));
                canvas.draw(&line::Line::new(
                    ((cx + 4) * s - i, (cy - 4) * s),
                    ((cx - 4) * s, (cy + 4) * s - i),
                    icon_color.into(),
                    true,
                ));
                canvas.draw(&line::Line::new(
                    ((cx + 4) * s, (cy - 4) * s + i),
                    ((cx - 4) * s + i, (cy + 4) * s),
                    icon_color.into(),
                    true,
                ));
            }
        }
        Icon::Maximize => {
            for i in 0..3 * scale {
                canvas.draw(&line::Line::new(
                    ((cx - 4) * s - i, (cy + 2) * s),
                    (cx * s, (cy - 2) * s - i),
                    icon_color.into(),
                    true,
                ));
                canvas.draw(&line::Line::new(
                    ((cx + 4) * s + i, (cy + 2) * s),
                    (cx * s, (cy - 2) * s - i),
                    icon_color.into(),
                    true,
                ));
            }
        }
        Icon::Minimize => {
            for i in 0..3 * scale {
                canvas.draw(&line::Line::new(
                    ((cx - 4) * s - i, (cy - 3) * s),
                    (cx * s, (cy + 1) * s + i),
                    icon_color.into(),
                    true,
                ));
                canvas.draw(&line::Line::new(
                    ((cx + 4) * s + i, (cy - 3) * s),
                    (cx * s, (cy + 1) * s + i),
                    icon_color.into(),
                    true,
                ));
            }
        }
    }
}
