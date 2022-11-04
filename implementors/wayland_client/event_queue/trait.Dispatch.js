(function() {var implementors = {
"smithay_client_toolkit":[["impl&lt;D, U&gt; Dispatch&lt;WlSurface, U, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/compositor/struct.CompositorState.html\" title=\"struct smithay_client_toolkit::compositor::CompositorState\">CompositorState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlSurface, U&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/compositor/trait.CompositorHandler.html\" title=\"trait smithay_client_toolkit::compositor::CompositorHandler\">CompositorHandler</a> + <a class=\"trait\" href=\"smithay_client_toolkit/output/trait.OutputHandler.html\" title=\"trait smithay_client_toolkit::output::OutputHandler\">OutputHandler</a> + 'static,<br>&nbsp;&nbsp;&nbsp;&nbsp;U: <a class=\"trait\" href=\"smithay_client_toolkit/compositor/trait.SurfaceDataExt.html\" title=\"trait smithay_client_toolkit::compositor::SurfaceDataExt\">SurfaceDataExt</a> + 'static,</span>"],["impl&lt;D&gt; Dispatch&lt;WlCompositor, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/compositor/struct.CompositorState.html\" title=\"struct smithay_client_toolkit::compositor::CompositorState\">CompositorState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlCompositor, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/compositor/trait.CompositorHandler.html\" title=\"trait smithay_client_toolkit::compositor::CompositorHandler\">CompositorHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;WlCallback, WlSurface, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/compositor/struct.CompositorState.html\" title=\"struct smithay_client_toolkit::compositor::CompositorState\">CompositorState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlCallback, WlSurface&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/compositor/trait.CompositorHandler.html\" title=\"trait smithay_client_toolkit::compositor::CompositorHandler\">CompositorHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;WlOutput, <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputData.html\" title=\"struct smithay_client_toolkit::output::OutputData\">OutputData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputState.html\" title=\"struct smithay_client_toolkit::output::OutputState\">OutputState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlOutput, <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputData.html\" title=\"struct smithay_client_toolkit::output::OutputData\">OutputData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/output/trait.OutputHandler.html\" title=\"trait smithay_client_toolkit::output::OutputHandler\">OutputHandler</a> + 'static,</span>"],["impl&lt;D&gt; Dispatch&lt;ZxdgOutputManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputState.html\" title=\"struct smithay_client_toolkit::output::OutputState\">OutputState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;ZxdgOutputManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/output/trait.OutputHandler.html\" title=\"trait smithay_client_toolkit::output::OutputHandler\">OutputHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;ZxdgOutputV1, <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputData.html\" title=\"struct smithay_client_toolkit::output::OutputData\">OutputData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputState.html\" title=\"struct smithay_client_toolkit::output::OutputState\">OutputState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;ZxdgOutputV1, <a class=\"struct\" href=\"smithay_client_toolkit/output/struct.OutputData.html\" title=\"struct smithay_client_toolkit::output::OutputData\">OutputData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/output/trait.OutputHandler.html\" title=\"trait smithay_client_toolkit::output::OutputHandler\">OutputHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;WlRegistry, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/registry/struct.RegistryState.html\" title=\"struct smithay_client_toolkit::registry::RegistryState\">RegistryState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlRegistry, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/registry/trait.ProvidesRegistryState.html\" title=\"trait smithay_client_toolkit::registry::ProvidesRegistryState\">ProvidesRegistryState</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;WlCallback, <a class=\"struct\" href=\"smithay_client_toolkit/registry/struct.RegistryReady.html\" title=\"struct smithay_client_toolkit::registry::RegistryReady\">RegistryReady</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/registry/struct.RegistryState.html\" title=\"struct smithay_client_toolkit::registry::RegistryState\">RegistryState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlCallback, <a class=\"struct\" href=\"smithay_client_toolkit/registry/struct.RegistryReady.html\" title=\"struct smithay_client_toolkit::registry::RegistryReady\">RegistryReady</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/registry/trait.ProvidesRegistryState.html\" title=\"trait smithay_client_toolkit::registry::ProvidesRegistryState\">ProvidesRegistryState</a>,</span>"],["impl&lt;D, I, const MAX_VERSION:&nbsp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.65.0/std/primitive.u32.html\">u32</a>&gt; Dispatch&lt;I, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.65.0/std/primitive.unit.html\">()</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/registry/struct.SimpleGlobal.html\" title=\"struct smithay_client_toolkit::registry::SimpleGlobal\">SimpleGlobal</a>&lt;I, MAX_VERSION&gt;<span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;I, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.65.0/std/primitive.unit.html\">()</a>&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;I: Proxy,</span>"],["impl&lt;D, U&gt; Dispatch&lt;WlKeyboard, U, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatState.html\" title=\"struct smithay_client_toolkit::seat::SeatState\">SeatState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlKeyboard, U&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/keyboard/trait.KeyboardHandler.html\" title=\"trait smithay_client_toolkit::seat::keyboard::KeyboardHandler\">KeyboardHandler</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;U: <a class=\"trait\" href=\"smithay_client_toolkit/seat/keyboard/trait.KeyboardDataExt.html\" title=\"trait smithay_client_toolkit::seat::keyboard::KeyboardDataExt\">KeyboardDataExt</a>,</span>"],["impl&lt;D, U&gt; Dispatch&lt;WlPointer, U, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatState.html\" title=\"struct smithay_client_toolkit::seat::SeatState\">SeatState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlPointer, U&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/pointer/trait.PointerHandler.html\" title=\"trait smithay_client_toolkit::seat::pointer::PointerHandler\">PointerHandler</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;U: <a class=\"trait\" href=\"smithay_client_toolkit/seat/pointer/trait.PointerDataExt.html\" title=\"trait smithay_client_toolkit::seat::pointer::PointerDataExt\">PointerDataExt</a>,</span>"],["impl&lt;D, U&gt; Dispatch&lt;WlTouch, U, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatState.html\" title=\"struct smithay_client_toolkit::seat::SeatState\">SeatState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlTouch, U&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/touch/trait.TouchHandler.html\" title=\"trait smithay_client_toolkit::seat::touch::TouchHandler\">TouchHandler</a>,<br>&nbsp;&nbsp;&nbsp;&nbsp;U: <a class=\"trait\" href=\"smithay_client_toolkit/seat/touch/trait.TouchDataExt.html\" title=\"trait smithay_client_toolkit::seat::touch::TouchDataExt\">TouchDataExt</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;WlSeat, <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatData.html\" title=\"struct smithay_client_toolkit::seat::SeatData\">SeatData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatState.html\" title=\"struct smithay_client_toolkit::seat::SeatState\">SeatState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlSeat, <a class=\"struct\" href=\"smithay_client_toolkit/seat/struct.SeatData.html\" title=\"struct smithay_client_toolkit::seat::SeatData\">SeatData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/seat/trait.SeatHandler.html\" title=\"trait smithay_client_toolkit::seat::SeatHandler\">SeatHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;ZwlrLayerShellV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/layer/struct.LayerShell.html\" title=\"struct smithay_client_toolkit::shell::layer::LayerShell\">LayerShell</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;ZwlrLayerShellV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/layer/trait.LayerShellHandler.html\" title=\"trait smithay_client_toolkit::shell::layer::LayerShellHandler\">LayerShellHandler</a> + 'static,</span>"],["impl&lt;D&gt; Dispatch&lt;ZwlrLayerSurfaceV1, <a class=\"struct\" href=\"smithay_client_toolkit/shell/layer/struct.LayerSurfaceData.html\" title=\"struct smithay_client_toolkit::shell::layer::LayerSurfaceData\">LayerSurfaceData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/layer/struct.LayerShell.html\" title=\"struct smithay_client_toolkit::shell::layer::LayerShell\">LayerShell</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;ZwlrLayerSurfaceV1, <a class=\"struct\" href=\"smithay_client_toolkit/shell/layer/struct.LayerSurfaceData.html\" title=\"struct smithay_client_toolkit::shell::layer::LayerSurfaceData\">LayerSurfaceData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/layer/trait.LayerShellHandler.html\" title=\"trait smithay_client_toolkit::shell::layer::LayerShellHandler\">LayerShellHandler</a> + 'static,</span>"],["impl&lt;D&gt; Dispatch&lt;XdgSurface, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;XdgSurface, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/popup/trait.PopupHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::popup::PopupHandler\">PopupHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;XdgPopup, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;XdgPopup, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/popup/struct.PopupData.html\" title=\"struct smithay_client_toolkit::shell::xdg::popup::PopupData\">PopupData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/popup/trait.PopupHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::popup::PopupHandler\">PopupHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;XdgSurface, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.XdgWindowState.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::XdgWindowState\">XdgWindowState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;XdgSurface, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/window/trait.WindowHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::window::WindowHandler\">WindowHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;XdgToplevel, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.XdgWindowState.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::XdgWindowState\">XdgWindowState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;XdgToplevel, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/window/trait.WindowHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::window::WindowHandler\">WindowHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;ZxdgDecorationManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.XdgWindowState.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::XdgWindowState\">XdgWindowState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;ZxdgDecorationManagerV1, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/window/trait.WindowHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::window::WindowHandler\">WindowHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;ZxdgToplevelDecorationV1, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.XdgWindowState.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::XdgWindowState\">XdgWindowState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;ZxdgToplevelDecorationV1, <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/window/struct.WindowData.html\" title=\"struct smithay_client_toolkit::shell::xdg::window::WindowData\">WindowData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shell/xdg/window/trait.WindowHandler.html\" title=\"trait smithay_client_toolkit::shell::xdg::window::WindowHandler\">WindowHandler</a>,</span>"],["impl&lt;D&gt; Dispatch&lt;XdgWmBase, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shell/xdg/struct.XdgShellState.html\" title=\"struct smithay_client_toolkit::shell::xdg::XdgShellState\">XdgShellState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;XdgWmBase, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt;,</span>"],["impl&lt;D&gt; Dispatch&lt;WlShm, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>, D&gt; for <a class=\"struct\" href=\"smithay_client_toolkit/shm/struct.ShmState.html\" title=\"struct smithay_client_toolkit::shm::ShmState\">ShmState</a><span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dispatch&lt;WlShm, <a class=\"struct\" href=\"smithay_client_toolkit/globals/struct.GlobalData.html\" title=\"struct smithay_client_toolkit::globals::GlobalData\">GlobalData</a>&gt; + <a class=\"trait\" href=\"smithay_client_toolkit/shm/trait.ShmHandler.html\" title=\"trait smithay_client_toolkit::shm::ShmHandler\">ShmHandler</a>,</span>"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()