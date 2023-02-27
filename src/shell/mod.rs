//! # Shell abstractions
//!
//! A shell describes a set of wayland protocol extensions which define the capabilities of a surface and how
//! the surface is displayed.
//!
//! ## Cross desktop group (XDG) shell
//!
//! The XDG shell describes the semantics of desktop application windows.
//!
//! The XDG shell defines two types of surfaces:
//! - [`Window`] - An application window[^window].
//! - [`Popup`] - A child surface positioned relative to a window.
//!
//! ### Why use the XDG shell
//!
//! The XDG shell is the primary protocol through which application windows are created. You can be near
//! certain every desktop compositor will implement this shell so that applications may create windows.
//!
//! See the [XDG shell module documentation] for more information about creating application windows.
//!
//! ## Layer shell
//!
//! The layer shell is a protocol which allows the creation of "layers". A layer refers to a surface rendered
//! at some specific z-depth relative to other layers. A layer may also be anchored to some edge and corner of
//! the screen.
//!
//! The layer shell defines one type of surface: the [`wlr_layer::LayerSurface`].
//!
//! There is no guarantee that the layer shell will be available in every compositor.
//!
//! ### Why use the layer shell
//!
//! The layer shell may be used to implement many desktop shell components, such as backgrounds, docks and
//! launchers.
//!
//! [^window]: The XDG shell protocol actually refers to a window as a toplevel surface, but we use the more
//! familiar term "window" for the sake of clarity.
//!
//! [XDG shell module documentation]: self::xdg
//! [`Window`]: self::xdg::window::Window
//! [`Popup`]: self::xdg::popup::Popup
//!
//! [`Layer`]: self::layer::LayerSurface

use wayland_client::{
    protocol::{wl_buffer, wl_output, wl_region, wl_surface},
    Proxy,
};

pub mod wlr_layer;
pub mod xdg;

/// An unsupported operation, often due to the version of the protocol.
#[derive(Debug, Default)]
pub struct Unsupported;

/// Functionality shared by all [`wl_surface::WlSurface`] backed shell role objects.
pub trait WaylandSurface: Sized {
    /// The underlying [`WlSurface`](wl_surface::WlSurface).
    fn wl_surface(&self) -> &wl_surface::WlSurface;

    fn attach(&self, buffer: Option<&wl_buffer::WlBuffer>, x: u32, y: u32) {
        // In version 5 and later, the x and y offset of `wl_surface::attach` must be zero and uses the
        // `offset` request instead.
        let (attach_x, attach_y) = if self.wl_surface().version() >= 5 { (0, 0) } else { (x, y) };

        self.wl_surface().attach(buffer, attach_x as i32, attach_y as i32);

        if self.wl_surface().version() >= 5 {
            // Ignore the error since the version is garunteed to be at least 5 here.
            let _ = self.offset(x, y);
        }
    }

    // TODO: Damage (Buffer and Surface-local)

    // TODO: Frame (a nice helper for this could exist).

    fn set_opaque_region(&self, region: Option<&wl_region::WlRegion>) {
        self.wl_surface().set_opaque_region(region);
    }

    fn set_input_region(&self, region: Option<&wl_region::WlRegion>) {
        self.wl_surface().set_input_region(region);
    }

    fn set_buffer_transform(&self, transform: wl_output::Transform) -> Result<(), Unsupported> {
        if self.wl_surface().version() < 2 {
            return Err(Unsupported);
        }

        self.wl_surface().set_buffer_transform(transform);
        Ok(())
    }

    fn set_buffer_scale(&self, scale: u32) -> Result<(), Unsupported> {
        if self.wl_surface().version() < 3 {
            return Err(Unsupported);
        }

        self.wl_surface().set_buffer_scale(scale as i32);
        Ok(())
    }

    fn offset(&self, x: u32, y: u32) -> Result<(), Unsupported> {
        if self.wl_surface().version() < 5 {
            return Err(Unsupported);
        }

        self.wl_surface().offset(x as i32, y as i32);
        Ok(())
    }

    /// Commits pending surface state.
    ///
    /// On commit, the pending double buffered state from the surface, including role dependent state is
    /// applied.
    ///
    /// # Initial commit
    ///
    /// In many protocol extensions, the concept of an initial commit is used. A initial commit provides the
    /// initial state of a surface to the compositor. For example with the [xdg shell](xdg),
    /// creating a window requires an initial commit.
    ///
    /// # Protocol Errors
    ///
    /// If the commit is the initial commit, no buffers must have been attached to the surface. This rule
    /// applies whether attaching the buffer was done using [`WaylandSurface::attach`] or under the hood in
    /// via window system integration in graphics APIs such as Vulkan (using `vkQueuePresentKHR`) and EGL
    /// (using `eglSwapBuffers`).
    fn commit(&self) {
        self.wl_surface().commit();
    }
}
