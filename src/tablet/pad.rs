use super::{
    devices::{notify_devices, TabletDeviceEvent},
    ListenerData,
};
use std::{cell::RefCell, rc::Rc, sync::Mutex};
use wayland_client::{protocol::wl_surface, Main};
use wayland_client::{Attached, DispatchData};
use wayland_protocols::unstable::tablet::v2::client::*;

pub(super) type PadCallback =
    dyn FnMut(Attached<zwp_tablet_pad_v2::ZwpTabletPadV2>, PadEvent, DispatchData) + 'static;

#[derive(Clone)]
pub enum PadEvent {
    Button { time: u32, button: u32, state: zwp_tablet_pad_v2::ButtonState },
    Enter { serial: u32, tablet: zwp_tablet_v2::ZwpTabletV2, surface: wl_surface::WlSurface },
    Leave { serial: u32, surface: wl_surface::WlSurface },
}
#[derive(Clone, Default)]
pub struct PadMetaData {
    path: String,
    buttons: u32,
}

pub(super) fn tablet_pad_cb(
    tablet_seat: Attached<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    tablet_pad: Main<zwp_tablet_pad_v2::ZwpTabletPadV2>,
    listener_data: Rc<RefCell<ListenerData>>,
    event: zwp_tablet_pad_v2::Event,
    ddata: DispatchData,
) {
    match event {
        zwp_tablet_pad_v2::Event::Path { path } => {
            let tool_data = tablet_pad.as_ref().user_data().get::<Mutex<PadMetaData>>().unwrap();
            let mut guard = tool_data.lock().unwrap();
            guard.path = path;
        }
        zwp_tablet_pad_v2::Event::Buttons { buttons } => {
            let tool_data = tablet_pad.as_ref().user_data().get::<Mutex<PadMetaData>>().unwrap();
            let mut guard = tool_data.lock().unwrap();
            guard.buttons = buttons;
        }
        zwp_tablet_pad_v2::Event::Done => notify_devices(
            &listener_data,
            TabletDeviceEvent::PadAdded { pad: tablet_pad.into() },
            ddata,
            &tablet_seat,
        ),
        zwp_tablet_pad_v2::Event::Removed => notify_devices(
            &listener_data,
            TabletDeviceEvent::PadRemoved { pad: tablet_pad.detach() },
            ddata,
            &tablet_seat,
        ),
        zwp_tablet_pad_v2::Event::Button { time, button, state } => notify_pads(
            &listener_data,
            PadEvent::Button { time, button, state },
            ddata,
            &tablet_pad,
        ),
        zwp_tablet_pad_v2::Event::Enter { serial, tablet, surface } => notify_pads(
            &listener_data,
            PadEvent::Enter { serial, tablet, surface },
            ddata,
            &tablet_pad,
        ),
        _ => {}
    }
}

fn notify_pads(
    listener_data: &Rc<RefCell<ListenerData>>,
    event: PadEvent,
    mut ddata: DispatchData,
    tablet_pad: &Attached<zwp_tablet_pad_v2::ZwpTabletPadV2>,
) {
    let mut shared_data = listener_data.borrow_mut();
    shared_data.pad_listeners.invoke_all(move |cb| {
        (&mut *cb.borrow_mut())(tablet_pad.clone(), event.clone(), ddata.reborrow());
    });
}

pub fn clone_pad_data(tablet: &zwp_tablet_tool_v2::ZwpTabletToolV2) -> Option<PadMetaData> {
    if let Some(ref udata_mutex) = tablet.as_ref().user_data().get::<Mutex<PadMetaData>>() {
        let udata = udata_mutex.lock().unwrap();
        Some(udata.clone())
    } else {
        None
    }
}

pub fn with_pad_data<T, F: FnOnce(&PadMetaData) -> T>(
    seat: &zwp_tablet_v2::ZwpTabletV2,
    f: F,
) -> Option<T> {
    if let Some(ref udata_mutex) = seat.as_ref().user_data().get::<Mutex<PadMetaData>>() {
        let udata = udata_mutex.lock().unwrap();
        Some(f(&*udata))
    } else {
        None
    }
}
