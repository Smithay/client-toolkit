//! Types and functions related to graphical outputs
//!
//! This modules provides two main elements. The first is the
//! [`OutputHandler`](struct.OutputHandler.html) type, which is a
//! [`MultiGlobalHandler`](../environment/trait.MultiGlobalHandler.html) for
//! use with the [`init_environment!`](../macro.init_environment.html) macro. It is automatically
//! included if you use the [`new_default_environment!`](../macro.new_default_environment.html).
//!
//! The second is the [`with_output_info`](fn.with_output_info.html) with allows you to
//! access the information associated to this output, as an [`OutputInfo`](struct.OutputInfo.html).

use std::{
    cell::RefCell,
    rc::{self, Rc},
    sync::{self, Arc, Mutex},
};

use wayland_client::{
    protocol::{
        wl_output::{self, Event, WlOutput},
        wl_registry,
    },
    Attached, DispatchData, Main,
};

pub use wayland_client::protocol::wl_output::{Subpixel, Transform};

/// A possible mode for an output
#[derive(Copy, Clone, Debug)]
pub struct Mode {
    /// Number of pixels of this mode in format `(width, height)`
    ///
    /// for example `(1920, 1080)`
    pub dimensions: (i32, i32),
    /// Refresh rate for this mode, in mHz
    pub refresh_rate: i32,
    /// Whether this is the current mode for this output
    pub is_current: bool,
    /// Whether this is the preferred mode for this output
    pub is_preferred: bool,
}

#[derive(Clone, Debug)]
/// Compiled information about an output
pub struct OutputInfo {
    /// The ID of this output as a global
    pub id: u32,
    /// The model name of this output as advertised by the server
    pub model: String,
    /// The make name of this output as advertised by the server
    pub make: String,
    /// Location of the top-left corner of this output in compositor
    /// space
    ///
    /// Note that the compositor may decide to always report (0,0) if
    /// it decides clients are not allowed to know this information.
    pub location: (i32, i32),
    /// Physical dimensions of this output, in unspecified units
    pub physical_size: (i32, i32),
    /// The subpixel layout for this output
    pub subpixel: Subpixel,
    /// The current transformation applied to this output
    ///
    /// You can pre-render your buffers taking this information
    /// into account and advertising it via `wl_buffer.set_tranform`
    /// for better performances.
    pub transform: Transform,
    /// The scaling factor of this output
    ///
    /// Any buffer whose scaling factor does not match the one
    /// of the output it is displayed on will be rescaled accordingly.
    ///
    /// For example, a buffer of scaling factor 1 will be doubled in
    /// size if the output scaling factor is 2.
    pub scale_factor: i32,
    /// Possible modes for an output
    pub modes: Vec<Mode>,
    /// Has this output been unadvertized by the registry
    ///
    /// If this is the case, it has become inert, you might want to
    /// call its `release()` method if you don't plan to use it any
    /// longer.
    pub obsolete: bool,
}

impl OutputInfo {
    fn new(id: u32) -> OutputInfo {
        OutputInfo {
            id,
            model: String::new(),
            make: String::new(),
            location: (0, 0),
            physical_size: (0, 0),
            subpixel: Subpixel::Unknown,
            transform: Transform::Normal,
            scale_factor: 1,
            modes: Vec::new(),
            obsolete: false,
        }
    }
}

type OutputCallback = dyn Fn(WlOutput, &OutputInfo, DispatchData) + Send + Sync;

enum OutputData {
    Ready { info: OutputInfo, callbacks: Vec<sync::Weak<OutputCallback>> },
    Pending { id: u32, events: Vec<Event>, callbacks: Vec<sync::Weak<OutputCallback>> },
}

type OutputStatusCallback = dyn FnMut(WlOutput, &OutputInfo, DispatchData) + 'static;

/// A handler for `wl_output`
///
/// This handler can be used for managing `wl_output` in the
/// [`init_environment!`](../macro.init_environment.html) macro, and is automatically
/// included in [`new_default_environment!`](../macro.new_default_environment.html).
///
/// It aggregates the output information and makes it available via the
/// [`with_output_info`](fn.with_output_info.html) function.
pub struct OutputHandler {
    outputs: Vec<(u32, Attached<WlOutput>)>,
    status_listeners: Rc<RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>>,
}

impl OutputHandler {
    /// Create a new instance of this handler
    pub fn new() -> OutputHandler {
        OutputHandler { outputs: Vec::new(), status_listeners: Rc::new(RefCell::new(Vec::new())) }
    }
}

impl crate::environment::MultiGlobalHandler<WlOutput> for OutputHandler {
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        _: DispatchData,
    ) {
        // We currently support wl_output up to version 3
        let version = std::cmp::min(version, 3);
        let output = registry.bind::<WlOutput>(version, id);
        if version > 1 {
            // wl_output.done event was only added at version 2
            // In case of an old version 1, we just behave as if it was send at the start
            output.as_ref().user_data().set_threadsafe(|| {
                Mutex::new(OutputData::Pending { id, events: vec![], callbacks: vec![] })
            });
        } else {
            output.as_ref().user_data().set_threadsafe(|| {
                Mutex::new(OutputData::Ready { info: OutputInfo::new(id), callbacks: vec![] })
            });
        }
        let status_listeners_handle = self.status_listeners.clone();
        output.quick_assign(move |output, event, ddata| {
            process_output_event(output, event, ddata, &status_listeners_handle)
        });
        self.outputs.push((id, (*output).clone()));
    }
    fn removed(&mut self, id: u32, mut ddata: DispatchData) {
        let status_listeners_handle = self.status_listeners.clone();
        self.outputs.retain(|(i, o)| {
            if *i != id {
                true
            } else {
                make_obsolete(o, ddata.reborrow(), &status_listeners_handle);
                false
            }
        });
    }
    fn get_all(&self) -> Vec<Attached<WlOutput>> {
        self.outputs.iter().map(|(_, o)| o.clone()).collect()
    }
}

fn process_output_event(
    output: Main<WlOutput>,
    event: Event,
    mut ddata: DispatchData,
    listeners: &RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>,
) {
    let udata_mutex = output
        .as_ref()
        .user_data()
        .get::<Mutex<OutputData>>()
        .expect("SCTK: wl_output has invalid UserData");
    let mut udata = udata_mutex.lock().unwrap();
    if let Event::Done = event {
        let (id, pending_events, mut callbacks) =
            if let OutputData::Pending { id, events: ref mut v, callbacks: ref mut cb } = *udata {
                (id, std::mem::replace(v, vec![]), std::mem::replace(cb, vec![]))
            } else {
                // a Done event on an output that is already ready => nothing to do
                return;
            };
        let mut info = OutputInfo::new(id);
        for evt in pending_events {
            merge_event(&mut info, evt);
        }
        notify(&output, &info, ddata.reborrow(), &mut callbacks);
        notify_status_listeners(&output, &info, ddata, listeners);
        *udata = OutputData::Ready { info, callbacks };
    } else {
        match *udata {
            OutputData::Pending { events: ref mut v, .. } => v.push(event),
            OutputData::Ready { ref mut info, ref mut callbacks } => {
                merge_event(info, event);
                notify(&output, info, ddata, callbacks);
            }
        }
    }
}

fn make_obsolete(
    output: &Attached<WlOutput>,
    mut ddata: DispatchData,
    listeners: &RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>,
) {
    let udata_mutex = output
        .as_ref()
        .user_data()
        .get::<Mutex<OutputData>>()
        .expect("SCTK: wl_output has invalid UserData");
    let mut udata = udata_mutex.lock().unwrap();
    let (id, mut callbacks) = match *udata {
        OutputData::Ready { ref mut info, ref mut callbacks } => {
            info.obsolete = true;
            notify(output, info, ddata.reborrow(), callbacks);
            notify_status_listeners(&output, info, ddata, listeners);
            return;
        }
        OutputData::Pending { id, callbacks: ref mut cb, .. } => {
            (id, std::mem::replace(cb, vec![]))
        }
    };
    let mut info = OutputInfo::new(id);
    info.obsolete = true;
    notify(output, &info, ddata.reborrow(), &mut callbacks);
    notify_status_listeners(&output, &info, ddata, listeners);
    *udata = OutputData::Ready { info, callbacks };
}

fn merge_event(info: &mut OutputInfo, event: Event) {
    match event {
        Event::Geometry {
            x,
            y,
            physical_width,
            physical_height,
            subpixel,
            model,
            make,
            transform,
        } => {
            info.location = (x, y);
            info.physical_size = (physical_width, physical_height);
            info.subpixel = subpixel;
            info.transform = transform;
            info.model = model;
            info.make = make;
        }
        Event::Scale { factor } => {
            info.scale_factor = factor;
        }
        Event::Mode { width, height, refresh, flags } => {
            let mut found = false;
            if let Some(mode) = info
                .modes
                .iter_mut()
                .find(|m| m.dimensions == (width, height) && m.refresh_rate == refresh)
            {
                // this mode already exists, update it
                mode.is_preferred = flags.contains(wl_output::Mode::Preferred);
                mode.is_current = flags.contains(wl_output::Mode::Current);
                found = true;
            }
            if !found {
                // otherwise, add it
                info.modes.push(Mode {
                    dimensions: (width, height),
                    refresh_rate: refresh,
                    is_preferred: flags.contains(wl_output::Mode::Preferred),
                    is_current: flags.contains(wl_output::Mode::Current),
                })
            }
        }
        // ignore all other events
        _ => (),
    }
}

fn notify(
    output: &WlOutput,
    info: &OutputInfo,
    mut ddata: DispatchData,
    callbacks: &mut Vec<sync::Weak<OutputCallback>>,
) {
    callbacks.retain(|weak| {
        if let Some(arc) = sync::Weak::upgrade(weak) {
            (*arc)(output.clone(), info, ddata.reborrow());
            true
        } else {
            false
        }
    });
}

fn notify_status_listeners(
    output: &Attached<WlOutput>,
    info: &OutputInfo,
    mut ddata: DispatchData,
    listeners: &RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>,
) {
    // Notify the callbacks listening for new outputs
    listeners.borrow_mut().retain(|lst| {
        if let Some(cb) = rc::Weak::upgrade(lst) {
            (&mut *cb.borrow_mut())(output.detach(), info, ddata.reborrow());
            true
        } else {
            false
        }
    })
}

/// Access the info associated with this output
///
/// The provided closure is given the [`OutputInfo`](struct.OutputInfo.html) as argument,
/// and its return value is returned from this function.
///
/// If the provided `WlOutput` has not yet been initialized or is not managed by SCTK, `None` is returned.
///
/// If the output has been removed by the compositor, the `obsolete` field of the `OutputInfo`
/// will be set to `true`. This handler will not automatically detroy the output by calling its
/// `release` method, to avoid interfering with your logic.
pub fn with_output_info<T, F: FnOnce(&OutputInfo) -> T>(output: &WlOutput, f: F) -> Option<T> {
    if let Some(ref udata_mutex) = output.as_ref().user_data().get::<Mutex<OutputData>>() {
        let udata = udata_mutex.lock().unwrap();
        match *udata {
            OutputData::Ready { ref info, .. } => Some(f(info)),
            OutputData::Pending { .. } => None,
        }
    } else {
        None
    }
}

/// Add a listener to this output
///
/// The provided closure will be called whenever a property of the output changes,
/// including when it is removed by the compositor (in this case it'll be marked as
/// obsolete).
///
/// The returned [`OutputListener`](struct.OutputListener) keeps your callback alive,
/// dropping it will disable the callback and free the closure.
pub fn add_output_listener<F: Fn(WlOutput, &OutputInfo, DispatchData) + Send + Sync + 'static>(
    output: &WlOutput,
    f: F,
) -> OutputListener {
    let arc = Arc::new(f) as Arc<_>;

    if let Some(udata_mutex) = output.as_ref().user_data().get::<Mutex<OutputData>>() {
        let mut udata = udata_mutex.lock().unwrap();

        match *udata {
            OutputData::Pending { ref mut callbacks, .. } => {
                callbacks.push(Arc::downgrade(&arc));
            }
            OutputData::Ready { ref mut callbacks, .. } => {
                callbacks.push(Arc::downgrade(&arc));
            }
        }
    }

    OutputListener { _cb: arc }
}

/// A handle to an output listener callback
///
/// Dropping it disables the associated callback and frees the closure.
pub struct OutputListener {
    _cb: Arc<dyn Fn(WlOutput, &OutputInfo, DispatchData) + Send + Sync + 'static>,
}

/// A handle to an output status callback
///
/// Dropping it disables the associated callback and frees the closure.
pub struct OutputStatusListener {
    _cb: Rc<RefCell<OutputStatusCallback>>,
}

/// Trait representing the OutputHandler functions
///
/// Implementing this trait on your inner environment struct used with the
/// [`environment!`](../macro.environment.html) by delegating it to its
/// [`OutputHandler`](struct.OutputHandler.html) field will make available the output-associated
/// method on your [`Environment`](../environment/struct.Environment.html).
pub trait OutputHandling {
    /// Insert a listener for output creation and removal events
    fn listen<F: FnMut(WlOutput, &OutputInfo, DispatchData) + 'static>(
        &mut self,
        f: F,
    ) -> OutputStatusListener;
}

impl OutputHandling for OutputHandler {
    fn listen<F: FnMut(WlOutput, &OutputInfo, DispatchData) + 'static>(
        &mut self,
        f: F,
    ) -> OutputStatusListener {
        let rc = Rc::new(RefCell::new(f)) as Rc<_>;
        self.status_listeners.borrow_mut().push(Rc::downgrade(&rc));
        OutputStatusListener { _cb: rc }
    }
}

impl<E: OutputHandling> crate::environment::Environment<E> {
    /// Insert a new listener for outputs
    ///
    /// The provided closure will be invoked whenever a `wl_output` is created or removed.
    ///
    /// Note that if outputs already exist when this callback is setup, it'll not be invoked on them.
    /// For you to be notified of them as well, you need to first process them manually by calling
    /// `.get_all_outputs()`.
    ///
    /// The returned [`OutputStatusListener`](../output/struct.OutputStatusListener.hmtl) keeps your
    /// callback alive, dropping it will disable it.
    #[must_use = "the returned OutputStatusListener keeps your callback alive, dropping it will disable it"]
    pub fn listen_for_outputs<F: FnMut(WlOutput, &OutputInfo, DispatchData) + 'static>(
        &self,
        f: F,
    ) -> OutputStatusListener {
        self.with_inner(move |inner| OutputHandling::listen(inner, f))
    }
}

impl<E: crate::environment::MultiGlobalHandler<WlOutput>> crate::environment::Environment<E> {
    /// Shorthand method to retrieve the list of outputs
    pub fn get_all_outputs(&self) -> Vec<WlOutput> {
        self.get_all_globals::<WlOutput>().into_iter().map(|o| o.detach()).collect()
    }
}
