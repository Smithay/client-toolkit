use smithay_client_toolkit::{
    delegate_image_copy_capture,
    image_copy_capture::{
        BufferConstraints, Frame, ImageCopyCaptureHandler, ImageCopyCaptureState,
        ImageCopyFrameData, ImageCopyFrameDataExt, ImageCopySessionData, ImageCopySessionDataExt,
    },
};
use wayland_client::{Connection, QueueHandle, WEnum};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_cursor_session_v1, ext_image_copy_capture_frame_v1,
    ext_image_copy_capture_manager_v1, ext_image_copy_capture_session_v1,
};

struct State {
    image_copy_capture_state: ImageCopyCaptureState,
}

struct SessionData {
    session_data: ImageCopySessionData,
}

impl ImageCopySessionDataExt for SessionData {
    fn image_copy_session_data(&self) -> &ImageCopySessionData {
        &self.session_data
    }
}

struct FrameData {
    frame_data: ImageCopyFrameData,
}

impl ImageCopyFrameDataExt for FrameData {
    fn image_copy_frame_data(&self) -> &ImageCopyFrameData {
        &self.frame_data
    }
}

struct CursorSessionData {}

fn main() {}

impl ImageCopyCaptureHandler for State {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState {
        &mut self.image_copy_capture_state
    }

    fn buffer_constraints(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
        constraints: BufferConstraints,
    ) {
    }

    fn stopped(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
    ) {
    }

    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        image_copy_frame: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        frame: Frame,
    ) {
    }

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        image_copy_frame: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        reason: WEnum<ext_image_copy_capture_frame_v1::FailureReason>,
    ) {
    }
}

delegate_image_copy_capture!(State, session: [SessionData], frame: [FrameData], cursor_session: [CursorSessionData]);
