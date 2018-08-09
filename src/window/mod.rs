use std::sync::{Arc, Mutex};

use wayland_client::commons::Implementation;
use wayland_client::protocol::{
    wl_compositor, wl_output, wl_seat, wl_shm, wl_subcompositor, wl_surface,
};
use wayland_client::Proxy;

use wayland_protocols::xdg_shell::client::xdg_toplevel::ResizeEdge;
pub use wayland_protocols::xdg_shell::client::xdg_toplevel::State;

use self::zxdg_decoration_manager_v1::RequestsTrait as DecorationMgrRequests;
use self::zxdg_toplevel_decoration_v1::RequestsTrait as DecorationRequests;
use wayland_protocols::unstable::xdg_decoration::v1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};

use Shell;

mod basic_frame;
mod shell;

pub use self::basic_frame::BasicFrame;

const MIN_WINDOW_SIZE: (u32, u32) = (2, 1);

/// Possible events generated by a window that you need to handle
#[derive(Clone, Debug)]
pub enum Event {
    /// The state of your window has been changed
    Configure {
        /// Optionnal new size for your *inner* surface
        ///
        /// This is the new size of the contents of your window
        /// as suggested by the server. You can ignore it and choose
        /// a new size if you want better control on the possible
        /// sizes of your window.
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

struct WindowInner<F> {
    frame: Arc<Mutex<F>>,
    shell_surface: Arc<Box<shell::ShellSurface>>,
    user_impl: Box<Implementation<(), Event> + Send>,
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
/// parameter. A few are provided in this crate, but any type implementing
/// the `Frame` trait can do.
pub struct Window<F: Frame> {
    frame: Arc<Mutex<F>>,
    surface: Proxy<wl_surface::WlSurface>,
    decoration: Mutex<Option<Proxy<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>>>,
    decoration_mgr: Option<Proxy<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>>,
    shell_surface: Arc<Box<shell::ShellSurface>>,
    inner: Arc<Mutex<Option<WindowInner<F>>>>,
}

impl<F: Frame + 'static> Window<F> {
    /// Create a new window wrapping a given wayland surface as its main content
    ///
    /// It can fail if the initialization of the frame fails (for example if the
    /// frame class fails to initialize its SHM).
    pub fn init<Impl>(
        surface: Proxy<wl_surface::WlSurface>,
        initial_dims: (u32, u32),
        compositor: &Proxy<wl_compositor::WlCompositor>,
        subcompositor: &Proxy<wl_subcompositor::WlSubcompositor>,
        shm: &Proxy<wl_shm::WlShm>,
        shell: &Shell,
        implementation: Impl,
    ) -> Result<Window<F>, F::Error>
    where
        Impl: Implementation<(), Event> + Send,
    {
        Self::init_with_decorations(
            surface,
            initial_dims,
            compositor,
            subcompositor,
            shm,
            shell,
            None,
            implementation,
        )
    }

    /// Create a new window wrapping a given wayland surface as its main content and
    /// following the compositor's preference regarding server-side decorations
    ///
    /// It can fail if the initialization of the frame fails (for example if the
    /// frame class fails to initialize its SHM).
    pub fn init_with_decorations<Impl>(
        surface: Proxy<wl_surface::WlSurface>,
        initial_dims: (u32, u32),
        compositor: &Proxy<wl_compositor::WlCompositor>,
        subcompositor: &Proxy<wl_subcompositor::WlSubcompositor>,
        shm: &Proxy<wl_shm::WlShm>,
        shell: &Shell,
        decoration_mgr: Option<&Proxy<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>>,
        implementation: Impl,
    ) -> Result<Window<F>, F::Error>
    where
        Impl: Implementation<(), Event> + Send,
    {
        let inner = Arc::new(Mutex::new(None::<WindowInner<F>>));
        let frame_inner = inner.clone();
        let shell_inner = inner.clone();
        let mut frame = F::init(
            &surface,
            compositor,
            subcompositor,
            shm,
            Box::new(move |req, serial| {
                if let Some(ref mut inner) = *shell_inner.lock().unwrap() {
                    match req {
                        FrameRequest::Minimize => inner.shell_surface.set_minimized(),
                        FrameRequest::Maximize => inner.shell_surface.set_maximized(),
                        FrameRequest::UnMaximize => inner.shell_surface.unset_maximized(),
                        FrameRequest::Move(seat) => inner.shell_surface.move_(&seat, serial),
                        FrameRequest::Resize(seat, edges) => {
                            inner.shell_surface.resize(&seat, serial, edges)
                        }
                        FrameRequest::Close => inner.user_impl.receive(Event::Close, ()),
                        FrameRequest::Refresh => inner.user_impl.receive(Event::Refresh, ()),
                    }
                }
            }) as Box<_>,
        )?;
        frame.resize(initial_dims);
        let frame = Arc::new(Mutex::new(frame));
        let shell_surface = Arc::new(shell::create_shell_surface(
            shell,
            &surface,
            move |evt, ()| {
                if let Some(ref mut inner) = *frame_inner.lock().unwrap() {
                    if let Event::Configure {
                        states,
                        mut new_size,
                    } = evt
                    {
                        let mut frame = inner.frame.lock().unwrap();
                        // clamp size
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
                        // compute frame changes
                        let mut need_refresh = false;
                        need_refresh |= frame.set_maximized(states.contains(&State::Maximized));
                        if need_refresh {
                            // the maximization state changed
                            if states.contains(&State::Maximized) {
                                // we are getting maximized, store the size for restoration
                                inner.old_size = Some(inner.current_size);
                            } else {
                                // we are getting de-maximized, restore the size
                                if new_size.is_none() {
                                    new_size = inner.old_size.take();
                                }
                            }
                        }
                        need_refresh |= frame.set_active(states.contains(&State::Activated));
                        if need_refresh {
                            inner.user_impl.receive(Event::Refresh, ());
                        }
                        inner
                            .user_impl
                            .receive(Event::Configure { states, new_size }, ());
                    } else {
                        inner.user_impl.receive(evt, ());
                    }
                }
            },
        ));

        // setup size and geometry
        {
            let frame = frame.lock().unwrap();
            let (minw, minh) = frame.add_borders(MIN_WINDOW_SIZE.0 as i32, MIN_WINDOW_SIZE.1 as i32);
            shell_surface.set_min_size(Some((minw, minh)));
            let (w, h) = frame.add_borders(initial_dims.0 as i32, initial_dims.1 as i32);
            let (x, y) = frame.location();
            shell_surface.set_geometry(x, y, w, h);
        }

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
            decoration_mgr: decoration_mgr.cloned(),
            surface,
            inner,
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
        decoration: &mut Option<Proxy<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>>,
    ) {
        if self.decoration_mgr.is_none() {
            return;
        }

        if let Some(ref decoration) = *decoration {
            if decoration.is_alive() {
                return;
            }
        }

        let decoration_frame = self.frame.clone();
        let decoration_inner = self.inner.clone();
        *decoration = match (self.shell_surface.get_xdg(), &self.decoration_mgr) {
            (Some(toplevel), &Some(ref mgr)) => mgr.get_toplevel_decoration(toplevel).ok().map(
                move |newdec| {
                    newdec.implement(move |event, _| {
                        use self::zxdg_toplevel_decoration_v1::{Event, Mode};
                        let Event::Configure { mode } = event;
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
                        }
                    })
                },
            ),
            _ => None,
        };
    }

    /// Notify this window that a new seat is accessible
    ///
    /// This allows the decoration manager to get an handle to the pointer
    /// to manage pointer events and change the pointer image appropriately.
    pub fn new_seat(&mut self, seat: &Proxy<wl_seat::WlSeat>) {
        self.frame.lock().unwrap().new_seat(seat);
    }

    /// Access the surface wrapped in this Window
    pub fn surface(&self) -> &Proxy<wl_surface::WlSurface> {
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
    /// This string may be used to identify the surface in a task bar, window list, or othe
    /// user interface elements provided by the compositor.
    pub fn set_title(&self, title: String) {
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
    pub fn set_decorate(&self, decorate: bool) {
        self.frame.lock().unwrap().set_hidden(!decorate);
        let mut decoration_guard = self.decoration.lock().unwrap();
        self.ensure_decoration(&mut decoration_guard);
        if let Some(ref dec) = *decoration_guard {
            if decorate {
                // let the server decide decorations
                dec.unset_mode();
            } else {
                // destroy the decoraiton object, so that the server does not
                // decorate us
                dec.destroy();
            }
        }
    }

    /// Set whether the window should be resizeable by the user
    ///
    /// This is not an hard blocking, as the compositor can always
    /// resize you forcibly if it wants. However it signals it that
    /// you don't want this window to be resized.
    ///
    /// Additionnaly, the decorations will stop suggesting the user
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
                    inner
                        .max_size
                        .map(|(w, h)| frame.add_borders(w as i32, h as i32)),
                );
            } else {
                // lock the min/max sizes to current size
                let (w, h) = inner.current_size;
                self.shell_surface
                    .set_min_size(Some(frame.add_borders(w as i32, h as i32)));
                self.shell_surface
                    .set_max_size(Some(frame.add_borders(w as i32, h as i32)));
            }
        }
    }

    /// Resize the decorations
    ///
    /// You should call this whenever you change the size of the contents
    /// of your window, with the new _inner size_ of your window.
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
    pub fn set_fullscreen(&self, output: Option<&Proxy<wl_output::WlOutput>>) {
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
    /// The provided size is the interior size, not counting decorations
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
    /// The provided size is the interior size, not counting decorations
    pub fn set_max_size(&mut self, size: Option<(u32, u32)>) {
        let max_size =
            size.map(|(w, h)| self.frame.lock().unwrap().add_borders(w as i32, h as i32));
        self.shell_surface.set_max_size(max_size);
        if let Some(ref mut inner) = *(self.inner.lock().unwrap()) {
            inner.max_size = size.map(|(w, h)| (w as u32, h as u32));
        }
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
    Move(Proxy<wl_seat::WlSeat>),
    /// An interactive resize should be started
    Resize(Proxy<wl_seat::WlSeat>, ResizeEdge),
    /// The frame requests to be refreshed
    Refresh,
}

/// Interface for defining the drawing of decorations
///
/// A type implementing this trait can be used to define custom
/// decorations additionnaly to the ones provided by this crate
/// and be used with `Window`.
pub trait Frame: Sized + Send {
    /// Type of errors that may occur when attempting to create a frame
    type Error;
    /// Initialize the Frame
    fn init(
        base_surface: &Proxy<wl_surface::WlSurface>,
        compositor: &Proxy<wl_compositor::WlCompositor>,
        subcompositor: &Proxy<wl_subcompositor::WlSubcompositor>,
        shm: &Proxy<wl_shm::WlShm>,
        implementation: Box<Implementation<u32, FrameRequest> + Send>,
    ) -> Result<Self, Self::Error>;
    /// Set whether the decorations should be drawn as active or not
    ///
    /// Calling this should *not* trigger a redraw, but return `true` if
    /// a redraw is needed.
    fn set_active(&mut self, active: bool) -> bool;
    /// Set whether the decorations should be drawn as maximized or not
    ///
    /// Calling this should *not* trigger a redraw, but return `true` if
    /// a redraw is needed.
    fn set_maximized(&mut self, maximized: bool) -> bool;
    /// Hide or show the decorations
    ///
    /// Calling this should *not* trigger a redraw
    fn set_hidden(&mut self, hidden: bool);
    /// Set whether interactive resize hints should be displayed
    /// and reacted to
    fn set_resizable(&mut self, resizable: bool);
    /// Notify that a new wl_seat should be handled
    fn new_seat(&mut self, seat: &Proxy<wl_seat::WlSeat>);
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
}
