use std::time::Duration;

use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::{
    image_capture_source::v1::client::{
        ext_foreign_toplevel_image_capture_source_manager_v1::{
            self, ExtForeignToplevelImageCaptureSourceManagerV1,
        },
        ext_image_capture_source_v1::{self, ExtImageCaptureSourceV1},
        ext_output_image_capture_source_manager_v1::{self, ExtOutputImageCaptureSourceManagerV1},
    },
    image_copy_capture::v1::client::{
        ext_image_copy_capture_cursor_session_v1::{self, ExtImageCopyCaptureCursorSessionV1},
        ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
        ext_image_copy_capture_manager_v1::{self, ExtImageCopyCaptureManagerV1},
        ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
    },
};

use super::{
    CaptureCursorSession, CaptureFrame, CaptureSession, Rect, ImageCopyCaptureCursorSessionDataExt,
    ImageCopyCaptureFrameDataExt, ImageCopyCaptureHandler, ImageCopyCaptureSessionDataExt, ImageCopyCaptureState,
};
use crate::globals::GlobalData;

impl<D> Dispatch<ExtImageCopyCaptureManagerV1, GlobalData, D> for ImageCopyCaptureState
where
    D: Dispatch<ExtImageCopyCaptureManagerV1, GlobalData> + ImageCopyCaptureHandler,
{
    fn event(
        _: &mut D,
        _: &ExtImageCopyCaptureManagerV1,
        _: ext_image_copy_capture_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D, U> Dispatch<ExtImageCopyCaptureSessionV1, U, D> for ImageCopyCaptureState
where
    D: Dispatch<ExtImageCopyCaptureSessionV1, U> + ImageCopyCaptureHandler,
    U: ImageCopyCaptureSessionDataExt,
{
    fn event(
        app_data: &mut D,
        session: &ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let formats = &udata.image_copy_capture_session_data().formats;
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                formats.lock().unwrap().buffer_size = (width, height);
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat { format } => {
                if let WEnum::Value(value) = format {
                    formats.lock().unwrap().shm_formats.push(value);
                }
            }
            ext_image_copy_capture_session_v1::Event::DmabufDevice { device } => {
                let device = libc::dev_t::from_ne_bytes(device.try_into().unwrap());
                formats.lock().unwrap().dmabuf_device = Some(device);
            }
            ext_image_copy_capture_session_v1::Event::DmabufFormat { format, modifiers } => {
                let modifiers = modifiers
                    .chunks_exact(8)
                    .map(|x| u64::from_ne_bytes(x.try_into().unwrap()))
                    .collect();
                formats.lock().unwrap().dmabuf_formats.push((format, modifiers));
            }
            ext_image_copy_capture_session_v1::Event::Done => {
                if let Some(session) = udata
                    .image_copy_capture_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureSession)
                {
                    app_data.init_done(conn, qh, &session, &formats.lock().unwrap());
                }
            }
            ext_image_copy_capture_session_v1::Event::Stopped => {
                if let Some(session) = udata
                    .image_copy_capture_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureSession)
                {
                    app_data.stopped(conn, qh, &session);
                }
                session.destroy();
            }
            _ => unreachable!(),
        }
    }
}

impl<D, U> Dispatch<ExtImageCopyCaptureFrameV1, U, D> for ImageCopyCaptureState
where
    D: Dispatch<ExtImageCopyCaptureFrameV1, U> + ImageCopyCaptureHandler,
    U: ImageCopyCaptureFrameDataExt,
{
    fn event(
        app_data: &mut D,
        capture_frame: &ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let frame = &udata.image_copy_capture_frame_data().frame;
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
                frame.lock().unwrap().present_time = Some(duration);
            }
            ext_image_copy_capture_frame_v1::Event::Ready => {
                let frame = frame.lock().unwrap().clone();
                app_data.ready(
                    conn,
                    qh,
                    &CaptureFrame { frame: capture_frame.clone() },
                    frame,
                );
                capture_frame.destroy();
            }
            ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
                app_data.failed(
                    conn,
                    qh,
                    &CaptureFrame { frame: capture_frame.clone() },
                    reason,
                );
                capture_frame.destroy();
            }
            _ => unreachable!(),
        }
    }
}

impl<D, U> Dispatch<ExtImageCopyCaptureCursorSessionV1, U, D> for ImageCopyCaptureState
where
    D: Dispatch<ExtImageCopyCaptureCursorSessionV1, U> + ImageCopyCaptureHandler,
    U: ImageCopyCaptureCursorSessionDataExt,
{
    fn event(
        app_data: &mut D,
        _cursor_session: &ExtImageCopyCaptureCursorSessionV1,
        event: ext_image_copy_capture_cursor_session_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_image_copy_capture_cursor_session_v1::Event::Enter => {
                if let Some(session) = udata
                    .image_copy_capture_cursor_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureCursorSession)
                {
                    app_data.cursor_enter(conn, qh, &session);
                }
            }
            ext_image_copy_capture_cursor_session_v1::Event::Leave => {
                if let Some(session) = udata
                    .image_copy_capture_cursor_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureCursorSession)
                {
                    app_data.cursor_leave(conn, qh, &session);
                }
            }
            ext_image_copy_capture_cursor_session_v1::Event::Position { x, y } => {
                if let Some(session) = udata
                    .image_copy_capture_cursor_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureCursorSession)
                {
                    app_data.cursor_position(conn, qh, &session, x, y);
                }
            }
            ext_image_copy_capture_cursor_session_v1::Event::Hotspot { x, y } => {
                if let Some(session) = udata
                    .image_copy_capture_cursor_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureCursorSession)
                {
                    app_data.cursor_hotspot(conn, qh, &session, x, y);
                }
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ExtImageCaptureSourceV1, GlobalData, D> for ImageCopyCaptureState
where
    D: Dispatch<ExtImageCaptureSourceV1, GlobalData> + ImageCopyCaptureHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &ExtImageCaptureSourceV1,
        _event: ext_image_capture_source_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<ExtOutputImageCaptureSourceManagerV1, GlobalData, D> for ImageCopyCaptureState
where
    D: Dispatch<ExtOutputImageCaptureSourceManagerV1, GlobalData> + ImageCopyCaptureHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &ExtOutputImageCaptureSourceManagerV1,
        _event: ext_output_image_capture_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D> Dispatch<ExtForeignToplevelImageCaptureSourceManagerV1, GlobalData, D> for ImageCopyCaptureState
where
    D: Dispatch<ExtForeignToplevelImageCaptureSourceManagerV1, GlobalData> + ImageCopyCaptureHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &ExtForeignToplevelImageCaptureSourceManagerV1,
        _event: ext_foreign_toplevel_image_capture_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

#[macro_export]
macro_rules! delegate_image_copy_capture {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1: $crate::globals::GlobalData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::image_capture_source::v1::client::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1: $crate::globals::GlobalData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::image_capture_source::v1::client::ext_image_capture_source_v1::ExtImageCaptureSourceV1: $crate::globals::GlobalData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1: $crate::globals::GlobalData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!(@<$( $lt $( : $clt $(+ $dlt )* )? ),* SessionData: ($crate::image_copy_capture::ImageCopyCaptureSessionDataExt)> $ty: [
            $crate::reexports::protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1: SessionData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!(@<$( $lt $( : $clt $(+ $dlt )* )? ),* FrameData: ($crate::image_copy_capture::ImageCopyCaptureFrameDataExt)> $ty: [
            $crate::reexports::protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1: FrameData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
        $crate::reexports::client::delegate_dispatch!(@<$( $lt $( : $clt $(+ $dlt )* )? ),* CursorSessionData: ($crate::image_copy_capture::ImageCopyCaptureCursorSessionDataExt)> $ty: [
            $crate::reexports::protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1: CursorSessionData
        ] => $crate::image_copy_capture::ImageCopyCaptureState);
    };
}
