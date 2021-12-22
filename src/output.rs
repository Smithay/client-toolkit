use std::{
    fmt::{self, Display, Formatter},
    mem,
    sync::{Arc, Mutex},
};

use wayland_client::{
    protocol::wl_output::{self, Subpixel, Transform},
    ConnectionHandle, DataInit, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy,
    QueueHandle, WEnum,
};
use wayland_protocols::unstable::xdg_output::v1::client::{
    zxdg_output_manager_v1::{self, ZxdgOutputManagerV1},
    zxdg_output_v1,
};

use crate::registry::{RegistryHandle, RegistryHandler};

pub trait OutputHandler {
    /// A new output has been advertised.
    fn new_output(&mut self, info: OutputInfo);

    /// An existing output has changed.
    fn update_output(&mut self, info: OutputInfo);

    /// An output is no longer advertised.
    ///
    /// The info passed to this function was the state of the output before destruction.
    fn output_destroyed(&mut self, info: OutputInfo);
}

#[derive(Debug)]
pub struct OutputState {
    xdg: Option<ZxdgOutputManagerV1>,
    outputs: Vec<OutputInner>,
}

impl OutputState {
    pub fn new() -> OutputState {
        OutputState { xdg: None, outputs: vec![] }
    }
}

#[derive(Debug, Clone)]
pub struct OutputData(Arc<Mutex<Option<OutputInfo>>>);

impl OutputData {
    pub fn new() -> OutputData {
        OutputData(Arc::new(Mutex::new(None)))
    }

    pub fn scale_factor(&self) -> i32 {
        let guard = self.0.lock().unwrap();

        guard.as_ref().map(|info| info.scale_factor).unwrap_or(1)
    }
}

#[derive(Debug)]
pub struct OutputDispatch<'s, H: OutputHandler>(pub &'s mut OutputState, pub &'s mut H);

impl<H: OutputHandler> DelegateDispatchBase<wl_output::WlOutput> for OutputDispatch<'_, H> {
    type UserData = OutputData;
}

impl<D, H> DelegateDispatch<wl_output::WlOutput, D> for OutputDispatch<'_, H>
where
    H: OutputHandler,
    D: Dispatch<wl_output::WlOutput, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        data: &Self::UserData,
        _cxhandle: &mut ConnectionHandle,
        _qhandle: &QueueHandle<D>,
        _: &mut DataInit<'_>,
    ) {
        match event {
            wl_output::Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel,
                make,
                model,
                transform,
            } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                let pending = inner.data.pending(true, false);

                pending.info.location = (x, y);
                pending.info.physical_size = (physical_width, physical_height);
                pending.info.subpixel = match subpixel {
                    WEnum::Value(subpixel) => subpixel,
                    WEnum::Unknown(_) => todo!("Warn about invalid subpixel value"),
                };
                pending.info.make = make;
                pending.info.model = model;
                pending.info.transform = match transform {
                    WEnum::Value(subpixel) => subpixel,
                    WEnum::Unknown(_) => todo!("Warn about invalid transform value"),
                };
            }

            wl_output::Event::Mode { flags, width, height, refresh } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                let pending = inner.data.pending(true, false);

                if let Some((index, _)) = pending.info.modes.iter().enumerate().find(|(_, mode)| {
                    mode.dimensions == (width, height) && mode.refresh_rate == refresh
                }) {
                    // We found a match, remove the old mode.
                    pending.info.modes.remove(index);
                }

                let flags = match flags {
                    WEnum::Value(flags) => flags,
                    WEnum::Unknown(_) => panic!("Invalid flags"),
                };

                let current = flags.contains(wl_output::Mode::Current);
                let preferred = flags.contains(wl_output::Mode::Preferred);

                // Now create the new mode.
                pending.info.modes.push(Mode {
                    dimensions: (width, height),
                    refresh_rate: refresh,
                    current,
                    preferred,
                });

                let index = pending.info.modes.len() - 1;

                // Any mode that isn't current is deprecated, let's deprecate any existing modes that may be
                // marked as current.
                //
                // If a new mode is advertised as preferred, then mark the existing preferred mode as not.
                pending.info.modes.iter_mut().enumerate().for_each(|(mode_index, mode)| {
                    if index != mode_index {
                        // This mode is no longer preferred.
                        if mode.preferred && preferred {
                            mode.preferred = false;
                        }

                        // This mode is no longer current.
                        if mode.current && current {
                            mode.current = false;
                        }
                    }
                });
            }

            wl_output::Event::Scale { factor } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                let pending = inner.data.pending(true, false);
                pending.info.scale_factor = factor;
            }

            wl_output::Event::Name { name } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                let pending = inner.data.pending(true, false);
                pending.info.name = Some(name);
            }

            wl_output::Event::Description { description } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                let pending = inner.data.pending(true, false);
                pending.info.description = Some(description);
            }

            wl_output::Event::Done => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                let info = inner.data.ready();

                // Set the user data
                data.set(info.clone());

                if inner.just_created {
                    inner.just_created = false;
                    self.1.new_output(info);
                } else {
                    self.1.update_output(info);
                }
            }

            _ => unreachable!(),
        }
    }
}

impl<H: OutputHandler> DelegateDispatchBase<zxdg_output_manager_v1::ZxdgOutputManagerV1>
    for OutputDispatch<'_, H>
{
    type UserData = ();
}

impl<D, H> DelegateDispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, D>
    for OutputDispatch<'_, H>
where
    H: OutputHandler,
    D: Dispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        _: &zxdg_output_manager_v1::ZxdgOutputManagerV1,
        _: zxdg_output_manager_v1::Event,
        _: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
        _: &mut DataInit<'_>,
    ) {
        unreachable!("zxdg_output_manager_v1 has no events")
    }
}

impl<H: OutputHandler> DelegateDispatchBase<zxdg_output_v1::ZxdgOutputV1>
    for OutputDispatch<'_, H>
{
    type UserData = OutputData;
}

impl<D, H> DelegateDispatch<zxdg_output_v1::ZxdgOutputV1, D> for OutputDispatch<'_, H>
where
    H: OutputHandler,
    D: Dispatch<zxdg_output_v1::ZxdgOutputV1, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        output: &zxdg_output_v1::ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        data: &Self::UserData,
        _cxhandle: &mut ConnectionHandle,
        _qhandle: &QueueHandle<D>,
        _: &mut DataInit<'_>,
    ) {
        match event {
            // Already provided by wl_output
            zxdg_output_v1::Event::LogicalPosition { x: _, y: _ } => (),
            zxdg_output_v1::Event::LogicalSize { width: _, height: _ } => (),

            zxdg_output_v1::Event::Name { name } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| inner.xdg_output.as_ref() == Some(output))
                    .expect("Received event for dead output");

                let pending = inner.data.pending(false, true);

                pending.info.name = Some(name);
            }

            zxdg_output_v1::Event::Description { description } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| inner.xdg_output.as_ref() == Some(output))
                    .expect("Received event for dead output");

                let pending = inner.data.pending(false, true);

                pending.info.description = Some(description);
            }

            zxdg_output_v1::Event::Done => {
                // This event is deprecated starting in version 3, wl_output::done should be sent instead.
                if output.version() < 3 {
                    let inner = self
                        .0
                        .outputs
                        .iter_mut()
                        .find(|inner| inner.xdg_output.as_ref() == Some(output))
                        .expect("Received event for dead output");

                    let info = inner.data.ready();

                    // Set the user data
                    data.set(info.clone());

                    if inner.just_created {
                        inner.just_created = false;
                        self.1.new_output(info);
                    } else {
                        self.1.update_output(info);
                    }
                }
            }

            _ => unreachable!(),
        }
    }
}

impl<D> RegistryHandler<D> for OutputState
where
    D: Dispatch<wl_output::WlOutput, UserData = OutputData>
        + Dispatch<zxdg_output_v1::ZxdgOutputV1, UserData = OutputData>
        + Dispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, UserData = ()>
        + 'static,
{
    fn new_global(
        &mut self,
        cx: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
        handle: &mut RegistryHandle,
    ) {
        match interface {
            "wl_output" => {
                let wl_output = handle
                    .bind_cached::<wl_output::WlOutput, _, _, _>(cx, qh, name, || {
                        (u32::min(version, 4), OutputData::new())
                    })
                    .expect("Failed to bind global");

                let version = wl_output.version();

                self.outputs.push(OutputInner {
                    name,
                    wl_output: wl_output.clone(),
                    xdg_output: None,
                    just_created: true,
                    // wl_output::done was added in version 2.
                    // If we have an output at version 1, assume the data was already sent.
                    data: if version > 1 {
                        OutputStatus::Pending(Pending {
                            pending_wl: true,
                            pending_xdg: self.xdg.is_some(),
                            info: OutputInfo::new(name),
                        })
                    } else {
                        OutputStatus::Ready { info: OutputInfo::new(name) }
                    },
                });

                if self.xdg.is_some() {
                    let xdg = self.xdg.as_ref().unwrap();

                    let data = wl_output.data::<OutputData>().unwrap().clone();

                    let xdg_output = xdg.get_xdg_output(cx, wl_output, qh, data).unwrap();
                    self.outputs.last_mut().unwrap().xdg_output = Some(xdg_output);
                }
            }

            "zxdg_output_manager_v1" => {
                let global = handle
                    .bind_once::<zxdg_output_manager_v1::ZxdgOutputManagerV1, _, _>(
                        cx,
                        qh,
                        name,
                        u32::min(version, 3),
                        (),
                    )
                    .expect("Failed to bind global");

                self.xdg = Some(global);

                let xdg = self.xdg.as_ref().unwrap();

                // Because the order in which globals are advertised is undefined, we need to get the extension of any
                // wl_output we have already gotten.
                self.outputs.iter_mut().for_each(|output| {
                    let data = output.wl_output.data::<OutputData>().unwrap().clone();

                    let xdg_output =
                        xdg.get_xdg_output(cx, output.wl_output.clone(), qh, data).unwrap();
                    output.xdg_output = Some(xdg_output);
                });
            }

            _ => (),
        }
    }

    fn remove_global(&mut self, cx: &mut ConnectionHandle, name: u32) {
        self.outputs.retain(|inner| {
            let destroy = inner.name != name;

            if destroy {
                if let Some(xdg_output) = &inner.xdg_output {
                    xdg_output.destroy(cx);
                }

                inner.wl_output.release(cx);
            }

            // FIXME: How do we tell the client that the output was destroyed?

            destroy
        })
    }
}

#[derive(Debug, Clone)]
pub struct Mode {
    /// Number of pixels of this mode in format `(width, height)`
    ///
    /// for example `(1920, 1080)`
    pub dimensions: (i32, i32),

    /// Refresh rate for this mode.
    ///
    /// The refresh rate is specified in terms of millihertz (mHz). To convert approximately to Hertz,
    /// divide the value by 1000.
    ///
    /// This value could be zero if an output has no correct refresh rate, such as a virtual output.
    pub refresh_rate: i32,

    /// Whether this is the current mode for this output.
    ///
    /// Per the Wayland protocol, non-current modes are deprecated and clients should not rely on deprecated
    /// modes.
    pub current: bool,

    /// Whether this is the preferred mode for this output.
    pub preferred: bool,
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.current {
            write!(f, "(current) ")?;
        }

        if self.preferred {
            write!(f, "(preferred) ")?;
        }

        write!(
            f,
            "{}×{}px @ {}.{:03} Hz",
            self.dimensions.0,
            self.dimensions.1,
            self.refresh_rate / 1000,
            self.refresh_rate % 1000
        )
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OutputInfo {
    /// The id of the output.
    ///
    /// This corresponds to the global id of the wl_output.
    pub id: u32,

    /// The model name of this output as advertised by the server.
    pub model: String,

    /// The make name of this output as advertised by the server.
    pub make: String,

    /// Location of the top-left corner of this output in compositor space.
    ///
    /// Note that the compositor may decide to always report (0,0) if it decides clients are not allowed to
    /// know this information.
    pub location: (i32, i32),

    /// Physical dimensions of this output, in millimeters.
    ///
    /// This value may be set to (0, 0) if a physical size does not make sense for the output (e.g. projectors
    /// and virtual outputs).
    pub physical_size: (i32, i32),

    /// The subpixel layout for this output.
    pub subpixel: Subpixel,

    /// The current transformation applied to this output
    ///
    /// You can pre-render your buffers taking this information into account and advertising it via
    /// `wl_buffer.set_transform` for better performance.
    pub transform: Transform,

    /// The scaling factor of this output
    ///
    /// Any buffer whose scaling factor does not match the one of the output it is displayed on will be
    /// rescaled accordingly.
    ///
    /// For example, a buffer of scaling factor 1 will be doubled in size if the output scaling factor is 2.
    ///
    /// You can pre-render your buffers taking this information into account and advertising it via
    /// `wl_surface.set_buffer_scale` so you may advertise a higher detail image.
    pub scale_factor: i32,

    /// Possible modes for an output.
    pub modes: Vec<Mode>,

    /// The name of the this output as advertised by the surface.
    ///
    /// Examples of names include 'HDMI-A-1', 'WL-1', 'X11-1', etc. However, do not assume that the name is a
    /// reflection of an underlying DRM connector, X11 connection, etc.
    ///
    /// Compositors are not required to provide a name for the output and the value may be [`None`].
    ///
    /// The name will be [`None`] if the compositor does not support version 4 of the wl-output protocol or
    /// version 2 of the zxdg-output-v1 protocol.
    pub name: Option<String>,

    /// The description of this output as advertised by the server
    ///
    /// The description is a UTF-8 string with no convention defined for its contents. The description is not
    /// guaranteed to be unique among all wl_output globals. Examples might include 'Foocorp 11" Display' or
    /// 'Virtual X11 output via :1'.
    ///
    /// Compositors are not required to provide a description of the output and the value may be [`None`].
    ///
    /// The value will be [`None`] if the compositor does not support version 4 of the wl-output
    /// protocol, version 2 of the zxdg-output-v1 protocol.
    pub description: Option<String>,
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
            modes: vec![],
            name: None,
            description: None,
        }
    }
}

impl OutputData {
    pub(crate) fn set(&self, info: OutputInfo) {
        let mut guard = self.0.lock().unwrap();

        *guard = Some(info);
    }
}

#[derive(Debug)]
struct OutputInner {
    /// The name of the wl_output global.
    name: u32,
    wl_output: wl_output::WlOutput,
    xdg_output: Option<zxdg_output_v1::ZxdgOutputV1>,
    /// Whether this output was just created and has not an event yet.
    just_created: bool,

    data: OutputStatus,
}

#[derive(Debug)]
enum OutputStatus {
    Ready {
        info: OutputInfo,
    },

    Pending(Pending),

    /// A variant of output data set while changing from Ready to pending.
    ///
    /// This is placed on the original memory of the Ready variant so the output info can be taken.
    IntermediateState,
}

#[derive(Debug)]
struct Pending {
    pending_wl: bool,
    pending_xdg: bool,
    info: OutputInfo,
}

impl Default for OutputStatus {
    fn default() -> Self {
        OutputStatus::IntermediateState
    }
}

impl OutputStatus {
    /// Returns the pending output data, converting the OutputData to pending if necessary.
    fn pending(&mut self, wl: bool, xdg: bool) -> &mut Pending {
        match self {
            OutputStatus::Ready { .. } => {
                // Round-about dance to take the OutputInfo out of Ready and place it into Pending.
                let data = mem::take(self);
                let info = match data {
                    OutputStatus::Ready { info } => info,
                    _ => unreachable!(),
                };

                *self = OutputStatus::Pending(Pending { pending_wl: wl, pending_xdg: xdg, info });
            }

            OutputStatus::Pending(Pending { pending_wl, pending_xdg, .. }) => {
                *pending_wl |= wl;
                *pending_xdg |= xdg;
            }

            OutputStatus::IntermediateState => unreachable!(),
        }

        match self {
            OutputStatus::Pending(pending) => pending,
            _ => unreachable!(),
        }
    }

    /// Returns the output data, and changes the enum variant to ready.
    fn ready(&mut self) -> OutputInfo {
        match self {
            // Already ready
            OutputStatus::Ready { info } => info.clone(),

            OutputStatus::Pending(_) => {
                // Round-about dance to take the OutputInfo out of Pending and place it into Ready.
                let pending = mem::take(self);
                let pending = match pending {
                    OutputStatus::Pending(pending) => pending,
                    _ => unreachable!(),
                };

                let info = pending.info.clone();
                *self = OutputStatus::Ready { info: pending.info };

                info
            }

            OutputStatus::IntermediateState => unreachable!(),
        }
    }
}
