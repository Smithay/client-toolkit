use wayland_protocols::xdg_shell::client::xdg_popup;

#[derive(Debug)]
pub(crate) struct XdgPopupInner {
    pub(crate) popup: xdg_popup::XdgPopup,
}
