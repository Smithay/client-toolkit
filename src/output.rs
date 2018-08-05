//! Types related to `wl_output` handling

use std::sync::{Arc, Mutex};

use wayland_client::protocol::wl_output::{self, Event, RequestsTrait, WlOutput};
use wayland_client::{NewProxy, Proxy};

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
    /// The model name of this output as advertized by the server
    pub model: String,
    /// The make name of this output as advertized by the server
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
    /// into account and advertizing it via `wl_buffer.set_tranform`
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
}

impl OutputInfo {
    fn new() -> OutputInfo {
        OutputInfo {
            model: String::new(),
            make: String::new(),
            location: (0, 0),
            physical_size: (0, 0),
            subpixel: Subpixel::Unknown,
            transform: Transform::Normal,
            scale_factor: 1,
            modes: Vec::new(),
        }
    }
}

struct Inner {
    outputs: Vec<(u32, Proxy<WlOutput>, OutputInfo)>,
    pendings: Vec<(Proxy<WlOutput>, Event)>,
}

impl Inner {
    fn merge(&mut self, output: &Proxy<WlOutput>) {
        let info = match self
            .outputs
            .iter_mut()
            .find(|&&mut (_, ref o, _)| o.equals(output))
        {
            Some(&mut (_, _, ref mut info)) => info,
            // trying to merge a non-existing output ?
            // well, might be some very bad luck of an
            // output being conccurently destroyed at the bad time ?
            None => {
                // clean stale state
                self.pendings.retain(|&(ref o, _)| o.is_alive());
                return;
            }
        };
        // slow, but could be improved with Vec::drain_filter
        // see https://github.com/rust-lang/rust/issues/43244
        // this vec should be pretty small at all times anyway
        while let Some(idx) = self
            .pendings
            .iter()
            .position(|&(ref o, _)| o.equals(output))
        {
            let (_, event) = self.pendings.swap_remove(idx);
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
                Event::Done => {
                    // should not happen
                    unreachable!();
                }
                Event::Mode {
                    width,
                    height,
                    refresh,
                    flags,
                } => {
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
            }
        }
    }
}

#[derive(Clone)]
/// An utility tracking the available outputs and their capabilities
pub struct OutputMgr {
    inner: Arc<Mutex<Inner>>,
}

impl OutputMgr {
    pub(crate) fn new() -> OutputMgr {
        OutputMgr {
            inner: Arc::new(Mutex::new(Inner {
                outputs: Vec::new(),
                pendings: Vec::new(),
            })),
        }
    }

    pub(crate) fn new_output(&self, id: u32, output: NewProxy<WlOutput>) {
        let inner = self.inner.clone();
        let output = output.implement(move |event, output| {
            let mut inner = inner.lock().unwrap();
            if let Event::Done = event {
                inner.merge(&output);
            } else {
                inner.pendings.push((output.clone(), event));
                if output.version() < 2 {
                    // in case of very old outputs, we can't treat the changes
                    // atomically as the Done event does not exist
                    inner.merge(&output);
                }
            }
        });
        self.inner
            .lock()
            .unwrap()
            .outputs
            .push((id, output, OutputInfo::new()));
    }

    pub(crate) fn output_removed(&self, id: u32) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(idx) = inner.outputs.iter().position(|&(i, _, _)| i == id) {
            let (_, output, _) = inner.outputs.swap_remove(idx);
            // cleanup all remaining pendings if any
            inner.pendings.retain(|&(ref o, _)| !o.equals(&output));
            if output.version() >= 3 {
                output.release();
            }
        }
    }

    /// Access the information of a specific output from its global id
    ///
    /// If the requested ouput is not found (likely because it has been destroyed)
    /// the closure is not called and `None` is returned.
    pub fn find_id<F, T>(&self, id: u32, f: F) -> Option<T>
    where
        F: FnOnce(&Proxy<wl_output::WlOutput>, &OutputInfo) -> T,
    {
        let inner = self.inner.lock().unwrap();
        if let Some(&(_, ref proxy, ref info)) = inner.outputs.iter().find(|&&(i, _, _)| i == id) {
            Some(f(proxy, info))
        } else {
            None
        }
    }

    /// Access the information of a specific output
    ///
    /// If the requested ouput is not found (likely because it has been destroyed)
    /// the closure is not called and `None` is returned.
    pub fn with_info<F, T>(&self, output: &Proxy<WlOutput>, f: F) -> Option<T>
    where
        F: FnOnce(u32, &OutputInfo) -> T,
    {
        let inner = self.inner.lock().unwrap();
        if let Some(&(id, _, ref info)) = inner
            .outputs
            .iter()
            .find(|&&(_, ref o, _)| o.equals(output))
        {
            Some(f(id, info))
        } else {
            None
        }
    }

    /// Access all output information
    pub fn with_all<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&[(u32, Proxy<WlOutput>, OutputInfo)]) -> T,
    {
        let inner = self.inner.lock().unwrap();
        f(&inner.outputs)
    }
}
