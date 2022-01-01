use std::{
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use wayland_client::{
    protocol::wl_output::{self, Subpixel, Transform},
    ConnectionHandle, DelegateDispatch, DelegateDispatchBase, Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::unstable::xdg_output::v1::client::{
    zxdg_output_manager_v1::{self, ZxdgOutputManagerV1},
    zxdg_output_v1,
};

use crate::registry::{RegistryHandle, RegistryHandler};

pub trait OutputHandler<D> {
    /// A new output has been advertised.
    fn new_output(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &OutputState,
        output: wl_output::WlOutput,
    );

    /// An existing output has changed.
    fn update_output(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &OutputState,
        output: wl_output::WlOutput,
    );

    /// An output is no longer advertised.
    ///
    /// The info passed to this function was the state of the output before destruction.
    fn output_destroyed(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        state: &OutputState,
        output: wl_output::WlOutput,
    );
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

    /// Returns an iterator over all outputs.
    pub fn outputs(&self) -> impl Iterator<Item = wl_output::WlOutput> {
        self.outputs.iter().map(|output| &output.wl_output).cloned().collect::<Vec<_>>().into_iter()
    }

    /// Returns information about an output.
    ///
    /// This may be none if the output has been destroyed or the compositor has not sent information about the
    /// output yet.
    pub fn info(&self, output: &wl_output::WlOutput) -> Option<OutputInfo> {
        self.outputs
            .iter()
            .find(|inner| &inner.wl_output == output)
            .and_then(|inner| inner.current_info.clone())
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
            // Print the refresh rate in hertz since it is more familiar unit.
            self.refresh_rate / 1000,
            self.refresh_rate % 1000
        )
    }
}

/// Information about an output.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OutputInfo {
    /// The id of the output.
    ///
    /// This corresponds to the global `name` of the wl_output.
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

#[derive(Debug)]
pub struct OutputDispatch<'s, D, H: OutputHandler<D>>(
    pub &'s mut OutputState,
    pub &'s mut H,
    pub PhantomData<D>,
);

#[macro_export]
macro_rules! delegate_output {
    ($ty: ty => $inner: ty: |$dispatcher: ident| $closure: block) => {
        type __WlOutput = $crate::reexports::client::protocol::wl_output::WlOutput;
        type __ZxdgOutputV1 =
            $crate::reexports::protocols::unstable::xdg_output::v1::client::zxdg_output_v1::ZxdgOutputV1;
        type __ZxdgOutputManagerV1 =
            $crate::reexports::protocols::unstable::xdg_output::v1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1;

        $crate::reexports::client::delegate_dispatch!($ty: <UserData = $crate::output::OutputData> [
            __WlOutput,
            __ZxdgOutputV1
        ] => $crate::output::OutputDispatch<'_, $ty, $inner> ; |$dispatcher| {
            $closure
        });

        // Zxdg manager
        $crate::reexports::client::delegate_dispatch!($ty: <UserData = ()> [
            __ZxdgOutputManagerV1
        ] => $crate::output::OutputDispatch<'_, $ty, $inner> ; |$dispatcher| {
            $closure
        });
    };
}

impl<D, H: OutputHandler<D>> DelegateDispatchBase<wl_output::WlOutput>
    for OutputDispatch<'_, D, H>
{
    type UserData = OutputData;
}

impl<D, H> DelegateDispatch<wl_output::WlOutput, D> for OutputDispatch<'_, D, H>
where
    H: OutputHandler<D>,
    D: Dispatch<wl_output::WlOutput, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        data: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
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

                inner.pending_info.location = (x, y);
                inner.pending_info.physical_size = (physical_width, physical_height);
                inner.pending_info.subpixel = match subpixel {
                    WEnum::Value(subpixel) => subpixel,
                    WEnum::Unknown(_) => todo!("Warn about invalid subpixel value"),
                };
                inner.pending_info.make = make;
                inner.pending_info.model = model;
                inner.pending_info.transform = match transform {
                    WEnum::Value(subpixel) => subpixel,
                    WEnum::Unknown(_) => todo!("Warn about invalid transform value"),
                };
                inner.pending_wl = true;
            }

            wl_output::Event::Mode { flags, width, height, refresh } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                if let Some((index, _)) =
                    inner.pending_info.modes.iter().enumerate().find(|(_, mode)| {
                        mode.dimensions == (width, height) && mode.refresh_rate == refresh
                    })
                {
                    // We found a match, remove the old mode.
                    inner.pending_info.modes.remove(index);
                }

                let flags = match flags {
                    WEnum::Value(flags) => flags,
                    WEnum::Unknown(_) => panic!("Invalid flags"),
                };

                let current = flags.contains(wl_output::Mode::Current);
                let preferred = flags.contains(wl_output::Mode::Preferred);

                // Now create the new mode.
                inner.pending_info.modes.push(Mode {
                    dimensions: (width, height),
                    refresh_rate: refresh,
                    current,
                    preferred,
                });

                let index = inner.pending_info.modes.len() - 1;

                // Any mode that isn't current is deprecated, let's deprecate any existing modes that may be
                // marked as current.
                //
                // If a new mode is advertised as preferred, then mark the existing preferred mode as not.
                inner.pending_info.modes.iter_mut().enumerate().for_each(|(mode_index, mode)| {
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

                inner.pending_wl = true;
            }

            wl_output::Event::Scale { factor } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                inner.pending_info.scale_factor = factor;
                inner.pending_wl = true;
            }

            wl_output::Event::Name { name } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                inner.pending_info.name = Some(name);
                inner.pending_wl = true;
            }

            wl_output::Event::Description { description } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                inner.pending_info.description = Some(description);
                inner.pending_wl = true;
            }

            wl_output::Event::Done => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| &inner.wl_output == output)
                    .expect("Received event for dead output");

                let info = inner.pending_info.clone();
                inner.current_info = Some(info.clone());
                inner.pending_wl = false;

                if inner
                    .xdg_output
                    .as_ref()
                    .map(Proxy::version)
                    .map(|v| v > 3) // version 3 of xdg_output deprecates xdg_output::done
                    .unwrap_or(false)
                {
                    inner.pending_xdg = false;
                }

                // Set the user data
                data.set(info);

                if inner.just_created {
                    inner.just_created = false;
                    self.1.new_output(conn, qh, self.0, output.clone());
                } else {
                    self.1.update_output(conn, qh, self.0, output.clone());
                }
            }

            _ => unreachable!(),
        }
    }
}

impl<D, H: OutputHandler<D>> DelegateDispatchBase<zxdg_output_manager_v1::ZxdgOutputManagerV1>
    for OutputDispatch<'_, D, H>
{
    type UserData = ();
}

impl<D, H> DelegateDispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, D>
    for OutputDispatch<'_, D, H>
where
    H: OutputHandler<D>,
    D: Dispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        _: &zxdg_output_manager_v1::ZxdgOutputManagerV1,
        _: zxdg_output_manager_v1::Event,
        _: &Self::UserData,
        _: &mut ConnectionHandle,
        _: &QueueHandle<D>,
    ) {
        unreachable!("zxdg_output_manager_v1 has no events")
    }
}

impl<D, H: OutputHandler<D>> DelegateDispatchBase<zxdg_output_v1::ZxdgOutputV1>
    for OutputDispatch<'_, D, H>
{
    type UserData = OutputData;
}

impl<D, H> DelegateDispatch<zxdg_output_v1::ZxdgOutputV1, D> for OutputDispatch<'_, D, H>
where
    H: OutputHandler<D>,
    D: Dispatch<zxdg_output_v1::ZxdgOutputV1, UserData = Self::UserData>,
{
    fn event(
        &mut self,
        output: &zxdg_output_v1::ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        data: &Self::UserData,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
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

                inner.pending_info.name = Some(name);
                inner.pending_xdg = true;
            }

            zxdg_output_v1::Event::Description { description } => {
                let inner = self
                    .0
                    .outputs
                    .iter_mut()
                    .find(|inner| inner.xdg_output.as_ref() == Some(output))
                    .expect("Received event for dead output");

                inner.pending_info.description = Some(description);
                inner.pending_xdg = true;
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

                    let info = inner.pending_info.clone();
                    inner.current_info = Some(info.clone());
                    inner.pending_xdg = false;

                    // Set the user data
                    data.set(info);

                    let pending_wl = inner.pending_wl;
                    let just_created = inner.just_created;
                    let output = inner.wl_output.clone();

                    if just_created {
                        inner.just_created = false;
                    }

                    if !pending_wl {
                        if just_created {
                            self.1.new_output(conn, qh, self.0, output);
                        } else {
                            self.1.update_output(conn, qh, self.0, output);
                        }
                    }
                }
            }

            _ => unreachable!(),
        }
    }
}

impl<D, H> RegistryHandler<D> for OutputDispatch<'_, D, H>
where
    H: OutputHandler<D>,
    D: Dispatch<wl_output::WlOutput, UserData = OutputData>
        + Dispatch<zxdg_output_v1::ZxdgOutputV1, UserData = OutputData>
        + Dispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, UserData = ()>
        + 'static,
{
    fn new_global(
        &mut self,
        conn: &mut ConnectionHandle,
        qh: &QueueHandle<D>,
        name: u32,
        interface: &str,
        version: u32,
        handle: &mut RegistryHandle,
    ) {
        match interface {
            "wl_output" => {
                let wl_output = handle
                    .bind_cached::<wl_output::WlOutput, _, _, _>(conn, qh, name, || {
                        (u32::min(version, 4), OutputData::new())
                    })
                    .expect("Failed to bind global");

                let version = wl_output.version();
                let inner = OutputInner {
                    name,
                    wl_output: wl_output.clone(),
                    xdg_output: None,
                    just_created: true,
                    // wl_output::done was added in version 2.
                    // If we have an output at version 1, assume the data was already sent.
                    current_info: if version > 1 { None } else { Some(OutputInfo::new(name)) },

                    pending_info: OutputInfo::new(name),
                    pending_wl: true,
                    pending_xdg: self.0.xdg.is_some(),
                };

                self.0.outputs.push(inner);

                if self.0.xdg.is_some() {
                    let xdg = self.0.xdg.as_ref().unwrap();

                    let data = wl_output.data::<OutputData>().unwrap().clone();

                    let xdg_output = xdg.get_xdg_output(conn, &wl_output, qh, data).unwrap();
                    self.0.outputs.last_mut().unwrap().xdg_output = Some(xdg_output);
                }
            }

            "zxdg_output_manager_v1" => {
                let global = handle
                    .bind_once::<zxdg_output_manager_v1::ZxdgOutputManagerV1, _, _>(
                        conn,
                        qh,
                        name,
                        u32::min(version, 3),
                        (),
                    )
                    .expect("Failed to bind global");

                self.0.xdg = Some(global);

                let xdg = self.0.xdg.as_ref().unwrap();

                // Because the order in which globals are advertised is undefined, we need to get the extension of any
                // wl_output we have already gotten.
                self.0.outputs.iter_mut().for_each(|output| {
                    let data = output.wl_output.data::<OutputData>().unwrap().clone();

                    let xdg_output = xdg.get_xdg_output(conn, &output.wl_output, qh, data).unwrap();
                    output.xdg_output = Some(xdg_output);
                });
            }

            _ => (),
        }
    }

    fn remove_global(&mut self, conn: &mut ConnectionHandle, qh: &QueueHandle<D>, name: u32) {
        let mut destroyed = vec![];

        self.0.outputs.retain(|inner| {
            let destroy = inner.name != name;

            if destroy {
                if let Some(xdg_output) = &inner.xdg_output {
                    xdg_output.destroy(conn);
                }

                inner.wl_output.release(conn);

                destroyed.push(inner.wl_output.clone());
            }

            destroy
        });

        for output in destroyed {
            self.1.output_destroyed(conn, qh, self.0, output);
        }
    }
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

    current_info: Option<OutputInfo>,
    pending_info: OutputInfo,
    pending_wl: bool,
    pending_xdg: bool,
}
