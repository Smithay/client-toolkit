use crate::{
    dmabuf::dev_t,
    error::GlobalError,
    globals::GlobalData,
    registry::GlobalProxy,
};
use std::{sync::Mutex, time::Duration};
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_output, wl_shm},
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1, ext_image_capture_source_v1,
    ext_output_image_capture_source_manager_v1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_cursor_session_v1, ext_image_copy_capture_frame_v1,
    ext_image_copy_capture_manager_v1, ext_image_copy_capture_session_v1,
};

#[derive(Debug, Default, Clone)]
pub struct BufferConstraints {
    pub size: (u32, u32),
    pub shm_formats: Vec<WEnum<wl_shm::Format>>,
    pub dmabuf_device: Option<dev_t>,
    pub dmabuf_formats: Vec<(u32, Vec<u64>)>,
}

pub trait ImageCopySessionDataExt {
    fn image_copy_session_data(&self) -> &ImageCopySessionData;
}

#[derive(Default, Debug)]
pub struct ImageCopySessionData {
    constraints: Mutex<BufferConstraints>,
}

impl ImageCopySessionDataExt for ImageCopySessionData {
    fn image_copy_session_data(&self) -> &ImageCopySessionData {
        self
    }
}

#[derive(Clone, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub transform: WEnum<wl_output::Transform>,
    pub damage: Vec<Rect>,
    // TODO: Better type for this?
    pub presentation_time: Option<Duration>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            transform: WEnum::Value(wl_output::Transform::Normal),
            damage: Vec::new(),
            presentation_time: None,
        }
    }
}

#[derive(Default, Debug)]
pub struct ImageCopyFrameData {
    frame: Mutex<Frame>,
}

pub trait ImageCopyFrameDataExt {
    fn image_copy_frame_data(&self) -> &ImageCopyFrameData;
}

impl ImageCopyFrameDataExt for ImageCopyFrameData {
    fn image_copy_frame_data(&self) -> &ImageCopyFrameData {
        self
    }
}

pub trait ImageCopyCaptureHandler: Sized {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState;

    fn buffer_constraints(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
        constraints: BufferConstraints,
    );

    fn stopped(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
    );

    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        image_copy_frame: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        frame: Frame,
    );

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        image_copy_frame: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        reason: WEnum<ext_image_copy_capture_frame_v1::FailureReason>,
    );

    fn cursor_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
    ) {
    }

    fn cursor_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
    ) {
    }

    fn cursor_position(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
        _x: i32,
        _y: i32,
    ) {
    }

    fn cursor_hotspot(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
        _x: i32,
        _y: i32,
    ) {
    }
}

#[derive(Debug)]
pub struct ImageCopyCaptureState {
    foreign_toplevel_source_manager: GlobalProxy<ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1>,
    output_source_manager: GlobalProxy<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1>,
    capture_manager: GlobalProxy<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1>,
}

impl ImageCopyCaptureState {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1, GlobalData>,
        D: Dispatch<
                ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
                GlobalData>,
        D: Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, GlobalData>,
        D: 'static,
    {
        let foreign_toplevel_source_manager =
            GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        let output_source_manager = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        let capture_manager = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { foreign_toplevel_source_manager, output_source_manager, capture_manager }
    }

    // TODO global accessors?
}

impl<D> Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, GlobalData, D>
    for ImageCopyCaptureState
where
    D: Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, GlobalData>,
{
    fn event(
        _: &mut D,
        _: &ext_image_capture_source_v1::ExtImageCaptureSourceV1,
        _: ext_image_capture_source_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<
        ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        GlobalData,
        D,
    > for ImageCopyCaptureState
where
    D: Dispatch<
        ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        GlobalData,
    >,
{
    fn event(
        _: &mut D,
        _: &ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        _: ext_output_image_capture_source_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1, GlobalData, D> for ImageCopyCaptureState
where
    D: Dispatch<ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1, GlobalData>
{
    fn event(
        _: &mut D,
        _: &ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
        _: ext_foreign_toplevel_image_capture_source_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, GlobalData, D>
    for ImageCopyCaptureState
where
    D: Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, GlobalData>,
{
    fn event(
        _: &mut D,
        _: &ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
        _: ext_image_copy_capture_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D, U> Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, U, D>
    for ImageCopyCaptureState
where
    D: Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, U>
        + ImageCopyCaptureHandler,
    U: ImageCopySessionDataExt,
{
    fn event(
        state: &mut D,
        proxy: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let constraints = &udata.image_copy_session_data().constraints;
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                constraints.lock().unwrap().size = (width, height);
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat { format } => {
                constraints.lock().unwrap().shm_formats.push(format);
            }
            ext_image_copy_capture_session_v1::Event::DmabufDevice { device } => {
                let device = dev_t::from_ne_bytes(device.try_into().unwrap());
                constraints.lock().unwrap().dmabuf_device = Some(device);
            }
            ext_image_copy_capture_session_v1::Event::DmabufFormat { format, modifiers } => {
                let modifiers = modifiers
                    .chunks_exact(8)
                    .map(|x| u64::from_ne_bytes(x.try_into().unwrap()))
                    .collect();
                constraints.lock().unwrap().dmabuf_formats.push((format, modifiers));
            }
            ext_image_copy_capture_session_v1::Event::Done => {
                let constraints = constraints.lock().unwrap().clone();
                state.buffer_constraints(conn, qh, proxy, constraints);
            }
            ext_image_copy_capture_session_v1::Event::Stopped => {
                state.stopped(conn, qh, proxy);
                proxy.destroy();
            }
            _ => unreachable!(),
        }
    }
}

impl<D, U> Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, U, D>
    for ImageCopyCaptureState
where
    D: Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, U>
        + ImageCopyCaptureHandler,
    U: ImageCopyFrameDataExt,
{
    fn event(
        state: &mut D,
        proxy: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let frame = &udata.image_copy_frame_data().frame;
        match event {
            ext_image_copy_capture_frame_v1::Event::Transform { transform } => {
                frame.lock().unwrap().transform = transform;
            }
            ext_image_copy_capture_frame_v1::Event::Damage { x, y, width, height } => {
                frame.lock().unwrap().damage.push(Rect { x, y, width, height });
            }
            ext_image_copy_capture_frame_v1::Event::PresentationTime {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let secs = (u64::from(tv_sec_hi) << 32) + u64::from(tv_sec_lo);
                let duration = Duration::new(secs, tv_nsec);
                frame.lock().unwrap().presentation_time = Some(duration);
            }
            ext_image_copy_capture_frame_v1::Event::Ready => {
                let frame = frame.lock().unwrap().clone();
                state.ready(conn, qh, proxy, frame);
            }
            ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
                state.failed(conn, qh, proxy, reason);
                proxy.destroy();
            }
            _ => unreachable!(),
        }
    }
}

// TODO
impl<D, U>
    Dispatch<ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1, U, D>
    for ImageCopyCaptureState
where
    D: Dispatch<ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1, U>
        + ImageCopyCaptureHandler,
{
    fn event(
        state: &mut D,
        proxy: &ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
        event: ext_image_copy_capture_cursor_session_v1::Event,
        _: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_image_copy_capture_cursor_session_v1::Event::Enter => {
                state.cursor_enter(conn, qh, proxy);
            }
            ext_image_copy_capture_cursor_session_v1::Event::Leave => {
                state.cursor_leave(conn, qh, proxy);
            }
            ext_image_copy_capture_cursor_session_v1::Event::Position { x, y } => {
                state.cursor_position(conn, qh, proxy, x, y);
            }
            ext_image_copy_capture_cursor_session_v1::Event::Hotspot { x, y } => {
                state.cursor_hotspot(conn, qh, proxy, x, y);
            }
            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_image_copy_capture {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::delegate_image_copy_capture($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty,
            session: $crate::image_copy_capture::ImageCopySessionData, frame: $crate::image_copy_capture::ImageCopyFrameData, cursor_session: $crate::globals::GlobalData);
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, session: [$($session_data:ty),* $(,)?], frame: [$($frame_data:ty),* $(,)?], cursor_session: [$($cursor_session_data:ty),* $(,)?]) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::image_capture_source::v1::client::ext_image_capture_source_v1::ExtImageCaptureSourceV1: $crate::globals::GlobalData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1: $crate::globals::GlobalData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::image_capture_source::v1::client::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1: $crate::globals::GlobalData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1: $crate::globals::GlobalData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $(
            $crate::reexports::protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1: $session_data
            ),*
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $(
            $crate::reexports::protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1: $frame_data
            ),*
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $(
            $crate::reexports::protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1: $cursor_session_data
            ),*
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
    }
}
