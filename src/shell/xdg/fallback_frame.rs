//! The default fallback frame which is intended to show some very basic derocations.

use std::mem;
use std::sync::Arc;
use std::time::Duration;
use std::{error::Error, num::NonZeroU32};

use crate::reexports::client::{
    protocol::{wl_shm, wl_subsurface::WlSubsurface, wl_surface::WlSurface},
    Dispatch, Proxy, QueueHandle,
};
use crate::reexports::csd_frame::{
    DecorationsFrame, FrameAction, FrameClick, ResizeEdge, WindowManagerCapabilities, WindowState,
};

use crate::{
    compositor::SurfaceData,
    seat::pointer::CursorIcon,
    shell::WaylandSurface,
    shm::{slot::SlotPool, Shm},
    subcompositor::{SubcompositorState, SubsurfaceData},
};

use wayland_backend::client::ObjectId;

/// The size of the header bar.
const HEADER_SIZE: u32 = 24;

/// The size of the border.
const BORDER_SIZE: u32 = 4;

const HEADER: usize = 0;
const TOP_BORDER: usize = 1;
const RIGHT_BORDER: usize = 2;
const BOTTOM_BORDER: usize = 3;
const LEFT_BORDER: usize = 4;

const BTN_ICON_COLOR: u32 = 0xFFCCCCCC;
const BTN_HOVER_BG: u32 = 0xFF808080;

const PRIMARY_COLOR_ACTIVE: u32 = 0xFF3A3A3A;
const PRIMARY_COLOR_INACTIVE: u32 = 0xFF242424;

/// The default ugly frame.
#[derive(Debug)]
pub struct FallbackFrame<State> {
    /// The parent surface.
    parent: WlSurface,

    /// The latest window state.
    state: WindowState,

    /// The wm capabilities.
    wm_capabilities: WindowManagerCapabilities,

    /// Whether the frame is resizable.
    resizable: bool,

    /// Whether the frame is waiting for redraw.
    dirty: bool,

    /// The location of the mouse.
    mouse_location: Location,

    /// The location of the mouse.
    mouse_coords: (i32, i32),

    /// The frame rendering data. When `None` the frame is hidden.
    render_data: Option<FrameRenderData>,

    /// Whether the frame should sync with the parent.
    ///
    /// This should happen in reaction to scale or resize changes.
    should_sync: bool,

    /// The active scale factor of the frame.
    scale_factor: f64,

    /// The frame queue handle.
    queue_handle: QueueHandle<State>,

    /// The memory pool to use for drawing.
    pool: SlotPool,

    /// The subcompositor.
    subcompositor: Arc<SubcompositorState>,

    /// Buttons state.
    buttons: [Option<UIButton>; 3],
}

impl<State> FallbackFrame<State>
where
    State: Dispatch<WlSurface, SurfaceData> + Dispatch<WlSubsurface, SubsurfaceData> + 'static,
{
    pub fn new(
        parent: &impl WaylandSurface,
        shm: &Shm,
        subcompositor: Arc<SubcompositorState>,
        queue_handle: QueueHandle<State>,
    ) -> Result<Self, Box<dyn Error>> {
        let parent = parent.wl_surface().clone();
        let pool = SlotPool::new(1, shm)?;
        let render_data = Some(FrameRenderData::new(&parent, &subcompositor, &queue_handle));

        let wm_capabilities = WindowManagerCapabilities::all();
        Ok(Self {
            parent,
            resizable: true,
            state: WindowState::empty(),
            wm_capabilities,
            dirty: true,
            scale_factor: 1.,
            pool,
            should_sync: true,
            queue_handle,
            subcompositor,
            render_data,
            mouse_location: Location::None,
            mouse_coords: (0, 0),
            buttons: Self::supported_buttons(wm_capabilities),
        })
    }

    fn supported_buttons(wm_capabilities: WindowManagerCapabilities) -> [Option<UIButton>; 3] {
        let maximize = wm_capabilities
            .contains(WindowManagerCapabilities::MAXIMIZE)
            .then_some(UIButton::Maximize);
        let minimize = wm_capabilities
            .contains(WindowManagerCapabilities::MINIMIZE)
            .then_some(UIButton::Minimize);
        [Some(UIButton::Close), maximize, minimize]
    }

    fn precise_location(
        buttons: &[Option<UIButton>],
        old: Location,
        width: u32,
        x: f64,
        y: f64,
    ) -> Location {
        match old {
            Location::Head | Location::Button(_) => Self::find_button(buttons, x, y, width),

            Location::Top | Location::TopLeft | Location::TopRight => {
                if x <= f64::from(BORDER_SIZE) {
                    Location::TopLeft
                } else if x >= f64::from(width - BORDER_SIZE) {
                    Location::TopRight
                } else {
                    Location::Top
                }
            }

            Location::Bottom | Location::BottomLeft | Location::BottomRight => {
                if x <= f64::from(BORDER_SIZE) {
                    Location::BottomLeft
                } else if x >= f64::from(width - BORDER_SIZE) {
                    Location::BottomRight
                } else {
                    Location::Bottom
                }
            }

            other => other,
        }
    }

    fn find_button(buttons: &[Option<UIButton>], x: f64, y: f64, w: u32) -> Location {
        for (idx, &button) in buttons.iter().flatten().enumerate() {
            let idx = idx as u32;
            if w >= (idx + 1) * HEADER_SIZE
                && x >= f64::from(w - (idx + 1) * HEADER_SIZE)
                && x <= f64::from(w - idx * HEADER_SIZE)
                && y <= f64::from(HEADER_SIZE)
                && y >= f64::from(0)
            {
                return Location::Button(button);
            }
        }

        Location::Head
    }

    #[inline]
    fn part_index_for_surface(&mut self, surface_id: &ObjectId) -> Option<usize> {
        self.render_data.as_ref()?.parts.iter().position(|part| &part.surface.id() == surface_id)
    }

    fn draw_buttons(
        buttons: &[Option<UIButton>],
        canvas: &mut [u8],
        width: u32,
        scale: u32,
        is_active: bool,
        mouse_location: &Location,
    ) {
        let scale = scale as usize;
        for (idx, &button) in buttons.iter().flatten().enumerate() {
            if width >= (idx + 1) as u32 * HEADER_SIZE {
                if is_active && mouse_location == &Location::Button(button) {
                    Self::draw_button(
                        canvas,
                        idx * HEADER_SIZE as usize,
                        scale,
                        width as usize,
                        BTN_HOVER_BG.to_le_bytes(),
                    );
                }
                Self::draw_icon(
                    canvas,
                    width as usize,
                    idx * HEADER_SIZE as usize,
                    scale,
                    BTN_ICON_COLOR.to_le_bytes(),
                    button,
                );
            }
        }
    }

    fn draw_button(
        canvas: &mut [u8],
        x_offset: usize,
        scale: usize,
        width: usize,
        btn_color: [u8; 4],
    ) {
        let h = HEADER_SIZE as usize;
        let x_start = width - h - x_offset;
        // main square
        for y in 0..h * scale {
            let canvas = &mut canvas
                [(x_start + y * width) * 4 * scale..(x_start + y * width + h) * scale * 4];
            for pixel in canvas.chunks_exact_mut(4) {
                pixel[0] = btn_color[0];
                pixel[1] = btn_color[1];
                pixel[2] = btn_color[2];
                pixel[3] = btn_color[3];
            }
        }
    }

    fn draw_icon(
        canvas: &mut [u8],
        width: usize,
        x_offset: usize,
        scale: usize,
        icon_color: [u8; 4],
        icon: UIButton,
    ) {
        let h = HEADER_SIZE as usize;
        let sh = scale * h;
        let x_start = width - h - x_offset;

        match icon {
            UIButton::Close => {
                // Draw black rectangle
                for y in sh / 4..3 * sh / 4 {
                    let line = &mut canvas[(x_start + y * width + h / 4) * 4 * scale
                        ..(x_start + y * width + 3 * h / 4) * 4 * scale];
                    for pixel in line.chunks_exact_mut(4) {
                        pixel[0] = icon_color[0];
                        pixel[1] = icon_color[1];
                        pixel[2] = icon_color[2];
                        pixel[3] = icon_color[3];
                    }
                }
            }
            UIButton::Maximize => {
                // Draw an empty rectangle
                for y in 2 * sh / 8..3 * sh / 8 {
                    let line = &mut canvas[(x_start + y * width + h / 4) * 4 * scale
                        ..(x_start + y * width + 3 * h / 4) * 4 * scale];
                    for pixel in line.chunks_exact_mut(4) {
                        pixel[0] = icon_color[0];
                        pixel[1] = icon_color[1];
                        pixel[2] = icon_color[2];
                        pixel[3] = icon_color[3];
                    }
                }
                for y in 3 * sh / 8..5 * sh / 8 {
                    let line = &mut canvas[(x_start + y * width + 2 * h / 8) * 4 * scale
                        ..(x_start + y * width + 3 * h / 8) * 4 * scale];
                    for pixel in line.chunks_exact_mut(4) {
                        pixel[0] = icon_color[0];
                        pixel[1] = icon_color[1];
                        pixel[2] = icon_color[2];
                        pixel[3] = icon_color[3];
                    }
                    let line = &mut canvas[(x_start + y * width + 5 * h / 8) * 4 * scale
                        ..(x_start + y * width + 6 * h / 8) * 4 * scale];
                    for pixel in line.chunks_exact_mut(4) {
                        pixel[0] = icon_color[0];
                        pixel[1] = icon_color[1];
                        pixel[2] = icon_color[2];
                        pixel[3] = icon_color[3];
                    }
                }
                for y in 5 * sh / 8..6 * sh / 8 {
                    let line = &mut canvas[(x_start + y * width + h / 4) * 4 * scale
                        ..(x_start + y * width + 3 * h / 4) * 4 * scale];
                    for pixel in line.chunks_exact_mut(4) {
                        pixel[0] = icon_color[0];
                        pixel[1] = icon_color[1];
                        pixel[2] = icon_color[2];
                        pixel[3] = icon_color[3];
                    }
                }
            }
            UIButton::Minimize => {
                // Draw an underline
                for y in 5 * sh / 8..3 * sh / 4 {
                    let line = &mut canvas[(x_start + y * width + h / 4) * 4 * scale
                        ..(x_start + y * width + 3 * h / 4) * 4 * scale];
                    for pixel in line.chunks_exact_mut(4) {
                        pixel[0] = icon_color[0];
                        pixel[1] = icon_color[1];
                        pixel[2] = icon_color[2];
                        pixel[3] = icon_color[3];
                    }
                }
            }
        }
    }
}

impl<State> DecorationsFrame for FallbackFrame<State>
where
    State: Dispatch<WlSurface, SurfaceData> + Dispatch<WlSubsurface, SubsurfaceData> + 'static,
{
    fn set_scaling_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        self.dirty = true;
        self.should_sync = true;
    }

    fn on_click(
        &mut self,
        _timestamp: Duration,
        click: FrameClick,
        pressed: bool,
    ) -> Option<FrameAction> {
        // Handle alternate click before everything else.
        if click == FrameClick::Alternate {
            return if Location::Head != self.mouse_location
                || !self.wm_capabilities.contains(WindowManagerCapabilities::WINDOW_MENU)
            {
                None
            } else {
                Some(FrameAction::ShowMenu(
                    self.mouse_coords.0,
                    self.mouse_coords.1 - HEADER_SIZE as i32,
                ))
            };
        }

        let resize = pressed && self.resizable;
        match self.mouse_location {
            Location::Head if pressed => Some(FrameAction::Move),
            Location::Button(UIButton::Close) if !pressed => Some(FrameAction::Close),
            Location::Button(UIButton::Minimize) if !pressed => Some(FrameAction::Minimize),
            Location::Button(UIButton::Maximize)
                if !pressed && !self.state.contains(WindowState::MAXIMIZED) =>
            {
                Some(FrameAction::Maximize)
            }
            Location::Button(UIButton::Maximize)
                if !pressed && self.state.contains(WindowState::MAXIMIZED) =>
            {
                Some(FrameAction::UnMaximize)
            }
            Location::Top if resize => Some(FrameAction::Resize(ResizeEdge::Top)),
            Location::TopLeft if resize => Some(FrameAction::Resize(ResizeEdge::TopLeft)),
            Location::Left if resize => Some(FrameAction::Resize(ResizeEdge::Left)),
            Location::BottomLeft if resize => Some(FrameAction::Resize(ResizeEdge::BottomLeft)),
            Location::Bottom if resize => Some(FrameAction::Resize(ResizeEdge::Bottom)),
            Location::BottomRight if resize => Some(FrameAction::Resize(ResizeEdge::BottomRight)),
            Location::Right if resize => Some(FrameAction::Resize(ResizeEdge::Right)),
            Location::TopRight if resize => Some(FrameAction::Resize(ResizeEdge::TopRight)),
            _ => None,
        }
    }

    fn click_point_moved(
        &mut self,
        _timestamp: Duration,
        surface_id: &ObjectId,
        x: f64,
        y: f64,
    ) -> Option<CursorIcon> {
        let part_index = self.part_index_for_surface(surface_id)?;
        let location = match part_index {
            LEFT_BORDER => Location::Left,
            RIGHT_BORDER => Location::Right,
            BOTTOM_BORDER => Location::Bottom,
            TOP_BORDER => Location::Top,
            _ => Location::Head,
        };

        let old_location = self.mouse_location;
        self.mouse_coords = (x as i32, y as i32);
        self.mouse_location = Self::precise_location(
            &self.buttons,
            location,
            self.render_data.as_ref().unwrap().parts[part_index].width,
            x,
            y,
        );

        // Set dirty if we moved the cursor between the buttons.
        self.dirty |= (matches!(old_location, Location::Button(_))
            || matches!(self.mouse_location, Location::Button(_)))
            && old_location != self.mouse_location;

        Some(match self.mouse_location {
            Location::Top => CursorIcon::NResize,
            Location::TopRight => CursorIcon::NeResize,
            Location::Right => CursorIcon::EResize,
            Location::BottomRight => CursorIcon::SeResize,
            Location::Bottom => CursorIcon::SResize,
            Location::BottomLeft => CursorIcon::SwResize,
            Location::Left => CursorIcon::WResize,
            Location::TopLeft => CursorIcon::NwResize,
            _ => CursorIcon::Default,
        })
    }

    fn click_point_left(&mut self) {
        self.mouse_location = Location::None;
        self.dirty = true;
    }

    fn set_hidden(&mut self, hidden: bool) {
        if self.is_hidden() == hidden {
            return;
        }

        if hidden {
            self.render_data = None;
        } else {
            let _ = self.pool.resize(1);
            self.render_data =
                Some(FrameRenderData::new(&self.parent, &self.subcompositor, &self.queue_handle));
        }
    }

    fn set_resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
    }

    fn update_state(&mut self, state: WindowState) {
        let difference = self.state.symmetric_difference(state);
        self.state = state;
        self.dirty |= !difference
            .intersection(WindowState::ACTIVATED | WindowState::FULLSCREEN | WindowState::MAXIMIZED)
            .is_empty();
    }

    fn resize(&mut self, width: NonZeroU32, height: NonZeroU32) {
        let parts = &mut self.render_data.as_mut().expect("trying to resize hidden frame").parts;

        let width = width.get();
        let height = height.get();

        parts[HEADER].width = width;

        parts[TOP_BORDER].width = width + 2 * BORDER_SIZE;

        parts[BOTTOM_BORDER].width = width + 2 * BORDER_SIZE;
        parts[BOTTOM_BORDER].pos.1 = height as i32;

        parts[LEFT_BORDER].height = height + HEADER_SIZE;

        parts[RIGHT_BORDER].height = parts[LEFT_BORDER].height;
        parts[RIGHT_BORDER].pos.0 = width as i32;

        self.dirty = true;
        self.should_sync = true;
    }

    fn subtract_borders(
        &self,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> (Option<NonZeroU32>, Option<NonZeroU32>) {
        if self.state.contains(WindowState::FULLSCREEN) || self.render_data.is_none() {
            (Some(width), Some(height))
        } else {
            (
                NonZeroU32::new(width.get().saturating_sub(2 * BORDER_SIZE)),
                NonZeroU32::new(height.get().saturating_sub(HEADER_SIZE + 2 * BORDER_SIZE)),
            )
        }
    }

    fn add_borders(&self, width: u32, height: u32) -> (u32, u32) {
        if self.state.contains(WindowState::FULLSCREEN) || self.render_data.is_none() {
            (width, height)
        } else {
            (width + 2 * BORDER_SIZE, height + (HEADER_SIZE + 2 * BORDER_SIZE))
        }
    }

    fn is_hidden(&self) -> bool {
        self.render_data.is_none()
    }

    fn location(&self) -> (i32, i32) {
        if self.state.contains(WindowState::FULLSCREEN) || self.is_hidden() {
            (0, 0)
        } else {
            self.render_data.as_ref().unwrap().parts[TOP_BORDER].pos
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn draw(&mut self) -> bool {
        let render_data = match self.render_data.as_mut() {
            Some(render_data) => render_data,
            None => return false,
        };

        // Reset the dirty bit and sync option.
        self.dirty = false;
        let should_sync = mem::take(&mut self.should_sync);

        if self.state.contains(WindowState::FULLSCREEN) {
            // Don't draw the decorations for the full screen surface.
            for part in &render_data.parts {
                part.surface.attach(None, 0, 0);
                part.surface.commit();
            }
            return should_sync;
        }

        let is_active = self.state.contains(WindowState::ACTIVATED);
        let fill_color =
            if is_active { PRIMARY_COLOR_ACTIVE } else { PRIMARY_COLOR_INACTIVE }.to_le_bytes();

        for (idx, part) in render_data.parts.iter().enumerate() {
            // We don't support fractinal scaling here, so round up.
            let scale = self.scale_factor.ceil() as i32;

            let (buffer, canvas) = match self.pool.create_buffer(
                part.width as i32 * scale,
                part.height as i32 * scale,
                part.width as i32 * 4 * scale,
                wl_shm::Format::Argb8888,
            ) {
                Ok((buffer, canvas)) => (buffer, canvas),
                Err(_) => continue,
            };

            // Fill the canvas.
            for pixel in canvas.chunks_exact_mut(4) {
                pixel[0] = fill_color[0];
                pixel[1] = fill_color[1];
                pixel[2] = fill_color[2];
                pixel[3] = fill_color[3];
            }

            // Draw the buttons for the header.
            if idx == HEADER {
                Self::draw_buttons(
                    &self.buttons,
                    canvas,
                    part.width,
                    scale as u32,
                    is_active,
                    &self.mouse_location,
                );
            }

            part.surface.set_buffer_scale(scale);
            if should_sync {
                part.subsurface.set_sync();
            } else {
                part.subsurface.set_desync();
            }

            // Update the subsurface position.
            part.subsurface.set_position(part.pos.0, part.pos.1);

            buffer.attach_to(&part.surface).expect("failed to attach the buffer");
            if part.surface.version() >= 4 {
                part.surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
            } else {
                part.surface.damage(0, 0, i32::MAX, i32::MAX);
            }

            part.surface.commit();
        }

        should_sync
    }

    fn update_wm_capabilities(&mut self, capabilities: WindowManagerCapabilities) {
        self.dirty |= self.wm_capabilities != capabilities;
        self.wm_capabilities = capabilities;
        self.buttons = Self::supported_buttons(capabilities);
    }

    fn set_title(&mut self, _: impl Into<String>) {}
}

/// Inner state to simplify dropping.
#[derive(Debug)]
struct FrameRenderData {
    /// The header subsurface.
    parts: [FramePart; 5],
}

impl FrameRenderData {
    fn new<State>(
        parent: &WlSurface,
        subcompositor: &SubcompositorState,
        queue_handle: &QueueHandle<State>,
    ) -> Self
    where
        State: Dispatch<WlSurface, SurfaceData> + Dispatch<WlSubsurface, SubsurfaceData> + 'static,
    {
        let parts = [
            // Header.
            FramePart::new(
                subcompositor.create_subsurface(parent.clone(), queue_handle),
                0,
                HEADER_SIZE,
                (0, -(HEADER_SIZE as i32)),
            ),
            // Top border.
            FramePart::new(
                subcompositor.create_subsurface(parent.clone(), queue_handle),
                0,
                BORDER_SIZE,
                (-(BORDER_SIZE as i32), -(HEADER_SIZE as i32 + BORDER_SIZE as i32)),
            ),
            // Right border.
            FramePart::new(
                subcompositor.create_subsurface(parent.clone(), queue_handle),
                BORDER_SIZE,
                0,
                (0, -(HEADER_SIZE as i32)),
            ),
            // Bottom border.
            FramePart::new(
                subcompositor.create_subsurface(parent.clone(), queue_handle),
                0,
                BORDER_SIZE,
                (-(BORDER_SIZE as i32), 0),
            ),
            // Left border.
            FramePart::new(
                subcompositor.create_subsurface(parent.clone(), queue_handle),
                BORDER_SIZE,
                0,
                (-(BORDER_SIZE as i32), -(HEADER_SIZE as i32)),
            ),
        ];

        Self { parts }
    }
}

#[derive(Debug)]
struct FramePart {
    /// The surface used for the frame part.
    subsurface: WlSubsurface,

    /// The surface used for this part.
    surface: WlSurface,

    /// The width of the Frame part in logical pixels.
    width: u32,

    /// The height of the Frame part in logical pixels.
    height: u32,

    /// The position for the subsurface.
    pos: (i32, i32),
}

impl FramePart {
    fn new(surfaces: (WlSubsurface, WlSurface), width: u32, height: u32, pos: (i32, i32)) -> Self {
        let (subsurface, surface) = surfaces;
        // XXX sync subsurfaces with the main surface.
        subsurface.set_sync();
        Self { surface, subsurface, width, height, pos }
    }
}

impl Drop for FramePart {
    fn drop(&mut self) {
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

/// The location inside the
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Location {
    /// The location doesn't belong to the frame.
    None,
    /// Header bar.
    Head,
    /// Top border.
    Top,
    /// Top right corner.
    TopRight,
    /// Right border.
    Right,
    /// Bottom right corner.
    BottomRight,
    /// Bottom border.
    Bottom,
    /// Bottom left corner.
    BottomLeft,
    /// Left border.
    Left,
    /// Top left corner.
    TopLeft,
    /// One of the buttons.
    Button(UIButton),
}

/// The frame button.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum UIButton {
    /// The minimize button, the left most.
    Minimize,
    /// The maximize button, in the middle.
    Maximize,
    /// The close botton, the right most.
    Close,
}
