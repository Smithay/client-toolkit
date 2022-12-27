// TODO: Cursor session
// TODO: wrapper around a session

use crate::registry::GlobalProxy;

use std::sync::Mutex;
use wayland_client::protocol::wl_shm;
use wayland_client::{globals::GlobalList, Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::{
    image_source::v1::client::{ext_image_source_manager_v1, ext_image_source_v1},
    screencopy::v1::client::{ext_screencopy_manager_v1, ext_screencopy_session_v1},
};

#[derive(Clone, Debug)]
pub struct BufferConstraintsShm {
    pub format: WEnum<wl_shm::Format>,
    pub min_width: u32,
    pub min_height: u32,
    pub optimal_stride: u32,
}

#[derive(Clone, Debug)]
pub struct BufferConstraintsDmabuf {
    pub format: u32,
    pub min_width: u32,
    pub min_height: u32,
}

#[derive(Debug)]
pub struct ScreencopyState {
    pub screencopy_manager: GlobalProxy<ext_screencopy_manager_v1::ExtScreencopyManagerV1>, // XXX pub
}

impl ScreencopyState {
    pub fn new<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Self
    where
        D: Dispatch<ext_screencopy_manager_v1::ExtScreencopyManagerV1, ()> + 'static,
    {
        let screencopy_manager = GlobalProxy::from(globals.bind(qh, 1..=1, ()));
        Self { screencopy_manager }
    }
}

pub trait ScreencopyHandler: Sized {
    fn screencopy_state(&mut self) -> &mut ScreencopyState;

    fn buffer_constraints(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &ext_screencopy_session_v1::ExtScreencopySessionV1,
        shm_constraints: &[BufferConstraintsShm],
        dmabuf_constraints: &[BufferConstraintsDmabuf],
    );

    // needs to take transform, damage, presentatation_time
    fn ready(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &ext_screencopy_session_v1::ExtScreencopySessionV1,
    );

    fn failed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        session: &ext_screencopy_session_v1::ExtScreencopySessionV1,
        reason: WEnum<ext_screencopy_session_v1::FailureReason>,
    );
}

#[derive(Default)]
pub struct ScreencopySessionData {
    shm_constraints: Mutex<Vec<BufferConstraintsShm>>,
    dmabuf_constraints: Mutex<Vec<BufferConstraintsDmabuf>>,
}

impl<D> Dispatch<ext_image_source_v1::ExtImageSourceV1, (), D> for ScreencopyState
where
    D: Dispatch<ext_image_source_v1::ExtImageSourceV1, ()> + ScreencopyHandler,
{
    fn event(
        state: &mut D,
        _: &ext_image_source_v1::ExtImageSourceV1,
        event: ext_image_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ext_image_source_manager_v1::ExtImageSourceManagerV1, (), D> for ScreencopyState
where
    D: Dispatch<ext_image_source_manager_v1::ExtImageSourceManagerV1, ()> + ScreencopyHandler,
{
    fn event(
        state: &mut D,
        _: &ext_image_source_manager_v1::ExtImageSourceManagerV1,
        event: ext_image_source_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ext_screencopy_manager_v1::ExtScreencopyManagerV1, (), D> for ScreencopyState
where
    D: Dispatch<ext_screencopy_manager_v1::ExtScreencopyManagerV1, ()> + ScreencopyHandler,
{
    fn event(
        state: &mut D,
        _: &ext_screencopy_manager_v1::ExtScreencopyManagerV1,
        event: ext_screencopy_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        match event {
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ext_screencopy_session_v1::ExtScreencopySessionV1, ScreencopySessionData, D>
    for ScreencopyState
where
    D: Dispatch<ext_screencopy_session_v1::ExtScreencopySessionV1, ScreencopySessionData>
        + ScreencopyHandler,
{
    fn event(
        app_data: &mut D,
        session: &ext_screencopy_session_v1::ExtScreencopySessionV1,
        event: ext_screencopy_session_v1::Event,
        data: &ScreencopySessionData,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        //println!("{:?}", event);
        match event {
            ext_screencopy_session_v1::Event::BufferConstraintsShm {
                format,
                min_width,
                min_height,
                optimal_stride,
            } => {
                let constraints =
                    BufferConstraintsShm { format, min_width, min_height, optimal_stride };
                data.shm_constraints.lock().unwrap().push(constraints);
            }

            ext_screencopy_session_v1::Event::BufferConstraintsDmabuf {
                format,
                min_width,
                min_height,
            } => {
                let constraints = BufferConstraintsDmabuf { format, min_width, min_height };
                data.dmabuf_constraints.lock().unwrap().push(constraints);
            }

            ext_screencopy_session_v1::Event::BufferConstraintsDone => {
                let shm_constraints = data.shm_constraints.lock().unwrap();
                let dmabuf_constraints = data.dmabuf_constraints.lock().unwrap();
                app_data.buffer_constraints(
                    conn,
                    qh,
                    session,
                    &*shm_constraints,
                    &*dmabuf_constraints,
                );
            }

            ext_screencopy_session_v1::Event::Transform { transform } => {}

            ext_screencopy_session_v1::Event::Damage { x, y, width, height } => {}

            ext_screencopy_session_v1::Event::Failed { reason } => {
                app_data.failed(conn, qh, session, reason);
            }

            ext_screencopy_session_v1::Event::PresentationTime {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let secs = (u64::from(tv_sec_hi) << 32) + u64::from(tv_sec_lo);
                // TODO
            }

            ext_screencopy_session_v1::Event::Ready => {
                app_data.ready(conn, qh, session); // pass other info?
            }

            _ => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! delegate_screencopy {
    ($ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::image_source::v1::client::ext_image_source_manager_v1::ExtImageSourceManagerV1: ()
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::image_source::v1::client::ext_image_source_v1::ExtImageSourceV1: ()
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::screencopy::v1::client::ext_screencopy_manager_v1::ExtScreencopyManagerV1: ()
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($ty: [
            $crate::cosmic_protocols::screencopy::v1::client::ext_screencopy_session_v1::ExtScreencopySessionV1: $crate::screencopy::ScreencopySessionData
        ] => $crate::screencopy::ScreencopyState);
    };
}
