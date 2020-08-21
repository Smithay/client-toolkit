//! Window abstraction
use std::sync::{Arc, Mutex};

use wayland_client::protocol::{
    wl_compositor, wl_output, wl_seat, wl_shm, wl_subcompositor, wl_surface,
};
use wayland_client::{Attached, DispatchData};

use wayland_protocols::xdg_shell::client::xdg_toplevel::ResizeEdge;
pub use wayland_protocols::xdg_shell::client::xdg_toplevel::State;

use wayland_protocols::unstable::xdg_decoration::v1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};

use crate::{
    environment::{Environment, GlobalHandler, MultiGlobalHandler},
    shell,
};

#[cfg(feature = "frames")]
mod concept_frame;
#[cfg(feature = "frames")]
pub use self::concept_frame::{ConceptConfig, ConceptFrame};

// Defines the minimum window size. Minimum width is set to 2 pixels to circumvent
// a bug in mutter - https://gitlab.gnome.org/GNOME/mutter/issues/259
const MIN_WINDOW_SIZE: (u32, u32) = (2, 1);

/// Represents the status of a button
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ButtonState {
    /// Button is being hovered over by pointer
    Hovered,
    /// Button is not being hovered over by pointer
    Idle,
    /// Button is disabled
    Disabled,
}

/// Represents the status of a window
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WindowState {
    /// The window is active, in the foreground
    Active,
    /// The window is inactive, in the background
    Inactive,
}

impl From<bool> for WindowState {
    fn from(b: bool) -> WindowState {
        if b {
            WindowState::Active
        } else {
            WindowState::Inactive
        }
    }
}

impl From<WindowState> for bool {
    fn from(s: WindowState) -> bool {
        match s {
            WindowState::Active => true,
            WindowState::Inactive => false,
        }
    }
}

/// Possible events generated by a window that you need to handle
#[derive(Clone, Debug)]
pub enum Event {
    /// The state of your window has been changed
    Configure {
        /// Optional new size for your *inner* surface
        ///
        /// This is the new size of the contents of your window
        /// as suggested by the server. You can ignore it and choose
        /// a new size if you want better control on the possible
        /// sizes of your window.
        ///
        /// The size is expressed in logical pixels, you need to multiply it by
        /// your buffer scale to get the actual number of pixels to draw.
        ///
        /// In all cases, these events can be generated in large batches
        /// during an interactive resize, and you should buffer them before
        /// processing them. You only need to handle the last one of a batch.
        new_size: Option<(u32, u32)>,
        /// New combination of states of your window
        ///
        /// Typically tells you if your surface is active/inactive, maximized,
        /// etc...
        states: Vec<State>,
    },
    /// A close request has been received
    ///
    /// Most likely the user has clicked on the close button of the decorations
    /// or something equivalent
    Close,
    /// The decorations need to be refreshed
    Refresh,
}

/// Possible decoration modes for a Window
///
/// This represents what your application requests from the server.
///
/// In any case, the compositor may override your requests. In that case SCTK
/// will follow its decision.
///
/// If you don't care about it, you should use `FollowServer` (which is the
/// SCTK default). It'd be the most ergonomic for your users.
pub enum Decorations {
    /// Request server-side decorations
    ServerSide,
    /// Force using the client-side `Frame`
    ClientSide,
    /// Follow the preference of the compositor
    FollowServer,
    /// Don't decorate the Window
    None,
}

struct WindowInner<F> {
    frame: Arc<Mutex<F>>,
    shell_surface: Arc<Box<dyn shell::ShellSurface>>,
    user_impl: Box<dyn FnMut(Event, DispatchData)>,
    min_size: (u32, u32),
    max_size: Option<(u32, u32)>,
    current_size: (u32, u32),
    old_size: Option<(u32, u32)>,
    decorated: bool,
}

/// A window
///
/// This wrapper handles for you the decoration of your window
/// and the interaction with the server regarding the shell protocol.
///
/// You are still entirely responsible for drawing the contents of your
/// window.
///
/// Note also that as the dimensions of wayland surfaces is defined by
/// their attached buffer, you need to keep the decorations in sync with
/// your contents via the `resize(..)` method.
///
/// Different kind of decorations can be used by customizing the type
/// parameter. A few are provided in this crate if the `frames` cargo feature
/// is enabled, but any type implementing the `Frame` trait can do.
pub struct Window<F: Frame> {
    frame: Arc<Mutex<F>>,
    surface: wl_surface::WlSurface,
    decoration: Mutex<Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>>,
    decoration_mgr: Option<Attached<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>>,
    shell_surface: Arc<Box<dyn shell::ShellSurface>>,
    inner: Arc<Mutex<Option<WindowInner<F>>>>,
    _seat_listener: crate::seat::SeatListener,
}

impl<F: Frame + 'static> Window<F> {
    /// Create a new window wrapping a given wayland surface as its main content and
    /// following the compositor's preference regarding server-side decorations
    ///
    /// It can fail if the initialization of the frame fails (for example if the
    /// frame class fails to initialize its SHM).
    fn init_with_decorations<Impl, E>(
        env: &crate::environment::Environment<E>,
        surface: wl_surface::WlSurface,
        initial_dims: (u32, u32),
        implementation: Impl,
    ) -> Result<Window<F>, F::Error>
    where
        Impl: FnMut(Event, DispatchData) + 'static,
        E: GlobalHandler<wl_compositor::WlCompositor>
            + GlobalHandler<wl_subcompositor::WlSubcompositor>
            + GlobalHandler<wl_shm::WlShm>
            + crate::shell::ShellHandling
            + MultiGlobalHandler<wl_seat::WlSeat>
            + GlobalHandler<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>
            + crate::seat::SeatHandling,
    {
        let compositor = env.require_global::<wl_compositor::WlCompositor>();
        let subcompositor = env.require_global::<wl_subcompositor::WlSubcompositor>();
        let shm = env.require_global::<wl_shm::WlShm>();
        let shell = env
            .get_shell()
            .expect("[SCTK] Cannot create a window if the compositor advertized no shell.");
        let decoration_mgr =
            env.get_global::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>();

        let inner = Arc::new(Mutex::new(None::<WindowInner<F>>));
        let frame_inner = inner.clone();
        let shell_inner = inner.clone();
        let mut frame = F::init(
            &surface,
            &compositor,
            &subcompositor,
            &shm,
            Box::new(move |req, serial, ddata: DispatchData| {
                if let Some(ref mut inner) = *shell_inner.lock().unwrap() {
                    match req {
                        FrameRequest::Minimize => inner.shell_surface.set_minimized(),
                        FrameRequest::Maximize => inner.shell_surface.set_maximized(),
                        FrameRequest::UnMaximize => inner.shell_surface.unset_maximized(),
                        FrameRequest::Move(seat) => inner.shell_surface.move_(&seat, serial),
                        FrameRequest::Resize(seat, edges) => {
                            inner.shell_surface.resize(&seat, serial, edges)
                        }
                        FrameRequest::Close => (inner.user_impl)(Event::Close, ddata),
                        FrameRequest::Refresh => (inner.user_impl)(Event::Refresh, ddata),
                    }
                }
            }) as Box<_>,
        )?;
        frame.resize(initial_dims);
        let frame = Arc::new(Mutex::new(frame));
        let shell_surface = Arc::new(shell::create_shell_surface(
            &shell,
            &surface,
            move |event, mut ddata: DispatchData| {
                let mut frame_inner = frame_inner.lock().unwrap();
                let mut inner = match frame_inner.as_mut() {
                    Some(inner) => inner,
                    None => return,
                };

                match event {
                    shell::Event::Configure { states, mut new_size } => {
                        let mut frame = inner.frame.lock().unwrap();

                        // Populate frame changes. We should do it before performing new_size
                        // recalculation, since we should account for a fullscreen state.
                        let need_refresh = frame.set_states(&states);

                        // Clamp size.
                        new_size = new_size.map(|(w, h)| {
                            use std::cmp::{max, min};
                            let (mut w, mut h) = frame.subtract_borders(w as i32, h as i32);
                            let (minw, minh) = inner.min_size;
                            w = max(w, minw as i32);
                            h = max(h, minh as i32);
                            if let Some((maxw, maxh)) = inner.max_size {
                                w = min(w, maxw as i32);
                                h = min(h, maxh as i32);
                            }
                            (max(w, 1) as u32, max(h, 1) as u32)
                        });

                        // Check whether we should save old size for later restoration.
                        let should_stash_size = states
                            .iter()
                            .find(|s| *s == &State::Maximized || *s == &State::Fullscreen)
                            .map(|_| true)
                            .unwrap_or(false);

                        if should_stash_size {
                            if inner.old_size.is_none() {
                                // We are getting maximized/fullscreened, store the size for
                                // restoration.
                                inner.old_size = Some(inner.current_size);
                            }
                        } else if new_size.is_none() {
                            // We are getting de-maximized/de-fullscreened, restore the size
                            // if we were not previously maximized/fullscreened, old_size is
                            // None and this does nothing.
                            new_size = inner.old_size.take();
                        } else {
                            // We are neither maximized nor fullscreened, but are given a size,
                            // respect it and forget about the old size.
                            inner.old_size = None;
                        }

                        if need_refresh {
                            (inner.user_impl)(Event::Refresh, ddata.reborrow());
                        }
                        (inner.user_impl)(Event::Configure { states, new_size }, ddata);
                    }
                    shell::Event::Close => {
                        (inner.user_impl)(Event::Close, ddata);
                    }
                }
            },
        ));

        // setup size and geometry
        {
            let frame = frame.lock().unwrap();
            let (minw, minh) =
                frame.add_borders(MIN_WINDOW_SIZE.0 as i32, MIN_WINDOW_SIZE.1 as i32);
            shell_surface.set_min_size(Some((minw, minh)));
            let (w, h) = frame.add_borders(initial_dims.0 as i32, initial_dims.1 as i32);
            let (x, y) = frame.location();
            shell_surface.set_geometry(x, y, w, h);
        }

        // initial seat setup
        let mut seats = Vec::<wl_seat::WlSeat>::new();
        for seat in env.get_all_seats() {
            crate::seat::with_seat_data(&seat, |seat_data| {
                if seat_data.has_pointer && !seat_data.defunct {
                    seats.push(seat.detach());
                    frame.lock().unwrap().new_seat(&seat);
                }
            });
        }

        // setup seat_listener
        let seat_frame = frame.clone();
        let seat_listener = env.listen_for_seats(move |seat, seat_data, _| {
            let is_known = seats.contains(&seat);
            if !is_known && seat_data.has_pointer && !seat_data.defunct {
                seat_frame.lock().unwrap().new_seat(&seat);
                seats.push(seat.detach());
            } else if is_known && ((!seat_data.has_pointer) || seat_data.defunct) {
                seat_frame.lock().unwrap().remove_seat(&seat);
                seats.retain(|s| s != &*seat);
            }
        });

        *(inner.lock().unwrap()) = Some(WindowInner {
            frame: frame.clone(),
            shell_surface: shell_surface.clone(),
            user_impl: Box::new(implementation) as Box<_>,
            min_size: (MIN_WINDOW_SIZE.0, MIN_WINDOW_SIZE.1),
            max_size: None,
            current_size: initial_dims,
            old_size: None,
            decorated: true,
        });

        let window = Window {
            frame,
            shell_surface,
            decoration: Mutex::new(None),
            decoration_mgr,
            surface,
            inner,
            _seat_listener: seat_listener,
        };

        // init decoration if applicable
        {
            let mut decoration = window.decoration.lock().unwrap();
            window.ensure_decoration(&mut decoration);
        }

        Ok(window)
    }

    fn ensure_decoration(
        &self,
        decoration: &mut Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    ) {
        if self.decoration_mgr.is_none() {
            return;
        }

        if let Some(ref decoration) = *decoration {
            if decoration.as_ref().is_alive() {
                return;
            }
        }

        let decoration_frame = self.frame.clone();
        let decoration_inner = self.inner.clone();
        *decoration = match (self.shell_surface.get_xdg(), &self.decoration_mgr) {
            (Some(toplevel), &Some(ref mgr)) => {
                use self::zxdg_toplevel_decoration_v1::{Event, Mode};
                let decoration = mgr.get_toplevel_decoration(toplevel);
                decoration.quick_assign(move |_, event, _| {
                    if let Event::Configure { mode } = event {
                        match mode {
                            Mode::ServerSide => {
                                decoration_frame.lock().unwrap().set_hidden(true);
                            }
                            Mode::ClientSide => {
                                let want_decorate = decoration_inner
                                    .lock()
                                    .unwrap()
                                    .as_ref()
                                    .map(|inner| inner.decorated)
                                    .unwrap_or(false);
                                decoration_frame.lock().unwrap().set_hidden(!want_decorate);
                            }
                            _ => unreachable!(),
                        }
                    }
                });
                Some(decoration.detach())
            }
            _ => None,
        };
    }

    /// Access the surface wrapped in this Window
    pub fn surface(&self) -> &wl_surface::WlSurface {
        &self.surface
    }

    /// Refreshes the frame
    ///
    /// Redraws the frame to match its requested state (dimensions, presence/
    /// absence of decorations, ...)
    ///
    /// You need to call this method after every change to the dimensions or state
    /// of the decorations of your window, otherwise the drawn decorations may go
    /// out of sync with the state of your content.
    ///
    /// Your implementation will also receive `Refresh` events when the frame requests
    /// to be redrawn (to provide some frame animations for example).
    pub fn refresh(&mut self) {
        self.frame.lock().unwrap().redraw();
    }

    /// Set a short title for the window.
    ///
    /// This string may be used to identify the surface in a task bar, window list, or other
    /// user interface elements provided by the compositor.
    ///
    /// You need to call `refresh()` afterwards for this to properly
    /// take effect.
    pub fn set_title(&self, mut title: String) {
        // Truncate the title to at most 1024 bytes, so that it does not blow up the protocol
        // messages
        if title.len() > 1024 {
            let mut new_len = 1024;
            while !title.is_char_boundary(new_len) {
                new_len -= 1;
            }
            title.truncate(new_len);
        }
        self.frame.lock().unwrap().set_title(title.clone());
        self.shell_surface.set_title(title);
    }

    /// Set an app id for the surface.
    ///
    /// The surface class identifies the general class of applications to which the surface
    /// belongs.
    ///
    /// Several wayland compositors will try to find a `.desktop` file matching this name
    /// to find metadata about your apps.
    pub fn set_app_id(&self, app_id: String) {
        self.shell_surface.set_app_id(app_id);
    }

    /// Set whether the window should be decorated or not
    ///
    /// You need to call `refresh()` afterwards for this to properly
    /// take effect.
    pub fn set_decorate(&self, decorate: Decorations) {
        use self::zxdg_toplevel_decoration_v1::Mode;
        let mut decoration_guard = self.decoration.lock().unwrap();

        if let Decorations::ClientSide = decorate {
            self.frame.lock().unwrap().set_hidden(false);
        } else {
            self.frame.lock().unwrap().set_hidden(true);
        }

        if let Some(ref mut inner) = *self.inner.lock().unwrap() {
            if let Decorations::None = decorate {
                inner.decorated = false;
            } else {
                inner.decorated = true;
            }
        }

        // destroy the decoration object, so that the server does not
        // decorate us if we don't want to
        if let Decorations::None | Decorations::ClientSide = decorate {
            if let Some(ref dec) = decoration_guard.take() {
                dec.destroy();
            }
            return;
        }

        // We didn't early exit, so we need to deal with server-side decorations
        self.ensure_decoration(&mut decoration_guard);
        if let Some(ref dec) = *decoration_guard {
            if let Decorations::ServerSide = decorate {
                dec.set_mode(Mode::ServerSide);
            } else {
                dec.unset_mode();
            }
        }
    }

    /// Set whether the window should be resizeable by the user
    ///
    /// This is not an hard blocking, as the compositor can always
    /// resize you forcibly if it wants. However it signals it that
    /// you don't want this window to be resized.
    ///
    /// Additionally, the decorations will stop suggesting the user
    /// to resize by dragging the borders if you set the window as
    /// non-resizable.
    ///
    /// When re-activating resizability, any previously set min/max
    /// sizes are restored.
    pub fn set_resizable(&self, resizable: bool) {
        let mut frame = self.frame.lock().unwrap();
        frame.set_resizable(resizable);
        let mut inner = self.inner.lock().unwrap();
        if let Some(ref mut inner) = *inner {
            if resizable {
                // restore the min/max sizes
                self.shell_surface.set_min_size(
                    Some(inner.min_size).map(|(w, h)| frame.add_borders(w as i32, h as i32)),
                );
                self.shell_surface.set_max_size(
                    inner.max_size.map(|(w, h)| frame.add_borders(w as i32, h as i32)),
                );
            } else {
                // lock the min/max sizes to current size
                let (w, h) = inner.current_size;
                self.shell_surface.set_min_size(Some(frame.add_borders(w as i32, h as i32)));
                self.shell_surface.set_max_size(Some(frame.add_borders(w as i32, h as i32)));
            }
        }
    }

    /// Resize the decorations
    ///
    /// You should call this whenever you change the size of the contents
    /// of your window, with the new _inner size_ of your window.
    ///
    /// This size is expressed in logical pixels, like the one received
    /// in [`Event::Configure`](enum.Event.html).
    ///
    /// You need to call `refresh()` afterwards for this to properly
    /// take effect.
    pub fn resize(&mut self, w: u32, h: u32) {
        use std::cmp::max;
        let w = max(w, 1);
        let h = max(h, 1);
        if let Some(ref mut inner) = *self.inner.lock().unwrap() {
            inner.current_size = (w, h);
        }
        let mut frame = self.frame.lock().unwrap();
        frame.resize((w, h));
        let (w, h) = frame.add_borders(w as i32, h as i32);
        let (x, y) = frame.location();
        self.shell_surface.set_geometry(x, y, w, h);
    }

    /// Request the window to be maximized
    pub fn set_maximized(&self) {
        self.shell_surface.set_maximized();
    }

    /// Request the window to be un-maximized
    pub fn unset_maximized(&self) {
        self.shell_surface.unset_maximized();
    }

    /// Request the window to be minimized
    pub fn set_minimized(&self) {
        self.shell_surface.set_minimized();
    }

    /// Request the window to be set fullscreen
    ///
    /// Note: you need to manually disable the decorations if you
    /// want to hide them!
    pub fn set_fullscreen(&self, output: Option<&wl_output::WlOutput>) {
        self.shell_surface.set_fullscreen(output);
    }

    /// Request the window to quit fullscreen mode
    pub fn unset_fullscreen(&self) {
        self.shell_surface.unset_fullscreen();
    }

    /// Sets the minimum possible size for this window
    ///
    /// Provide either a tuple `Some((width, height))` or `None` to unset the
    /// minimum size.
    ///
    /// Setting either value in the tuple to `0` means that this axis should not
    /// be limited.
    ///
    /// The provided size is the interior size, not counting decorations.
    ///
    /// This size is expressed in logical pixels, like the one received
    /// in [`Event::Configure`](enum.Event.html).
    pub fn set_min_size(&mut self, size: Option<(u32, u32)>) {
        let (w, h) = size.unwrap_or(MIN_WINDOW_SIZE);
        let (w, h) = self.frame.lock().unwrap().add_borders(w as i32, h as i32);
        self.shell_surface.set_min_size(Some((w, h)));
        if let Some(ref mut inner) = *(self.inner.lock().unwrap()) {
            inner.min_size = size.unwrap_or(MIN_WINDOW_SIZE)
        }
    }

    /// Sets the maximum possible size for this window
    ///
    /// Provide either a tuple `Some((width, height))` or `None` to unset the
    /// maximum size.
    ///
    /// Setting either value in the tuple to `0` means that this axis should not
    /// be limited.
    ///
    /// The provided size is the interior size, not counting decorations.
    ///
    /// This size is expressed in logical pixels, like the one received
    /// in [`Event::Configure`](enum.Event.html).
    pub fn set_max_size(&mut self, size: Option<(u32, u32)>) {
        let max_size =
            size.map(|(w, h)| self.frame.lock().unwrap().add_borders(w as i32, h as i32));
        self.shell_surface.set_max_size(max_size);
        if let Some(ref mut inner) = *(self.inner.lock().unwrap()) {
            inner.max_size = size.map(|(w, h)| (w as u32, h as u32));
        }
    }

    /// Sets the frame configuration for the window
    ///
    /// This allows to configure the frame at runtime if it supports
    /// it. See the documentation of your `Frame` implementation for
    /// details about what configuration it supports.
    pub fn set_frame_config(&mut self, config: F::Config) {
        self.frame.lock().unwrap().set_config(config)
    }
}

impl<F: Frame> Drop for Window<F> {
    fn drop(&mut self) {
        self.inner.lock().unwrap().take();
    }
}

/// Request generated by a Frame
///
/// These requests are generated by a Frame and the Window will
/// forward them appropriately to the server.
pub enum FrameRequest {
    /// The window should be minimized
    Minimize,
    /// The window should be maximized
    Maximize,
    /// The window should be unmaximized
    UnMaximize,
    /// The window should be closed
    Close,
    /// An interactive move should be started
    Move(wl_seat::WlSeat),
    /// An interactive resize should be started
    Resize(wl_seat::WlSeat, ResizeEdge),
    /// The frame requests to be refreshed
    Refresh,
}

/// Interface for defining the drawing of decorations
///
/// A type implementing this trait can be used to define custom
/// decorations additionnaly to the ones provided by this crate
/// and be used with `Window`.
pub trait Frame: Sized {
    /// Type of errors that may occur when attempting to create a frame
    type Error;
    /// Configuration for this frame
    type Config;
    /// Initialize the Frame
    fn init(
        base_surface: &wl_surface::WlSurface,
        compositor: &Attached<wl_compositor::WlCompositor>,
        subcompositor: &Attached<wl_subcompositor::WlSubcompositor>,
        shm: &Attached<wl_shm::WlShm>,
        callback: Box<dyn FnMut(FrameRequest, u32, DispatchData)>,
    ) -> Result<Self, Self::Error>;
    /// Set the Window XDG states for the frame
    ///
    /// This notably includes information about whether the window is
    /// maximized, active, or tiled, and can affect the way decorations
    /// are drawn.
    ///
    /// Calling this should *not* trigger a redraw, but return `true` if
    /// a redraw is needed.
    fn set_states(&mut self, states: &[State]) -> bool;
    /// Hide or show the decorations
    ///
    /// Calling this should *not* trigger a redraw
    fn set_hidden(&mut self, hidden: bool);
    /// Set whether interactive resize hints should be displayed
    /// and reacted to
    fn set_resizable(&mut self, resizable: bool);
    /// Notify that a new wl_seat should be handled
    ///
    /// This seat is guaranteed to have pointer capability
    fn new_seat(&mut self, seat: &Attached<wl_seat::WlSeat>);
    /// Notify that this seat has lost the pointer capability or
    /// has been lost
    fn remove_seat(&mut self, seat: &wl_seat::WlSeat);
    /// Change the size of the decorations
    ///
    /// Calling this should *not* trigger a redraw
    fn resize(&mut self, newsize: (u32, u32));
    /// Redraw the decorations
    fn redraw(&mut self);
    /// Subtracts the border dimensions from the given dimensions.
    fn subtract_borders(&self, width: i32, height: i32) -> (i32, i32);
    /// Adds the border dimensions to the given dimensions.
    fn add_borders(&self, width: i32, height: i32) -> (i32, i32);
    /// Returns the coordinates of the top-left corner of the borders relative to the content
    ///
    /// Values should thus be negative
    fn location(&self) -> (i32, i32) {
        (0, 0)
    }
    /// Sets the configuration for the frame
    fn set_config(&mut self, config: Self::Config);

    /// Sets the frames title
    fn set_title(&mut self, title: String);
}

impl<E> Environment<E>
where
    E: GlobalHandler<wl_compositor::WlCompositor>
        + GlobalHandler<wl_subcompositor::WlSubcompositor>
        + GlobalHandler<wl_shm::WlShm>
        + crate::shell::ShellHandling
        + MultiGlobalHandler<wl_seat::WlSeat>
        + GlobalHandler<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>
        + crate::seat::SeatHandling,
{
    /// Create a new window wrapping given surface
    ///
    /// This window handles decorations for you, this includes
    /// drawing them if the compositor doe snot support them, resizing interactions
    /// and moving the window. It also provides close/maximize/minimize buttons.
    ///
    /// Many interactions still require your input, and are given to you via the
    /// callback you need to provide.
    pub fn create_window<F: Frame + 'static, CB>(
        &self,
        surface: wl_surface::WlSurface,
        initial_dims: (u32, u32),
        callback: CB,
    ) -> Result<Window<F>, F::Error>
    where
        CB: FnMut(Event, DispatchData) + 'static,
    {
        Window::<F>::init_with_decorations(self, surface, initial_dims, callback)
    }
}

//
// Some helpers for Frame configuration
//

/// Color specification to be used in Frame configuration
///
/// It regroups two colors, one for when the window is active and
/// one for when it is not.
#[derive(Copy, Clone, Debug)]
pub struct ColorSpec {
    /// The active color
    pub active: ARGBColor,
    /// The inactive color
    pub inactive: ARGBColor,
}

impl ColorSpec {
    /// Access the color associated with a certain window state
    #[inline]
    pub fn get_for(self, state: WindowState) -> ARGBColor {
        match state {
            WindowState::Active => self.active,
            WindowState::Inactive => self.inactive,
        }
    }

    /// Create a ColorSpec that is always the same color
    #[inline]
    pub const fn identical(color: ARGBColor) -> ColorSpec {
        ColorSpec { active: color, inactive: color }
    }

    /// Create a ColorSpec corresponding to an always invisible color
    #[inline]
    pub const fn invisible() -> ColorSpec {
        ColorSpec::identical(ARGBColor::zero())
    }
}

/// A color specification associated with a button
///
/// It regroups 3 color specifications depending on the state of the
/// button: idle, hovered, or disabled.
#[derive(Copy, Clone, Debug)]
pub struct ButtonColorSpec {
    /// ColorSpec for an idle button
    pub idle: ColorSpec,
    /// ColorSpec for an hovered button
    pub hovered: ColorSpec,
    /// ColorSpec for a disabled button
    pub disabled: ColorSpec,
}

impl ButtonColorSpec {
    /// Get the ColorSpec associated with a given button state
    pub fn get_for(&self, state: ButtonState) -> ColorSpec {
        match state {
            ButtonState::Idle => self.idle,
            ButtonState::Hovered => self.hovered,
            ButtonState::Disabled => self.disabled,
        }
    }
}

/// Unambiguous representation of an ARGB color
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ARGBColor {
    /// Alpha channel
    pub a: u8,
    /// Red channel
    pub r: u8,
    /// Green channel
    pub g: u8,
    /// Blue channel
    pub b: u8,
}

impl ARGBColor {
    /// The invisible `#00000000` color
    pub const fn zero() -> ARGBColor {
        ARGBColor { a: 0, r: 0, g: 0, b: 0 }
    }
}

impl From<ARGBColor> for [u8; 4] {
    fn from(color: ARGBColor) -> [u8; 4] {
        [color.a, color.r, color.g, color.b]
    }
}

impl From<[u8; 4]> for ARGBColor {
    fn from(array: [u8; 4]) -> ARGBColor {
        ARGBColor { a: array[0], r: array[1], g: array[2], b: array[3] }
    }
}
