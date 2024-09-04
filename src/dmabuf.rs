use crate::{error::GlobalError, globals::GlobalData, registry::GlobalProxy};
use memmap2::{Mmap, MmapOptions};
use std::{fmt, mem, os::unix::io::BorrowedFd, slice, sync::Mutex};
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_buffer, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1,
    zwp_linux_dmabuf_feedback_v1::{self, TrancheFlags},
    zwp_linux_dmabuf_v1,
};

// Workaround until `libc` updates to FreeBSD 12 ABI
#[cfg(target_os = "freebsd")]
type dev_t = u64;
#[cfg(not(target_os = "freebsd"))]
use libc::dev_t;

/// A preference tranche of dmabuf formats
#[derive(Clone, Debug)]
pub struct DmabufFeedbackTranche {
    /// `dev_t` value for preferred target device. May be scan-out or
    /// renderer device.
    pub device: dev_t,
    /// Flags for tranche
    pub flags: WEnum<TrancheFlags>,
    /// Indices of formats in the format table
    pub formats: Vec<u16>,
}

impl Default for DmabufFeedbackTranche {
    fn default() -> DmabufFeedbackTranche {
        DmabufFeedbackTranche {
            device: 0,
            flags: WEnum::Value(TrancheFlags::empty()),
            formats: Vec::new(),
        }
    }
}

/// A single dmabuf format/modifier pair
// Must have correct representation to be able to mmap format table
#[repr(C)]
#[derive(Copy, Clone)]
pub struct DmabufFormat {
    /// Fourcc format
    pub format: u32,
    _padding: u32,
    /// Modifier, or `DRM_FORMAT_MOD_INVALID` for implict modifier
    pub modifier: u64,
}

impl fmt::Debug for DmabufFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DmabufFormat")
            .field("format", &self.format)
            .field("modifier", &self.modifier)
            .finish()
    }
}

/// Description of supported and preferred dmabuf formats
#[derive(Default)]
pub struct DmabufFeedback {
    format_table: Option<(Mmap, usize)>,
    main_device: dev_t,
    tranches: Vec<DmabufFeedbackTranche>,
}

impl fmt::Debug for DmabufFeedback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DmabufFeedback")
            .field("format_table", &self.format_table())
            .field("main_device", &self.main_device)
            .field("tranches", &self.tranches)
            .finish()
    }
}

impl DmabufFeedback {
    /// Format/modifier pairs
    pub fn format_table(&self) -> &[DmabufFormat] {
        self.format_table.as_ref().map_or(&[], |(mmap, len)| unsafe {
            slice::from_raw_parts(mmap.as_ptr() as *const DmabufFormat, *len)
        })
    }

    /// `dev_t` value for main device. Buffers must be importable from main device.
    pub fn main_device(&self) -> dev_t {
        self.main_device
    }

    /// Tranches in descending order of preference
    pub fn tranches(&self) -> &[DmabufFeedbackTranche] {
        &self.tranches
    }
}

#[doc(hidden)]
#[derive(Debug, Default)]
pub struct DmabufFeedbackData {
    pending: Mutex<DmabufFeedback>,
    pending_tranche: Mutex<DmabufFeedbackTranche>,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct DmaBufferData;

/// A handler for [`zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1`]
#[derive(Debug)]
pub struct DmabufState {
    zwp_linux_dmabuf: GlobalProxy<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1>,
    modifiers: Vec<DmabufFormat>,
}

impl DmabufState {
    /// Bind `zwp_linux_dmabuf_v1` global version 3 or 4, if it exists.
    ///
    /// This does not fail if the global does not exist.
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1, GlobalData> + 'static,
    {
        // Mesa (at least the latest version) also requires version 3 or 4
        let zwp_linux_dmabuf = GlobalProxy::from(globals.bind(qh, 3..=5, GlobalData));
        Self { zwp_linux_dmabuf, modifiers: Vec::new() }
    }

    /// Only populated in version `<4`
    ///
    /// On version `4`, use [`DmabufState::get_surface_feedback`].
    pub fn modifiers(&self) -> &[DmabufFormat] {
        &self.modifiers
    }

    /// Supported protocol version, if any
    pub fn version(&self) -> Option<u32> {
        Some(self.zwp_linux_dmabuf.get().ok()?.version())
    }

    /// Create a params object for constructing a buffer
    ///
    /// Errors if `zwp_linux_dmabuf_v1` does not exist or has unsupported
    /// version. An application can then fallback to using `shm` buffers.
    pub fn create_params<D>(&self, qh: &QueueHandle<D>) -> Result<DmabufParams, GlobalError>
    where
        D: Dispatch<zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, GlobalData> + 'static,
    {
        let zwp_linux_dmabuf = self.zwp_linux_dmabuf.get()?;
        let params = zwp_linux_dmabuf.create_params(qh, GlobalData);
        Ok(DmabufParams { params })
    }

    /// Get default dmabuf feedback. Requires version `4`.
    ///
    /// On version `3`, use [`DmabufState::modifiers`].
    pub fn get_default_feedback<D>(
        &self,
        qh: &QueueHandle<D>,
    ) -> Result<zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1, GlobalError>
    where
        D: Dispatch<zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1, DmabufFeedbackData>
            + 'static,
    {
        let zwp_linux_dmabuf = self.zwp_linux_dmabuf.with_min_version(4)?;
        Ok(zwp_linux_dmabuf.get_default_feedback(qh, DmabufFeedbackData::default()))
    }

    /// Get default dmabuf feedback for given surface. Requires version `4`.
    ///
    /// On version `3`, use [`DmabufState::modifiers`].
    pub fn get_surface_feedback<D>(
        &self,
        surface: &wl_surface::WlSurface,
        qh: &QueueHandle<D>,
    ) -> Result<zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1, GlobalError>
    where
        D: Dispatch<zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1, DmabufFeedbackData>
            + 'static,
    {
        let zwp_linux_dmabuf = self.zwp_linux_dmabuf.with_min_version(4)?;
        Ok(zwp_linux_dmabuf.get_surface_feedback(surface, qh, DmabufFeedbackData::default()))
    }
}

pub trait DmabufHandler: Sized {
    fn dmabuf_state(&mut self) -> &mut DmabufState;

    /// Server has sent dmabuf feedback information. This may be received multiple
    /// times by a `ZwpLinuxDmabufFeedbackV1` object.
    fn dmabuf_feedback(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        proxy: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        feedback: DmabufFeedback,
    );

    /// `wl_buffer` associated with `params` has been created successfully.
    fn created(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        params: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
        buffer: wl_buffer::WlBuffer,
    );

    /// Failed to create `wl_buffer` for `params`.
    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        params: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
    );

    /// Compositor has released a `wl_buffer` created through [`DmabufParams`].
    fn released(&mut self, conn: &Connection, qh: &QueueHandle<Self>, buffer: &wl_buffer::WlBuffer);
}

/// Builder for a dmabuf backed buffer
#[derive(Debug)]
pub struct DmabufParams {
    params: zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
}

impl DmabufParams {
    /// Add a plane
    ///
    /// In version `4`, it is a protocol error if `format`/`modifier` pair wasn't
    /// advertised as supported.
    ///
    /// `modifier` should be the same for all planes. It is a protocol error in version `5` if
    /// they differ.
    pub fn add(&self, fd: BorrowedFd<'_>, plane_idx: u32, offset: u32, stride: u32, modifier: u64) {
        let modifier_hi = (modifier >> 32) as u32;
        let modifier_lo = (modifier & 0xffffffff) as u32;
        self.params.add(fd, plane_idx, offset, stride, modifier_hi, modifier_lo);
    }

    /// Create buffer.
    ///
    /// [`DmabufHandler::created`] or [`DmabufHandler::failed`] will be invoked when the
    /// operation succeeds or fails.
    pub fn create(
        self,
        width: i32,
        height: i32,
        format: u32,
        flags: zwp_linux_buffer_params_v1::Flags,
    ) -> zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1 {
        self.params.create(width, height, format, flags);
        self.params
    }

    /// Create buffer immediately.
    ///
    /// On failure buffer is invalid, and server may raise protocol error or
    /// send [`DmabufHandler::failed`].
    pub fn create_immed<D>(
        self,
        width: i32,
        height: i32,
        format: u32,
        flags: zwp_linux_buffer_params_v1::Flags,
        qh: &QueueHandle<D>,
    ) -> (wl_buffer::WlBuffer, zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1)
    where
        D: Dispatch<wl_buffer::WlBuffer, DmaBufferData> + 'static,
    {
        let buffer = self.params.create_immed(width, height, format, flags, qh, DmaBufferData);
        (buffer, self.params)
    }
}

impl<D> Dispatch<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1, GlobalData, D> for DmabufState
where
    D: Dispatch<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1, GlobalData> + DmabufHandler,
{
    fn event(
        state: &mut D,
        proxy: &zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        event: zwp_linux_dmabuf_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            zwp_linux_dmabuf_v1::Event::Format { format: _ } => {
                // Formats are duplicated by modifier events since version 3.
                // Ignore this event, like Mesa does.
            }
            zwp_linux_dmabuf_v1::Event::Modifier { format, modifier_hi, modifier_lo } => {
                if proxy.version() < 4 {
                    let modifier = (u64::from(modifier_hi) << 32) | u64::from(modifier_lo);
                    state.dmabuf_state().modifiers.push(DmabufFormat {
                        format,
                        _padding: 0,
                        modifier,
                    });
                }
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1, DmabufFeedbackData, D>
    for DmabufState
where
    D: Dispatch<zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1, DmabufFeedbackData>
        + DmabufHandler,
{
    fn event(
        state: &mut D,
        proxy: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        event: zwp_linux_dmabuf_feedback_v1::Event,
        data: &DmabufFeedbackData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zwp_linux_dmabuf_feedback_v1::Event::Done => {
                let feedback = mem::take(&mut *data.pending.lock().unwrap());
                state.dmabuf_feedback(conn, qh, proxy, feedback);
            }
            zwp_linux_dmabuf_feedback_v1::Event::FormatTable { fd, size } => {
                let size = size as usize;
                let mmap = unsafe {
                    MmapOptions::new().map_copy_read_only(&fd).expect("Failed to map format table")
                };
                assert!(mmap.len() >= size);
                let entry_size = mem::size_of::<DmabufFormat>();
                assert!((size % entry_size) == 0);
                let len = size / entry_size;
                data.pending.lock().unwrap().format_table = Some((mmap, len));
            }
            zwp_linux_dmabuf_feedback_v1::Event::MainDevice { device } => {
                let device = dev_t::from_ne_bytes(device.try_into().unwrap());
                data.pending.lock().unwrap().main_device = device;
            }
            zwp_linux_dmabuf_feedback_v1::Event::TrancheDone => {
                let tranche = mem::take(&mut *data.pending_tranche.lock().unwrap());
                data.pending.lock().unwrap().tranches.push(tranche);
            }
            zwp_linux_dmabuf_feedback_v1::Event::TrancheTargetDevice { device } => {
                let device = dev_t::from_ne_bytes(device.try_into().unwrap());
                data.pending_tranche.lock().unwrap().device = device;
            }
            zwp_linux_dmabuf_feedback_v1::Event::TrancheFormats { indices } => {
                assert!((indices.len() % 2) == 0);
                let indices =
                    indices.chunks(2).map(|i| u16::from_ne_bytes([i[0], i[1]])).collect::<Vec<_>>();
                data.pending_tranche.lock().unwrap().formats = indices;
            }
            zwp_linux_dmabuf_feedback_v1::Event::TrancheFlags { flags } => {
                data.pending_tranche.lock().unwrap().flags = flags;
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, GlobalData, D> for DmabufState
where
    D: Dispatch<zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, GlobalData>
        + Dispatch<wl_buffer::WlBuffer, DmaBufferData>
        + DmabufHandler
        + 'static,
{
    fn event(
        state: &mut D,
        proxy: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
        event: zwp_linux_buffer_params_v1::Event,
        _: &GlobalData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            zwp_linux_buffer_params_v1::Event::Created { buffer } => {
                state.created(conn, qh, proxy, buffer);
            }
            zwp_linux_buffer_params_v1::Event::Failed => {
                state.failed(conn, qh, proxy);
            }
            _ => unreachable!(),
        }
    }

    wayland_client::event_created_child!(D, zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, [
        zwp_linux_buffer_params_v1::EVT_CREATED_OPCODE => (wl_buffer::WlBuffer, DmaBufferData)
    ]);
}

impl<D> Dispatch<wl_buffer::WlBuffer, DmaBufferData, D> for DmabufState
where
    D: Dispatch<wl_buffer::WlBuffer, DmaBufferData> + DmabufHandler,
{
    fn event(
        state: &mut D,
        proxy: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _: &DmaBufferData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wl_buffer::Event::Release => state.released(conn, qh, proxy),
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_dmabuf {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1: $crate::globals::GlobalData
            ] => $crate::dmabuf::DmabufState
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::wp::linux_dmabuf::zv1::client::zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1: $crate::globals::GlobalData
            ] => $crate::dmabuf::DmabufState
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1: $crate::dmabuf::DmabufFeedbackData
            ] => $crate::dmabuf::DmabufState
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::client::protocol::wl_buffer::WlBuffer: $crate::dmabuf::DmaBufferData
            ] => $crate::dmabuf::DmabufState
        );
    };
}
