use std::sync::{Arc, Mutex};

use super::{popup::inner::XdgPopupInner, window::inner::XdgToplevelInner};

#[derive(Debug)]
pub(crate) enum XdgSurfaceInner {
    Window(Arc<Mutex<XdgToplevelInner>>),

    Popup(Arc<Mutex<XdgPopupInner>>),

    Uninit,
}
