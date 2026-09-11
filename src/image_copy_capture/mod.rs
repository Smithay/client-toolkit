use std::{
    sync::{Arc, Mutex, OnceLock, Weak},
    time::Duration,
};

use wayland_client::{
    globals::GlobalList,
    protocol::{wl_buffer::WlBuffer, wl_output::Transform, wl_pointer::WlPointer, wl_shm},
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::ext::{
    image_capture_source::v1::client::{
        ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
        ext_image_capture_source_v1::ExtImageCaptureSourceV1,
        ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
    },
    image_copy_capture::v1::client::{
        ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
        ext_image_copy_capture_frame_v1::{ExtImageCopyCaptureFrameV1, FailureReason},
        ext_image_copy_capture_manager_v1::{
            ExtImageCopyCaptureManagerV1, Options as CaptureOptions,
        },
        ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
    },
};

use crate::globals::GlobalData;

mod capture_source;
pub use capture_source::{CaptureSource, CaptureSourceError, CaptureSourceKind};
mod dispatch;

#[derive(Clone, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub transform: WEnum<Transform>,
    pub damage: Vec<Rect>,
    // XXX monotonic? Is this used elsewhere in wayland?
    pub present_time: Option<Duration>,
}

impl Default for Frame {
    fn default() -> Self {
        Self { transform: WEnum::Value(Transform::Normal), damage: Vec::new(), present_time: None }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Formats {
    pub buffer_size: (u32, u32),
    pub shm_formats: Vec<wl_shm::Format>,
    pub dmabuf_device: Option<libc::dev_t>,
    pub dmabuf_formats: Vec<(u32, Vec<u64>)>,
}

#[derive(Debug)]
struct CapturerInner {
    image_copy_capture_manager: Option<ExtImageCopyCaptureManagerV1>,
    output_source_manager: Option<ExtOutputImageCaptureSourceManagerV1>,
    foreign_toplevel_source_manager: Option<ExtForeignToplevelImageCaptureSourceManagerV1>,
}

impl Drop for CapturerInner {
    fn drop(&mut self) {
        if let Some(manager) = &self.image_copy_capture_manager {
            manager.destroy();
        }
        if let Some(manager) = &self.output_source_manager {
            manager.destroy();
        }
        if let Some(manager) = &self.foreign_toplevel_source_manager {
            manager.destroy();
        }
    }
}

#[derive(Clone, Debug)]
pub struct Capturer(Arc<CapturerInner>);

impl Capturer {
    // TODO check supported capture types

    pub fn create_session<D, U>(
        &self,
        source: &CaptureSource,
        options: CaptureOptions,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<CaptureSession, CaptureSourceError>
    where
        D: 'static,
        D: Dispatch<ExtImageCaptureSourceV1, GlobalData>,
        D: Dispatch<ExtImageCopyCaptureSessionV1, U>,
        U: ImageCopyCaptureSessionDataExt + Send + Sync + 'static,
    {
        let source = source.create_source(self, qh)?;
        Ok(CaptureSession(Arc::new_cyclic(|weak_session| {
            udata.image_copy_capture_session_data().session.set(weak_session.clone()).unwrap();
            CaptureSessionInner {
                session: self
                    .0
                    .image_copy_capture_manager
                    .as_ref()
                    .expect("ext capture source with no image capture copy manager")
                    .create_session(&source.0, options, qh, udata),
            }
        })))
    }

    pub fn create_cursor_session<D, U>(
        &self,
        source: &CaptureSource,
        pointer: &WlPointer,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<CaptureCursorSession, CaptureSourceError>
    where
        D: 'static,
        D: Dispatch<ExtImageCaptureSourceV1, GlobalData>,
        D: Dispatch<ExtImageCopyCaptureCursorSessionV1, U>,
        U: ImageCopyCaptureCursorSessionDataExt + Send + Sync + 'static,
    {
        let source = source.create_source(self, qh)?;
        Ok(CaptureCursorSession(Arc::new_cyclic(|weak_session| {
            udata.image_copy_capture_cursor_session_data().session.set(weak_session.clone()).unwrap();
            CaptureCursorSessionInner {
                session: self
                    .0
                    .image_copy_capture_manager
                    .as_ref()
                    .expect("ext capture source with no image capture copy manager")
                    .create_pointer_cursor_session(&source.0, pointer, qh, udata),
            }
        })))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct CaptureSessionInner {
    session: ExtImageCopyCaptureSessionV1,
}

impl Drop for CaptureSessionInner {
    fn drop(&mut self) {
        self.session.destroy();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CaptureSession(Arc<CaptureSessionInner>);

impl CaptureSession {
    pub fn capture<D, U>(
        &self,
        buffer: &WlBuffer,
        buffer_damage: &[Rect],
        qh: &QueueHandle<D>,
        udata: U,
    ) -> CaptureFrame
    where
        D: 'static,
        D: Dispatch<ExtImageCopyCaptureFrameV1, U>,
        U: ImageCopyCaptureFrameDataExt + Send + Sync + 'static,
    {
        udata.image_copy_capture_frame_data().session.set(Arc::downgrade(&self.0)).unwrap();
        let frame = self.0.session.create_frame(qh, udata);
        frame.attach_buffer(buffer);
        for Rect { x, y, width, height } in buffer_damage {
            frame.damage_buffer(*x, *y, *width, *height);
        }
        frame.capture();
        CaptureFrame { frame }
    }

    pub fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
        self.0.session.data()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CaptureFrame {
    frame: ExtImageCopyCaptureFrameV1,
}

impl CaptureFrame {
    pub fn session<U: ImageCopyCaptureFrameDataExt + Send + Sync + 'static>(
        &self,
    ) -> Option<CaptureSession> {
        Some(CaptureSession(
            self.data::<U>()?.image_copy_capture_frame_data().session.get().unwrap().upgrade()?,
        ))
    }

    pub fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
        self.frame.data()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct CaptureCursorSessionInner {
    session: ExtImageCopyCaptureCursorSessionV1,
}

impl Drop for CaptureCursorSessionInner {
    fn drop(&mut self) {
        self.session.destroy();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CaptureCursorSession(Arc<CaptureCursorSessionInner>);

impl CaptureCursorSession {
    pub fn capture_session<D, U>(
        &self,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> Result<CaptureSession, CaptureSourceError>
    where
        D: 'static,
        D: Dispatch<ExtImageCaptureSourceV1, GlobalData>,
        D: Dispatch<ExtImageCopyCaptureSessionV1, U>,
        U: ImageCopyCaptureSessionDataExt + Send + Sync + 'static,
    {
        Ok(CaptureSession(Arc::new_cyclic(|weak_session| {
            udata.image_copy_capture_session_data().session.set(weak_session.clone()).unwrap();
            CaptureSessionInner { session: self.0.session.get_capture_session(qh, udata) }
        })))
    }

    pub fn data<U: Send + Sync + 'static>(&self) -> Option<&U> {
        self.0.session.data()
    }
}

#[derive(Debug)]
pub struct ImageCopyCaptureState {
    capturer: Capturer,
}

impl ImageCopyCaptureState {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: 'static,
        D: Dispatch<ExtImageCopyCaptureManagerV1, GlobalData>,
        D: Dispatch<ExtOutputImageCaptureSourceManagerV1, GlobalData>,
        D: Dispatch<ExtForeignToplevelImageCaptureSourceManagerV1, GlobalData>,
    {
        let image_copy_capture_manager = globals.bind(qh, 1..=1, GlobalData).ok();
        let output_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();
        let foreign_toplevel_source_manager = globals.bind(qh, 1..=1, GlobalData).ok();

        let capturer = Capturer(Arc::new(CapturerInner {
            image_copy_capture_manager,
            output_source_manager,
            foreign_toplevel_source_manager,
        }));

        Self { capturer }
    }

    pub fn capturer(&self) -> &Capturer {
        &self.capturer
    }
}

pub trait ImageCopyCaptureHandler: Sized {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState;

    fn init_done(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &CaptureSession,
        formats: &Formats,
    );

    fn stopped(&mut self, conn: &Connection, qh: &QueueHandle<Self>, session: &CaptureSession);

    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        frame: &CaptureFrame,
        frame: Frame,
    );

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        frame: &CaptureFrame,
        reason: WEnum<FailureReason>,
    );

    fn cursor_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &CaptureCursorSession,
    ) {
    }

    fn cursor_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &CaptureCursorSession,
    ) {
    }

    fn cursor_position(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &CaptureCursorSession,
        _x: i32,
        _y: i32,
    ) {
    }

    fn cursor_hotspot(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &CaptureCursorSession,
        _x: i32,
        _y: i32,
    ) {
    }
}

pub trait ImageCopyCaptureSessionDataExt {
    fn image_copy_capture_session_data(&self) -> &ImageCopyCaptureSessionData;
}

#[derive(Default, Debug)]
pub struct ImageCopyCaptureSessionData {
    formats: Mutex<Formats>,
    session: OnceLock<Weak<CaptureSessionInner>>,
}

impl ImageCopyCaptureSessionDataExt for ImageCopyCaptureSessionData {
    fn image_copy_capture_session_data(&self) -> &ImageCopyCaptureSessionData {
        self
    }
}

#[derive(Default, Debug)]
pub struct ImageCopyCaptureFrameData {
    frame: Mutex<Frame>,
    session: OnceLock<Weak<CaptureSessionInner>>,
}

pub trait ImageCopyCaptureFrameDataExt {
    fn image_copy_capture_frame_data(&self) -> &ImageCopyCaptureFrameData;
}

impl ImageCopyCaptureFrameDataExt for ImageCopyCaptureFrameData {
    fn image_copy_capture_frame_data(&self) -> &ImageCopyCaptureFrameData {
        self
    }
}

#[derive(Default, Debug)]
pub struct ImageCopyCaptureCursorSessionData {
    session: OnceLock<Weak<CaptureCursorSessionInner>>,
}

pub trait ImageCopyCaptureCursorSessionDataExt {
    fn image_copy_capture_cursor_session_data(&self) -> &ImageCopyCaptureCursorSessionData;
}

impl ImageCopyCaptureCursorSessionDataExt for ImageCopyCaptureCursorSessionData {
    fn image_copy_capture_cursor_session_data(&self) -> &ImageCopyCaptureCursorSessionData {
        self
    }
}
