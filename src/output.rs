use std::fmt::{self, Display, Formatter};

use wayland_client::{
    protocol::wl_output::{self, Subpixel, Transform},
    ConnectionHandle, DataInit, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy,
    QueueHandle, WEnum,
};
use wayland_protocols::unstable::xdg_output::v1::client::{
    zxdg_output_manager_v1::{self, ZxdgOutputManagerV1},
    zxdg_output_v1,
};

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

impl Default for OutputState {
    fn default() -> Self {
        unreachable!("Default requirement is marked for removal")
    }
}

#[derive(Debug)]
pub struct OutputDispatch<'s, H: OutputHandler>(pub &'s mut OutputState, pub &'s mut H);

impl<H: OutputHandler> DelegateDispatchBase<wl_output::WlOutput> for OutputDispatch<'_, H> {
    type UserData = ();
}

impl<H: OutputHandler> DelegateDispatch<wl_output::WlOutput, H> for OutputDispatch<'_, H>
where
    H: Dispatch<wl_output::WlOutput, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _data: &Self::UserData,
        _cxhandle: &mut ConnectionHandle,
        _qhandle: &QueueHandle<H>,
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

                let mut pending = &mut inner.pending;
                pending.location = (x, y);
                pending.physical_size = (physical_width, physical_height);
                // TODO: Not ideal?
                pending.subpixel = match subpixel {
                    WEnum::Value(value) => value,
                    WEnum::Unknown(_) => panic!("Invalid subpixel?"),
                };
                pending.make = make;
                pending.model = model;
                // TODO: Not ideal?
                pending.transform = match transform {
                    WEnum::Value(value) => value,
                    WEnum::Unknown(_) => panic!("Invalid transform?"),
                };
            }

            wl_output::Event::Mode { flags: _, width: _, height: _, refresh: _ } => todo!(),

            wl_output::Event::Scale { factor } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                inner.pending.scale_factor = factor;
            }

            wl_output::Event::Done => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                inner.current = Some(inner.pending.clone());

                let event_info = inner.pending.clone();

                if inner.just_created {
                    inner.just_created = false;
                    self.1.new_output(event_info);
                } else {
                    self.1.update_output(event_info);
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

impl<H: OutputHandler> DelegateDispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, H>
    for OutputDispatch<'_, H>
where
    H: Dispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        _: &zxdg_output_manager_v1::ZxdgOutputManagerV1,
        _: zxdg_output_manager_v1::Event,
        _: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<H>,
        _: &mut DataInit<'_>,
    ) {
        unreachable!("zxdg_output_manager_v1 has no events")
    }
}

impl<H: OutputHandler> DelegateDispatchBase<zxdg_output_v1::ZxdgOutputV1>
    for OutputDispatch<'_, H>
{
    type UserData = ();
}

impl<H: OutputHandler> DelegateDispatch<zxdg_output_v1::ZxdgOutputV1, H> for OutputDispatch<'_, H>
where
    H: Dispatch<wl_output::WlOutput, UserData = Self::UserData>
        + Dispatch<zxdg_output_v1::ZxdgOutputV1, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        output: &zxdg_output_v1::ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        _data: &Self::UserData,
        _cxhandle: &mut ConnectionHandle,
        _qhandle: &QueueHandle<H>,
        _: &mut DataInit<'_>,
    ) {
        match event {
            zxdg_output_v1::Event::LogicalPosition { x: _, y: _ } => todo!(),

            zxdg_output_v1::Event::LogicalSize { width: _, height: _ } => todo!(),

            zxdg_output_v1::Event::Name { name: _ } => todo!(),

            zxdg_output_v1::Event::Description { description: _ } => {
                // TODO: Immutable in version 2 and below, mutable in version 3.
                todo!()
            }

            zxdg_output_v1::Event::Done => {
                // This event is deprecated starting in version 3, wl_output::done should be sent instead.
                if output.version() < 3 {
                    todo!("Send notification")
                } else {
                    // TODO: Warn in log about bad compositor impl?
                }
            }

            _ => unreachable!(),
        }
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

    current: Option<OutputInfo>,
    pending: OutputInfo,
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
    pub refresh_rate: i32,

    /// Whether this is the current mode for this output
    pub current: bool,

    /// Whether this is the preferred mode for this output
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
