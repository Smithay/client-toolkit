use cursor_icon::CursorIcon;

use crate::globals::GlobalData;
use crate::reexports::client::globals::{BindError, GlobalList};
use crate::reexports::client::protocol::wl_pointer::WlPointer;
use crate::reexports::client::{Connection, Dispatch, Proxy, QueueHandle};
use crate::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;
use crate::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1;
use crate::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_manager_v1::WpCursorShapeManagerV1;

#[derive(Debug)]
pub struct CursorShapeManager {
    cursor_shape_manager: WpCursorShapeManagerV1,
}

impl CursorShapeManager {
    pub fn bind<State>(
        globals: &GlobalList,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, BindError>
    where
        State: Dispatch<WpCursorShapeManagerV1, GlobalData> + 'static,
    {
        let cursor_shape_manager = globals.bind(queue_handle, 1..=2, GlobalData)?;
        Ok(Self { cursor_shape_manager })
    }

    pub(crate) fn from_existing(cursor_shape_manager: WpCursorShapeManagerV1) -> Self {
        Self { cursor_shape_manager }
    }

    pub fn get_shape_device<State>(
        &self,
        pointer: &WlPointer,
        queue_handle: &QueueHandle<State>,
    ) -> WpCursorShapeDeviceV1
    where
        State: Dispatch<WpCursorShapeDeviceV1, GlobalData> + 'static,
    {
        self.cursor_shape_manager.get_pointer(pointer, queue_handle, GlobalData)
    }

    pub fn inner(&self) -> &WpCursorShapeManagerV1 {
        &self.cursor_shape_manager
    }
}

impl<State> Dispatch<WpCursorShapeManagerV1, GlobalData, State> for CursorShapeManager
where
    State: Dispatch<WpCursorShapeManagerV1, GlobalData>,
{
    fn event(
        _: &mut State,
        _: &WpCursorShapeManagerV1,
        _: <WpCursorShapeManagerV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        unreachable!("wl_cursor_shape_manager_v1 has no events")
    }
}

impl<State> Dispatch<WpCursorShapeDeviceV1, GlobalData, State> for CursorShapeManager
where
    State: Dispatch<WpCursorShapeDeviceV1, GlobalData>,
{
    fn event(
        _: &mut State,
        _: &WpCursorShapeDeviceV1,
        _: <WpCursorShapeDeviceV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        unreachable!("wl_cursor_shape_device_v1 has no events")
    }
}

pub(crate) fn cursor_icon_to_shape(cursor_icon: CursorIcon, version: u32) -> Shape {
    match cursor_icon {
        CursorIcon::Default => Shape::Default,
        CursorIcon::ContextMenu => Shape::ContextMenu,
        CursorIcon::Help => Shape::Help,
        CursorIcon::Pointer => Shape::Pointer,
        CursorIcon::Progress => Shape::Progress,
        CursorIcon::Wait => Shape::Wait,
        CursorIcon::Cell => Shape::Cell,
        CursorIcon::Crosshair => Shape::Crosshair,
        CursorIcon::Text => Shape::Text,
        CursorIcon::VerticalText => Shape::VerticalText,
        CursorIcon::Alias => Shape::Alias,
        CursorIcon::Copy => Shape::Copy,
        CursorIcon::Move => Shape::Move,
        CursorIcon::NoDrop => Shape::NoDrop,
        CursorIcon::NotAllowed => Shape::NotAllowed,
        CursorIcon::Grab => Shape::Grab,
        CursorIcon::Grabbing => Shape::Grabbing,
        CursorIcon::EResize => Shape::EResize,
        CursorIcon::NResize => Shape::NResize,
        CursorIcon::NeResize => Shape::NeResize,
        CursorIcon::NwResize => Shape::NwResize,
        CursorIcon::SResize => Shape::SResize,
        CursorIcon::SeResize => Shape::SeResize,
        CursorIcon::SwResize => Shape::SwResize,
        CursorIcon::WResize => Shape::WResize,
        CursorIcon::EwResize => Shape::EwResize,
        CursorIcon::NsResize => Shape::NsResize,
        CursorIcon::NeswResize => Shape::NeswResize,
        CursorIcon::NwseResize => Shape::NwseResize,
        CursorIcon::ColResize => Shape::ColResize,
        CursorIcon::RowResize => Shape::RowResize,
        CursorIcon::AllScroll => Shape::AllScroll,
        CursorIcon::ZoomIn => Shape::ZoomIn,
        CursorIcon::ZoomOut => Shape::ZoomOut,
        CursorIcon::DndAsk if version >= 2 => Shape::DndAsk,
        CursorIcon::AllResize if version >= 2 => Shape::AllResize,
        _ => Shape::Default,
    }
}
