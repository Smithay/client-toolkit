//! The frame to use with XDG shell window.

use std::num::NonZeroU32;

use crate::reexports::client::protocol::wl_surface::WlSurface;
use crate::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;

use crate::shell::xdg::window::{WindowManagerCapabilities, WindowState};

pub mod fallback_frame;

/// The interface for the client side decorations.
pub trait DecorationsFrame: Sized {
    /// Emulate click on the decorations.
    ///
    /// The `click` is a variant of click to use, see [`FrameClick`] for more information.
    ///
    /// The return value is a [`FrameAction`] you should apply, this action could be
    /// ignored.
    ///
    /// The location of the click is the one passed to [`Self::click_point_moved`].
    fn on_click(&mut self, click: FrameClick, pressed: bool) -> Option<FrameAction>;

    /// Emulate pointer moved event on the decorations frame.
    ///
    /// The `x` and `y` are location in the surface local coordinates relative to the `surface`.
    ///
    /// The return value is the new cursor icon you should apply to provide better visual
    /// feedback for the user. However, you might want to ignore it, if you're using touch events
    /// to drive the movements.
    fn click_point_moved(&mut self, surface: &WlSurface, x: f64, y: f64) -> Option<&str>;

    /// All clicks left the decorations.
    ///
    /// This function should be called when input leaves the decorations.
    fn click_point_left(&mut self);

    /// Update the state of the frame.
    ///
    /// The state is usually obtained from the [`WindowConfigure`] event.
    ///
    /// [`WindowConfigure`]: crate::shell::window::WindowConfigure
    fn update_state(&mut self, state: WindowState);

    /// Update the window manager capabilites.
    ///
    /// The capabilites are usually obtained from the [`WindowConfigure`] event.
    ///
    /// [`WindowConfigure`]: crate::shell::window::WindowConfigure
    fn update_wm_capabilities(&mut self, wm_capabilities: WindowManagerCapabilities);

    /// Resize the window to the new size.
    ///
    /// The size must be without the borders, as in [`Self::subtract_borders]` were used on it.
    ///
    /// **Note:** The [`Self::update_state`] and [`Self::update_wm_capabilities`] **must be**
    /// applied before calling this function.
    ///
    /// # Panics
    ///
    /// Panics when resizing the hidden frame.
    fn resize(&mut self, width: NonZeroU32, height: NonZeroU32);

    /// Return the coordinates of the top-left corner of the borders relative to the content.
    ///
    /// Values **must** thus be negative.
    fn location(&self) -> (i32, i32);

    /// Subtract the borders from the given `width` and `height`.
    ///
    /// `None` will be returned for the particular dimension when the given
    /// value for it was too small.
    fn subtract_borders(
        &self,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> (Option<NonZeroU32>, Option<NonZeroU32>);

    /// Add the borders to the given `width` and `height`.
    ///
    /// Passing zero for both width and height could be used to get the size
    /// of the decorations frame.
    fn add_borders(&self, width: u32, height: u32) -> (u32, u32);

    /// Whether the given frame is dirty and should be redrawn.
    fn is_dirty(&self) -> bool;

    /// Set the frame as hidden.
    ///
    /// The frame **must be** visible by default.
    fn set_hidden(&mut self, hidden: bool);

    /// Get the frame hidden state.
    ///
    /// Get the state of the last [`DecorationsFrame::set_hidden`].
    fn is_hidden(&self) -> bool;

    /// Mark the frame as resizable.
    ///
    /// By default the frame is resizable.
    fn set_resizable(&mut self, resizable: bool);

    /// Draw the decorations frame.
    ///
    /// The user of the frame **must** commit the base surface afterwards.
    fn draw(&mut self);

    /// Set the frames title.
    fn set_title(&mut self, title: impl Into<String>);
}

/// The Frame action user should perform in responce to mouse click events.
#[derive(Debug, Clone, Copy)]
pub enum FrameAction {
    /// The window should be minimized.
    Minimize,
    /// The window should be maximized.
    Maximize,
    /// The window should be unmaximized.
    UnMaximize,
    /// The window should be closed.
    Close,
    /// An interactive move should be started.
    Move,
    /// An interactive resize should be started with the provided edge.
    Resize(ResizeEdge),
    /// Show window menu.
    ///
    /// The coordinates are relative to the base surface, as in should be directly passed
    /// to the `xdg_toplevel::show_window_menu`.
    ShowMenu(i32, i32),
}

/// The user clicked or touched the decoractions frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameClick {
    /// The user done normal click, likely with left mouse button or single finger touch.
    Normal,

    /// The user done right mouse click or some touch sequence that was treated as alternate click.
    ///
    /// The alternate click exists solely to provide alternative action, like show window
    /// menu when doing right mouse button cilck on the header decorations, nothing more.
    Alternate,
}
