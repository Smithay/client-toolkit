use std::{mem, sync::Mutex};
use wayland_client::{
    globals::GlobalList,
    protocol::{wl_output, wl_surface},
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols::wp::presentation_time::client::{wp_presentation, wp_presentation_feedback};

use crate::{error::GlobalError, globals::GlobalData, registry::GlobalProxy};

#[derive(Debug)]
pub struct PresentTime {
    pub clk_id: u32,
    pub tv_sec: u64,
    pub tv_nsec: u32,
}

#[derive(Debug)]
pub struct PresentationTimeState {
    presentation: GlobalProxy<wp_presentation::WpPresentation>,
    clk_id: Option<u32>,
}

impl PresentationTimeState {
    /// Bind `wp_presentation` global, if it exists
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<wp_presentation::WpPresentation, GlobalData> + 'static,
    {
        let presentation = GlobalProxy::from(globals.bind(qh, 1..=1, GlobalData));
        Self { presentation, clk_id: None }
    }

    /// Request feedback for current submission to surface.
    pub fn feedback<D>(
        &self,
        surface: &wl_surface::WlSurface,
        qh: &QueueHandle<D>,
    ) -> Result<wp_presentation_feedback::WpPresentationFeedback, GlobalError>
    where
        D: Dispatch<wp_presentation_feedback::WpPresentationFeedback, PresentationTimeData>
            + 'static,
    {
        let udata = PresentationTimeData {
            wl_surface: surface.clone(),
            sync_outputs: Mutex::new(Vec::new()),
        };
        Ok(self.presentation.get()?.feedback(surface, qh, udata))
    }
}

pub trait PresentationTimeHandler: Sized {
    fn presentation_time_state(&mut self) -> &mut PresentationTimeState;

    /// Content update displayed to user at indicated time
    #[allow(clippy::too_many_arguments)]
    fn presented(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        feedback: &wp_presentation_feedback::WpPresentationFeedback,
        surface: &wl_surface::WlSurface,
        outputs: Vec<wl_output::WlOutput>,
        time: PresentTime,
        refresh: u32,
        seq: u64,
        flags: WEnum<wp_presentation_feedback::Kind>,
    );

    /// Content update not displayed
    fn discarded(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        feedback: &wp_presentation_feedback::WpPresentationFeedback,
        surface: &wl_surface::WlSurface,
    );
}

#[doc(hidden)]
#[derive(Debug)]
pub struct PresentationTimeData {
    wl_surface: wl_surface::WlSurface,
    sync_outputs: Mutex<Vec<wl_output::WlOutput>>,
}

impl<D> Dispatch<wp_presentation::WpPresentation, GlobalData, D> for PresentationTimeState
where
    D: Dispatch<wp_presentation::WpPresentation, GlobalData> + PresentationTimeHandler,
{
    fn event(
        data: &mut D,
        _presentation: &wp_presentation::WpPresentation,
        event: wp_presentation::Event,
        _: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        match event {
            wp_presentation::Event::ClockId { clk_id } => {
                data.presentation_time_state().clk_id = Some(clk_id);
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<wp_presentation_feedback::WpPresentationFeedback, PresentationTimeData, D>
    for PresentationTimeState
where
    D: Dispatch<wp_presentation_feedback::WpPresentationFeedback, PresentationTimeData>
        + PresentationTimeHandler,
{
    fn event(
        data: &mut D,
        feedback: &wp_presentation_feedback::WpPresentationFeedback,
        event: wp_presentation_feedback::Event,
        udata: &PresentationTimeData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            wp_presentation_feedback::Event::SyncOutput { output } => {
                udata.sync_outputs.lock().unwrap().push(output);
            }
            wp_presentation_feedback::Event::Presented {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                refresh,
                seq_hi,
                seq_lo,
                flags,
            } => {
                let sync_outputs = mem::take(&mut *udata.sync_outputs.lock().unwrap());
                let clk_id = data.presentation_time_state().clk_id.unwrap(); // XXX unwrap
                let time = PresentTime {
                    clk_id,
                    tv_sec: ((tv_sec_hi as u64) << 32) | (tv_sec_lo as u64),
                    tv_nsec,
                };
                let seq = ((seq_hi as u64) << 32) | (seq_lo as u64);
                data.presented(
                    conn,
                    qh,
                    feedback,
                    &udata.wl_surface,
                    sync_outputs,
                    time,
                    refresh,
                    seq,
                    flags,
                );
            }
            wp_presentation_feedback::Event::Discarded => {
                data.discarded(conn, qh, feedback, &udata.wl_surface)
            }
            _ => {}
        }
    }
}

#[macro_export]
macro_rules! delegate_presentation_time {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::presentation_time::client::wp_presentation::WpPresentation: $crate::globals::GlobalData
        ] => $crate::presentation_time::PresentationTimeState);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::wp::presentation_time::client::wp_presentation_feedback::WpPresentationFeedback: $crate::presentation_time::PresentationTimeData
        ] => $crate::presentation_time::PresentationTimeState);
    };
}
