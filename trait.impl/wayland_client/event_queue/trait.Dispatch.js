(function() {var implementors = {
"smithay_client_toolkit":[["impl&lt;D&gt; Dispatch&lt;ZxdgDecorationManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/struct.XdgShell.html\" title=\"struct smithay_client_toolkit::shell::xdg::XdgShell\">XdgShell</a><div class=\"where\">where\n    D: Dispatch&lt;ZxdgDecorationManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/window/trait.WindowHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::window::WindowHandler\">WindowHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;ExtSessionLockSurfaceV1, <a class=\"struct\" href=\"smithay_client_toolkit/session_lock/struct.SessionLockSurfaceData.html\" title=\"struct smithay_client_toolkit::session_lock::SessionLockSurfaceData\">SessionLockSurfaceData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/session_lock/struct.SessionLockState.html\" title=\"struct smithay_client_toolkit::session_lock::SessionLockState\">SessionLockState</a><div class=\"where\">where\n    D: Dispatch&lt;ExtSessionLockSurfaceV1, <a class=\"struct\" href=\"smithay_client_toolkit/session_lock/struct.SessionLockSurfaceData.html\" title=\"struct smithay_client_toolkit::session_lock::SessionLockSurfaceData\">SessionLockSurfaceData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/session_lock/trait.SessionLockHandler.html\" title=\"trait smithay_client_toolkit::session_lock::SessionLockHandler\">SessionLockHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;WlSeat, <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatData.html\" title=\"struct smithay_client_toolkit::seat::SeatData\">SeatData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatState.html\" title=\"struct smithay_client_toolkit::seat::SeatState\">SeatState</a><div class=\"where\">where\n    D: Dispatch&lt;WlSeat, <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatData.html\" title=\"struct smithay_client_toolkit::seat::SeatData\">SeatData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/trait.SeatHandler.html\" title=\"trait smithay_client_toolkit::seat::SeatHandler\">SeatHandler</a>,</div>"],["impl&lt;D, I, const MAX_VERSION: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.76.0/std/primitive.u32.html\">u32</a>&gt; Dispatch&lt;I, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.76.0/std/primitive.unit.html\">()</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/registry/struct.SimpleGlobal.html\" title=\"struct smithay_client_toolkit::registry::SimpleGlobal\">SimpleGlobal</a>&lt;I, MAX_VERSION&gt;<div class=\"where\">where\n    D: Dispatch&lt;I, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.76.0/std/primitive.unit.html\">()</a>&gt;,\n    I: Proxy,</div>"],["impl&lt;D&gt; Dispatch&lt;ZwpLinuxDmabufV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/dmabuf/struct.DmabufState.html\" title=\"struct smithay_client_toolkit::dmabuf::DmabufState\">DmabufState</a><div class=\"where\">where\n    D: Dispatch&lt;ZwpLinuxDmabufV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/dmabuf/trait.DmabufHandler.html\" title=\"trait smithay_client_toolkit::dmabuf::DmabufHandler\">DmabufHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;ZxdgToplevelDecorationV1, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/struct.XdgShell.html\" title=\"struct smithay_client_toolkit::shell::xdg::XdgShell\">XdgShell</a><div class=\"where\">where\n    D: Dispatch&lt;ZxdgToplevelDecorationV1, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/window/trait.WindowHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::window::WindowHandler\">WindowHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;XdgSurface, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/struct.XdgShell.html\" title=\"struct smithay_client_toolkit::shell::xdg::XdgShell\">XdgShell</a><div class=\"where\">where\n    D: Dispatch&lt;XdgSurface, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/window/trait.WindowHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::window::WindowHandler\">WindowHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;ZwpLinuxBufferParamsV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/dmabuf/struct.DmabufState.html\" title=\"struct smithay_client_toolkit::dmabuf::DmabufState\">DmabufState</a><div class=\"where\">where\n    D: Dispatch&lt;ZwpLinuxBufferParamsV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + Dispatch&lt;WlBuffer, DmaBufferData&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/dmabuf/trait.DmabufHandler.html\" title=\"trait smithay_client_toolkit::dmabuf::DmabufHandler\">DmabufHandler</a> + 'static,</div>"],["impl&lt;D, U&gt; Dispatch&lt;WlSurface, U, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/compositor/struct.CompositorState.html\" title=\"struct smithay_client_toolkit::compositor::CompositorState\">CompositorState</a><div class=\"where\">where\n    D: Dispatch&lt;WlSurface, U&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/compositor/trait.CompositorHandler.html\" title=\"trait smithay_client_toolkit::compositor::CompositorHandler\">CompositorHandler</a> + <a class=\"trait\" href=\"smithay_client_toolkit/output/trait.OutputHandler.html\" title=\"trait smithay_client_toolkit::output::OutputHandler\">OutputHandler</a> + 'static,\n    U: <a class=\"trait\" href=\"smithay_client_toolkit/compositor/trait.SurfaceDataExt.html\" title=\"trait smithay_client_toolkit::compositor::SurfaceDataExt\">SurfaceDataExt</a> + 'static,</div>"],["impl&lt;D&gt; Dispatch&lt;WlSubsurface, <a class=\"struct\" href=\"smithay_client_toolkit/subcompositor/struct.SubsurfaceData.html\" title=\"struct smithay_client_toolkit::subcompositor::SubsurfaceData\">SubsurfaceData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/subcompositor/struct.SubcompositorState.html\" title=\"struct smithay_client_toolkit::subcompositor::SubcompositorState\">SubcompositorState</a><div class=\"where\">where\n    D: Dispatch&lt;WlSubsurface, <a class=\"struct\" href=\"smithay_client_toolkit/subcompositor/struct.SubsurfaceData.html\" title=\"struct smithay_client_toolkit::subcompositor::SubsurfaceData\">SubsurfaceData</a>&gt;,</div>"],["impl&lt;D&gt; Dispatch&lt;ZxdgOutputManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputState.html\" title=\"struct smithay_client_toolkit::output::OutputState\">OutputState</a><div class=\"where\">where\n    D: Dispatch&lt;ZxdgOutputManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/output/trait.OutputHandler.html\" title=\"trait smithay_client_toolkit::output::OutputHandler\">OutputHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;ZxdgOutputV1, <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputData.html\" title=\"struct smithay_client_toolkit::output::OutputData\">OutputData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputState.html\" title=\"struct smithay_client_toolkit::output::OutputState\">OutputState</a><div class=\"where\">where\n    D: Dispatch&lt;ZxdgOutputV1, <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputData.html\" title=\"struct smithay_client_toolkit::output::OutputData\">OutputData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/output/trait.OutputHandler.html\" title=\"trait smithay_client_toolkit::output::OutputHandler\">OutputHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;WlCompositor, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/compositor/struct.CompositorState.html\" title=\"struct smithay_client_toolkit::compositor::CompositorState\">CompositorState</a><div class=\"where\">where\n    D: Dispatch&lt;WlCompositor, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/compositor/trait.CompositorHandler.html\" title=\"trait smithay_client_toolkit::compositor::CompositorHandler\">CompositorHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;WlDataDeviceManager, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/data_device_manager/struct.DataDeviceManagerState.html\" title=\"struct smithay_client_toolkit::data_device_manager::DataDeviceManagerState\">DataDeviceManagerState</a><div class=\"where\">where\n    D: Dispatch&lt;WlDataDeviceManager, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt;,</div>"],["impl&lt;State&gt; Dispatch&lt;WpCursorShapeManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, State&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/pointer/cursor_shape/struct.CursorShapeManager.html\" title=\"struct smithay_client_toolkit::seat::pointer::cursor_shape::CursorShapeManager\">CursorShapeManager</a><div class=\"where\">where\n    State: Dispatch&lt;WpCursorShapeManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt;,</div>"],["impl&lt;D, R&gt; Dispatch&lt;XdgActivationTokenV1, R, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/activation/struct.ActivationState.html\" title=\"struct smithay_client_toolkit::activation::ActivationState\">ActivationState</a><div class=\"where\">where\n    D: Dispatch&lt;XdgActivationTokenV1, R&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/activation/trait.ActivationHandler.html\" title=\"trait smithay_client_toolkit::activation::ActivationHandler\">ActivationHandler</a>&lt;RequestData = R&gt;,\n    R: <a class=\"trait\" href=\"smithay_client_toolkit/activation/trait.RequestDataExt.html\" title=\"trait smithay_client_toolkit::activation::RequestDataExt\">RequestDataExt</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;ExtSessionLockV1, <a class=\"struct\" href=\"smithay_client_toolkit/session_lock/struct.SessionLockData.html\" title=\"struct smithay_client_toolkit::session_lock::SessionLockData\">SessionLockData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/session_lock/struct.SessionLockState.html\" title=\"struct smithay_client_toolkit::session_lock::SessionLockState\">SessionLockState</a><div class=\"where\">where\n    D: Dispatch&lt;ExtSessionLockV1, <a class=\"struct\" href=\"smithay_client_toolkit/session_lock/struct.SessionLockData.html\" title=\"struct smithay_client_toolkit::session_lock::SessionLockData\">SessionLockData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/session_lock/trait.SessionLockHandler.html\" title=\"trait smithay_client_toolkit::session_lock::SessionLockHandler\">SessionLockHandler</a>,</div>"],["impl&lt;State&gt; Dispatch&lt;ZwpPrimarySelectionOfferV1, <a class=\"struct\" href=\"smithay_client_toolkit/primary_selection/offer/struct.PrimarySelectionOfferData.html\" title=\"struct smithay_client_toolkit::primary_selection::offer::PrimarySelectionOfferData\">PrimarySelectionOfferData</a>, State&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/primary_selection/struct.PrimarySelectionManagerState.html\" title=\"struct smithay_client_toolkit::primary_selection::PrimarySelectionManagerState\">PrimarySelectionManagerState</a><div class=\"where\">where\n    State: Dispatch&lt;ZwpPrimarySelectionOfferV1, <a class=\"struct\" href=\"smithay_client_toolkit/primary_selection/offer/struct.PrimarySelectionOfferData.html\" title=\"struct smithay_client_toolkit::primary_selection::offer::PrimarySelectionOfferData\">PrimarySelectionOfferData</a>&gt;,</div>"],["impl&lt;D&gt; Dispatch&lt;WlDataDevice, <a class=\"struct\" href=\"smithay_client_toolkit/data_device_manager/data_device/struct.DataDeviceData.html\" title=\"struct smithay_client_toolkit::data_device_manager::data_device::DataDeviceData\">DataDeviceData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/data_device_manager/struct.DataDeviceManagerState.html\" title=\"struct smithay_client_toolkit::data_device_manager::DataDeviceManagerState\">DataDeviceManagerState</a><div class=\"where\">where\n    D: Dispatch&lt;WlDataDevice, <a class=\"struct\" href=\"smithay_client_toolkit/data_device_manager/data_device/struct.DataDeviceData.html\" title=\"struct smithay_client_toolkit::data_device_manager::data_device::DataDeviceData\">DataDeviceData</a>&gt; + Dispatch&lt;WlDataOffer, <a class=\"struct\" href=\"smithay_client_toolkit/data_device_manager/data_offer/struct.DataOfferData.html\" title=\"struct smithay_client_toolkit::data_device_manager::data_offer::DataOfferData\">DataOfferData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/data_device_manager/data_device/trait.DataDeviceHandler.html\" title=\"trait smithay_client_toolkit::data_device_manager::data_device::DataDeviceHandler\">DataDeviceHandler</a> + <a class=\"trait\" href=\"smithay_client_toolkit/data_device_manager/data_offer/trait.DataOfferHandler.html\" title=\"trait smithay_client_toolkit::data_device_manager::data_offer::DataOfferHandler\">DataOfferHandler</a> + 'static,</div>"],["impl&lt;D&gt; Dispatch&lt;ZwlrLayerSurfaceV1, <a class=\"struct\" href=\"smithay_client_toolkit/shell/wlr_layer/struct.LayerSurfaceData.html\" title=\"struct smithay_client_toolkit::shell::wlr_layer::LayerSurfaceData\">LayerSurfaceData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/wlr_layer/struct.LayerShell.html\" title=\"struct smithay_client_toolkit::shell::wlr_layer::LayerShell\">LayerShell</a><div class=\"where\">where\n    D: Dispatch&lt;ZwlrLayerSurfaceV1, <a class=\"struct\" href=\"smithay_client_toolkit/shell/wlr_layer/struct.LayerSurfaceData.html\" title=\"struct smithay_client_toolkit::shell::wlr_layer::LayerSurfaceData\">LayerSurfaceData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/wlr_layer/trait.LayerShellHandler.html\" title=\"trait smithay_client_toolkit::shell::wlr_layer::LayerShellHandler\">LayerShellHandler</a> + 'static,</div>"],["impl&lt;D&gt; Dispatch&lt;WlShm, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shm/struct.Shm.html\" title=\"struct smithay_client_toolkit::shm::Shm\">Shm</a><div class=\"where\">where\n    D: Dispatch&lt;WlShm, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shm/trait.ShmHandler.html\" title=\"trait smithay_client_toolkit::shm::ShmHandler\">ShmHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;XdgActivationV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/activation/struct.ActivationState.html\" title=\"struct smithay_client_toolkit::activation::ActivationState\">ActivationState</a><div class=\"where\">where\n    D: Dispatch&lt;XdgActivationV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/activation/trait.ActivationHandler.html\" title=\"trait smithay_client_toolkit::activation::ActivationHandler\">ActivationHandler</a>,</div>"],["impl&lt;D, U&gt; Dispatch&lt;WlTouch, U, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatState.html\" title=\"struct smithay_client_toolkit::seat::SeatState\">SeatState</a><div class=\"where\">where\n    D: Dispatch&lt;WlTouch, U&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/touch/trait.TouchHandler.html\" title=\"trait smithay_client_toolkit::seat::touch::TouchHandler\">TouchHandler</a>,\n    U: <a class=\"trait\" href=\"smithay_client_toolkit/seat/touch/trait.TouchDataExt.html\" title=\"trait smithay_client_toolkit::seat::touch::TouchDataExt\">TouchDataExt</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;ZwlrLayerShellV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/wlr_layer/struct.LayerShell.html\" title=\"struct smithay_client_toolkit::shell::wlr_layer::LayerShell\">LayerShell</a><div class=\"where\">where\n    D: Dispatch&lt;ZwlrLayerShellV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/wlr_layer/trait.LayerShellHandler.html\" title=\"trait smithay_client_toolkit::shell::wlr_layer::LayerShellHandler\">LayerShellHandler</a> + 'static,</div>"],["impl&lt;D, U&gt; Dispatch&lt;WlDataSource, U, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/data_device_manager/struct.DataDeviceManagerState.html\" title=\"struct smithay_client_toolkit::data_device_manager::DataDeviceManagerState\">DataDeviceManagerState</a><div class=\"where\">where\n    D: Dispatch&lt;WlDataSource, U&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/data_device_manager/data_source/trait.DataSourceHandler.html\" title=\"trait smithay_client_toolkit::data_device_manager::data_source::DataSourceHandler\">DataSourceHandler</a>,\n    U: <a class=\"trait\" href=\"smithay_client_toolkit/data_device_manager/data_source/trait.DataSourceDataExt.html\" title=\"trait smithay_client_toolkit::data_device_manager::data_source::DataSourceDataExt\">DataSourceDataExt</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;XdgToplevel, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/struct.XdgShell.html\" title=\"struct smithay_client_toolkit::shell::xdg::XdgShell\">XdgShell</a><div class=\"where\">where\n    D: Dispatch&lt;XdgToplevel, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/window/trait.WindowHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::window::WindowHandler\">WindowHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;ZwpPrimarySelectionDeviceManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/primary_selection/struct.PrimarySelectionManagerState.html\" title=\"struct smithay_client_toolkit::primary_selection::PrimarySelectionManagerState\">PrimarySelectionManagerState</a><div class=\"where\">where\n    D: Dispatch&lt;ZwpPrimarySelectionDeviceManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt;,</div>"],["impl&lt;D&gt; Dispatch&lt;WlSubcompositor, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/subcompositor/struct.SubcompositorState.html\" title=\"struct smithay_client_toolkit::subcompositor::SubcompositorState\">SubcompositorState</a><div class=\"where\">where\n    D: Dispatch&lt;WlSubcompositor, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt;,</div>"],["impl&lt;State&gt; Dispatch&lt;WpCursorShapeDeviceV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, State&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/pointer/cursor_shape/struct.CursorShapeManager.html\" title=\"struct smithay_client_toolkit::seat::pointer::cursor_shape::CursorShapeManager\">CursorShapeManager</a><div class=\"where\">where\n    State: Dispatch&lt;WpCursorShapeDeviceV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt;,</div>"],["impl&lt;D&gt; Dispatch&lt;ExtSessionLockManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/session_lock/struct.SessionLockState.html\" title=\"struct smithay_client_toolkit::session_lock::SessionLockState\">SessionLockState</a><div class=\"where\">where\n    D: Dispatch&lt;ExtSessionLockManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt;,</div>"],["impl&lt;D&gt; Dispatch&lt;WlRegistry, GlobalListContents, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/registry/struct.RegistryState.html\" title=\"struct smithay_client_toolkit::registry::RegistryState\">RegistryState</a><div class=\"where\">where\n    D: Dispatch&lt;WlRegistry, GlobalListContents&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/registry/trait.ProvidesRegistryState.html\" title=\"trait smithay_client_toolkit::registry::ProvidesRegistryState\">ProvidesRegistryState</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;XdgSurface, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a><div class=\"where\">where\n    D: Dispatch&lt;XdgSurface, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/popup/trait.PopupHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::popup::PopupHandler\">PopupHandler</a>,</div>"],["impl&lt;D, U&gt; Dispatch&lt;WlKeyboard, U, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatState.html\" title=\"struct smithay_client_toolkit::seat::SeatState\">SeatState</a><div class=\"where\">where\n    D: Dispatch&lt;WlKeyboard, U&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/keyboard/trait.KeyboardHandler.html\" title=\"trait smithay_client_toolkit::seat::keyboard::KeyboardHandler\">KeyboardHandler</a>,\n    U: <a class=\"trait\" href=\"smithay_client_toolkit/seat/keyboard/trait.KeyboardDataExt.html\" title=\"trait smithay_client_toolkit::seat::keyboard::KeyboardDataExt\">KeyboardDataExt</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;ZwpRelativePointerManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/relative_pointer/struct.RelativePointerState.html\" title=\"struct smithay_client_toolkit::seat::relative_pointer::RelativePointerState\">RelativePointerState</a><div class=\"where\">where\n    D: Dispatch&lt;ZwpRelativePointerManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/relative_pointer/trait.RelativePointerHandler.html\" title=\"trait smithay_client_toolkit::seat::relative_pointer::RelativePointerHandler\">RelativePointerHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;WlDataOffer, <a class=\"struct\" href=\"smithay_client_toolkit/data_device_manager/data_offer/struct.DataOfferData.html\" title=\"struct smithay_client_toolkit::data_device_manager::data_offer::DataOfferData\">DataOfferData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/data_device_manager/struct.DataDeviceManagerState.html\" title=\"struct smithay_client_toolkit::data_device_manager::DataDeviceManagerState\">DataDeviceManagerState</a><div class=\"where\">where\n    D: Dispatch&lt;WlDataOffer, <a class=\"struct\" href=\"smithay_client_toolkit/data_device_manager/data_offer/struct.DataOfferData.html\" title=\"struct smithay_client_toolkit::data_device_manager::data_offer::DataOfferData\">DataOfferData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/data_device_manager/data_offer/trait.DataOfferHandler.html\" title=\"trait smithay_client_toolkit::data_device_manager::data_offer::DataOfferHandler\">DataOfferHandler</a>,</div>"],["impl&lt;D, U&gt; Dispatch&lt;WlPointer, U, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatState.html\" title=\"struct smithay_client_toolkit::seat::SeatState\">SeatState</a><div class=\"where\">where\n    D: Dispatch&lt;WlPointer, U&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/pointer/trait.PointerHandler.html\" title=\"trait smithay_client_toolkit::seat::pointer::PointerHandler\">PointerHandler</a>,\n    U: <a class=\"trait\" href=\"smithay_client_toolkit/seat/pointer/trait.PointerDataExt.html\" title=\"trait smithay_client_toolkit::seat::pointer::PointerDataExt\">PointerDataExt</a>,</div>"],["impl&lt;State&gt; Dispatch&lt;ZwpPrimarySelectionDeviceV1, <a class=\"struct\" href=\"smithay_client_toolkit/primary_selection/device/struct.PrimarySelectionDeviceData.html\" title=\"struct smithay_client_toolkit::primary_selection::device::PrimarySelectionDeviceData\">PrimarySelectionDeviceData</a>, State&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/primary_selection/struct.PrimarySelectionManagerState.html\" title=\"struct smithay_client_toolkit::primary_selection::PrimarySelectionManagerState\">PrimarySelectionManagerState</a><div class=\"where\">where\n    State: Dispatch&lt;ZwpPrimarySelectionDeviceV1, <a class=\"struct\" href=\"smithay_client_toolkit/primary_selection/device/struct.PrimarySelectionDeviceData.html\" title=\"struct smithay_client_toolkit::primary_selection::device::PrimarySelectionDeviceData\">PrimarySelectionDeviceData</a>&gt; + Dispatch&lt;ZwpPrimarySelectionOfferV1, <a class=\"struct\" href=\"smithay_client_toolkit/primary_selection/offer/struct.PrimarySelectionOfferData.html\" title=\"struct smithay_client_toolkit::primary_selection::offer::PrimarySelectionOfferData\">PrimarySelectionOfferData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/primary_selection/device/trait.PrimarySelectionDeviceHandler.html\" title=\"trait smithay_client_toolkit::primary_selection::device::PrimarySelectionDeviceHandler\">PrimarySelectionDeviceHandler</a> + 'static,</div>"],["impl&lt;D&gt; Dispatch&lt;XdgWmBase, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/struct.XdgShell.html\" title=\"struct smithay_client_toolkit::shell::xdg::XdgShell\">XdgShell</a><div class=\"where\">where\n    D: Dispatch&lt;XdgWmBase, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt;,</div>"],["impl&lt;State&gt; Dispatch&lt;ZwpPrimarySelectionSourceV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, State&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/primary_selection/struct.PrimarySelectionManagerState.html\" title=\"struct smithay_client_toolkit::primary_selection::PrimarySelectionManagerState\">PrimarySelectionManagerState</a><div class=\"where\">where\n    State: Dispatch&lt;ZwpPrimarySelectionSourceV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/primary_selection/selection/trait.PrimarySelectionSourceHandler.html\" title=\"trait smithay_client_toolkit::primary_selection::selection::PrimarySelectionSourceHandler\">PrimarySelectionSourceHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;ZwpPointerConstraintsV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/pointer_constraints/struct.PointerConstraintsState.html\" title=\"struct smithay_client_toolkit::seat::pointer_constraints::PointerConstraintsState\">PointerConstraintsState</a><div class=\"where\">where\n    D: Dispatch&lt;ZwpPointerConstraintsV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/pointer_constraints/trait.PointerConstraintsHandler.html\" title=\"trait smithay_client_toolkit::seat::pointer_constraints::PointerConstraintsHandler\">PointerConstraintsHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;WlCallback, WlSurface, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/compositor/struct.CompositorState.html\" title=\"struct smithay_client_toolkit::compositor::CompositorState\">CompositorState</a><div class=\"where\">where\n    D: Dispatch&lt;WlCallback, WlSurface&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/compositor/trait.CompositorHandler.html\" title=\"trait smithay_client_toolkit::compositor::CompositorHandler\">CompositorHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;XdgPopup, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a><div class=\"where\">where\n    D: Dispatch&lt;XdgPopup, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/popup/trait.PopupHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::popup::PopupHandler\">PopupHandler</a>,</div>"],["impl&lt;D&gt; Dispatch&lt;WlOutput, <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputData.html\" title=\"struct smithay_client_toolkit::output::OutputData\">OutputData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputState.html\" title=\"struct smithay_client_toolkit::output::OutputState\">OutputState</a><div class=\"where\">where\n    D: Dispatch&lt;WlOutput, <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputData.html\" title=\"struct smithay_client_toolkit::output::OutputData\">OutputData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/output/trait.OutputHandler.html\" title=\"trait smithay_client_toolkit::output::OutputHandler\">OutputHandler</a> + 'static,</div>"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()