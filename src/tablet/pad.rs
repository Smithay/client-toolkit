use wayland_client::{Attached, DispatchData};
use wayland_protocols::unstable::tablet::v2::client::*;

pub(super) type PadCallback =
    dyn FnMut(Attached<zwp_tablet_pad_v2::ZwpTabletPadV2>, PadEvent, DispatchData) + 'static;

pub enum PadEvent {}
