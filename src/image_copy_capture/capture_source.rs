use std::{error::Error, fmt};

use wayland_client::{protocol::wl_output::WlOutput, Dispatch, QueueHandle};
use wayland_protocols::ext::{
    foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    image_capture_source::v1::client::ext_image_capture_source_v1::ExtImageCaptureSourceV1,
};

use super::Capturer;
use crate::globals::GlobalData;

#[derive(Debug)]
pub struct CaptureSourceError(CaptureSourceKind);

impl fmt::Display for CaptureSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "capture kind '{:?}' unsupported by compositor", self.0)
    }
}

impl Error for CaptureSourceError {}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CaptureSourceKind {
    Output,
    Toplevel,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum CaptureSource {
    Output(WlOutput),
    Toplevel(ExtForeignToplevelHandleV1),
}

impl CaptureSource {
    pub fn kind(&self) -> CaptureSourceKind {
        match self {
            Self::Output(_) => CaptureSourceKind::Output,
            Self::Toplevel(_) => CaptureSourceKind::Toplevel,
        }
    }

    pub(crate) fn create_source<D>(
        &self,
        capturer: &Capturer,
        qh: &QueueHandle<D>,
    ) -> Result<WlCaptureSource, CaptureSourceError>
    where
        D: 'static,
        D: Dispatch<ExtImageCaptureSourceV1, GlobalData>,
    {
        match self {
            CaptureSource::Output(output) => {
                if let Some(manager) = &capturer.0.output_source_manager {
                    return Ok(WlCaptureSource(manager.create_source(output, qh, GlobalData)));
                }
            }
            CaptureSource::Toplevel(toplevel) => {
                if let Some(manager) = &capturer.0.foreign_toplevel_source_manager {
                    return Ok(WlCaptureSource(manager.create_source(toplevel, qh, GlobalData)));
                }
            }
        }
        Err(CaptureSourceError(self.kind()))
    }
}

// TODO name?
pub(crate) struct WlCaptureSource(pub ExtImageCaptureSourceV1);

impl Drop for WlCaptureSource {
    fn drop(&mut self) {
        self.0.destroy();
    }
}
