# Change Log

## 0.20.0 - 2025-07-29

#### Breaking Changes
- Update `calloop` to `0.14.0`, `calloop-wayland-source` to `0.4.0`.
- Update `rustix` to `1.0.7`.
- Update `thiserror` to `2.0.12`.
- Add a `raw_modifiers` argument to `update_modifiers`.

#### Fixed
- Fix keyboard sending press events when repeat is disabled.

#### Additions
- Add support for `wl_keyboard` v10.
- Add support for `wl_pointer` v9.
- Add support for `wl_pointer` v8.
- Add support for `wp_cursor_shape_manager_v1` v2.
- Add partial support for `zwp-input-method-v2`.
- Add `xx-input-method-v2` protocol.
- Add `wp_presentation` protocol.
- Add `ext-foreign-toplevel-list-v1` protocol.
- Add `SimpleGlobal::from_bound` method to construct from proxy object.
- Add `Clone` for dmabuf feedback structs.
- Implement `AsFd` for `RawPool`.
- Implement `From<RawPool>` for `OwnedFd`.

## 0.19.2 - 2024-07-15

#### Fixed

- Fix crash when compositor sends event to dead `wl_output`.

## 0.19.1 - 2024-06-04

#### Additions

- `clone` derive for `CompositorState`

## 0.19.0 - 2024-05-31

#### Breaking Changes
- Update `calloop` to 0.13.0, `calloop-wayland-source` to `0.3.0`.
- Add `surface_enter`/`surface_leave` methods to `CompositorHandler` trait.
- Require explicit unlock call in SessionLock.
- Pass layout to `KeyboardHandler::update_modifiers`.
- Update `wayland-protocols-*`.

#### Fixed
- Require explicit unlock call in `SessionLock` to avoid accidental unlock.
- Work around touch up events delivered too late with certain Weston versions.
- Internal DnD event handlers are provided position and surface.
- `MultiPool::get` returns an overlap error when its appropriate.
- Fix `delegate_*` macros with custom `UserData`.

#### Additions
- Method to make subsurface from surface.
- Track latest touch_down event serial.
- Try alternative cursor icon names.
- Implement support for xdg-activation.
- Implement support for session-lock.

## 0.18.0 - 2023-09-23

#### Breaking Changes

- `ThemedPointer::set_cursor` now takes only `Connection` and `&str`.
- `SeatState:get_pointer_with_them*` now takes `Shm` and `WlSurface` for the themed cursor.
- `ThemedPointer` now automatically releases the associated `WlPointer`.
- `CursorIcon` from `cursor-icon` crate is now used for `set_cursor` and `Frame`.
- `wayland-csd-frame` is now used for CSD types like `WindowState`, `DecorationsFrame`, etc.
- Added `CompositorHandle::transform_changed` to listen for transform changes.
- `xkeysym::Keysym` is used as a keyboard key representation instead of `u32`
- `wayland-rs` dependencies are updated to 0.31
- `calloop` dependency updated to 0.12.1
- Take `OwnedFd` instead of `RawFd` as argument to `receive_to_fd` functions.

#### Fixed

- Crash when `wl_output` is below version 3.

#### Additions

- Make `DataDeviceManagerState`'s `create_{copy_paste,drag_and_drop}_source` accept `IntoIterator<Item = T: ToString>`.
- Add support for `zwp_primary_selection_v1`.
- `CursorShapeManager` providing handling for `cursor-shape-v1` protocol.
- `SeatState::get_pointer_with_theme` will now automatically use `wp_cursor_shape_v1` when available.
- Add support for `xdg_shell` version 6.
- Add support for `wl_surafce::preferred_buffer_scale` and `wl_surface::preferred_buffer_transform`.

## 0.17.0 - 2023-03-28

#### Breaking Changes

- `wayland-rs` dependencies are updated to 0.30 and all APIs have changed significantly as a result

#### Additions

- `xkbcommon` is a new optional dependecy for keyboard handling gated by the `xkbcommon` feature
- pointer-constraints-unstable-v1 protocol support
- relative-pointer-unstable-v1 protocol support
- wlr-layer-shell-unstable-v1 protocol support
- OutputInfo includes logical size and logical position
- New SHM pool types

## 0.16.0 - 2022-06-18

#### Breaking changes

- `calloop` is updated to version 0.10, and the keyboard handling API is slightly changed as a result.

#### Additions

- `DataDevice::with_dnd` and `DataOffer::receive_to_fd` allow more flexible interaction with the data device abstraction
- the output integration now supports version `4` of `wl_output`

## 0.15.4 - 2022-04-10

#### Bugfixes

- `Window`'s `wl_pointer` not being relased on `Drop`.

## 0.15.3 - 2021-12-27

#### Bugfixes

- SCTK now correctly interacts with the wayland socket being conccurently polled from
  other threads.

## 0.15.2 - 2021-10-27

- Most types are now `Debug`

## 0.15.1 - 2021-08-23

#### Bugfixes

- when not using `dlopen` feature, `xkbdcommon` library is linked using `pkg-config`

## 0.15.0 - 2021-08-10

#### Breaking Changes

- Update `wayland-client` to 0.29

#### Additions

- `AutoMemPool` now guarantees a minimum alignment of returned buffers

## 0.14.0 - 2021-05-07

#### Breaking Changes

- `ConceptFrame` is removed, as well as the `frames` cargo feature, and replaced by a more minimalistic
  `FallbackFrame`. Dependency on `andrew` and `fontconfig` is dropped in the process. If fancier
  decorations are needed, they should be implemented using the `Frame` trait.
- Update to calloop 0.7: `calloop::Source` is replaced by `calloop::RegistrationToken`

#### Additions

- `AutoMemPool` added as an alternative to the existing SHM pools

## 0.13.0 - 2021-03-04

#### Breaking Changes

- Mark OutputInfo as `#[non_exhaustive]` to allow future expansion without
  breaking API.
- Batch output information updates instead of potentially making multiple
  callbacks for one logical change
- Add name and description fields to OutputInfo.

#### Additions

- `Window::start_interactive_move` to enable dragging the window with a user action

#### Bugfixes

- `ConceptFrame` now correctly loads fonts using fontconfig

## 0.12.2 -- 2020-12-30

#### Changes

- Dependency on `byteorder` was replaced with `u32::from_ne_bytes()`

#### Bugfixes

- Don't crash when the font cannot be loaded to draw decorations

## 0.12.1 -- 2020-12-08

#### Changes

- Unmaintained `memmap` dependency is replaced with `memmap2`

## 0.12.0 -- 2020-09-30

#### Breaking Changes

- Update `wayland-client` to version 0.28
- `Environment::init` was renamed to `Environment::new_pending`
- `init_default_environment!` macro was renamed to `new_default_environment!`

#### Additions

- `Environment::new` method to fully bootstrap environment

## 0.11.0 -- 2020-08-30

#### Breaking Changes

- `window.set_decorate` is now taking mutable reference
- Added `show_window_menu` on a `Frame` trait to request a window menu for a window.
- `ShowMenu` enum variant to `FrameRequest`
- `create_window` now also takes `Option<ThemeManager>`
- `Frame::init` now also takes `Option<ThemeManager>` to reuse users' `ThemeManager`

#### Additions

- `WaylandSource::queue` to access the `EventQueue` underlying a `WaylandSource`
- A window menu could be shown on right click on decorations for `ConceptFrame`
- `ConceptFrame` will no longer change cursor over base surface if `ThemeManager` was provided

#### Changes

- `Window::set_title` now truncates the provided string to 1024 bytes, to avoid blowing up
  the Wayland connection
- `ConceptFrame` is now hiding decorations for `State::Fullscreen`
- Restore original size of fullscreened window on unfullscreen
- Explicitly setting `ClientSide` decorations will result in `ServerSide` ones being destroyed
- Requesting `ServerSide` decorations in `set_decorate` will now fallback to `ClientSide`
  if the former are not available
- Requesting `None` decorations if `ServerSide` are presented will result in setting
  `ClientSide` decorations with hidden frame
- `ConceptFrame` will use `Disabled` style for maximized button for non-resizeable frame
- `ConceptFrame` will create subsurfaces for client side decorations only if a frame is visible
- `Window` will restore original size after being tiled

#### Bugfixes

- Toggling between `ServerSide` and `None` decorations raising protocol error
- Precision in a rate of key repeat events
- `ThemeManager` not being clone-able even if it was stated in docs
- Repeat rate not being disabled when receiving zero for `rate`  in `wl_keyboard.repeat_info`

## 0.10.0 -- 2020-07-10

#### Breaking Changes

- `create_surface` and `create_surface_with_scale_callback` now return `Attached<WlSurface>`
- Update `wayland-client` to `0.27`

#### Changes

- `andrew` is updated to `0.3`.

#### Bugfixes

- seat: Seats with an empty name are no longer filtered out

## 0.9.1 -- 2020-05-03

#### Additions

- keyboard: Update the keysyms list with new symbols
- Add primary selection helpers, which are included as part of default `Environment`.

#### Changes

- surfaces: dpi-aware surface will no longer believe their DPI factor reverts to 1 when they
  become hidden.

#### BugFixes

- keyboard: Remove the unnecessary type parameter of `map_keyboard`

## 0.9.0 -- 2020-04-22

#### Breaking Changes

- `AutoThemer` is removed as it is no longer necessary with `wayland-cursor` 0.26
- `calloop` is updated to 0.6, and the adapters are modified in consequence

#### Additions

- Add `clone_seat_data()` method as a shorthand to get `SeatData`

#### Bugfixes

- Surface lock held across scale factor callback deadlocks scale factor API.

## 0.8.1 -- 2020-04-09

#### Additions

- Add `listen_for_outputs()` which calls a provided callback on creation/removal of outputs.
- Add an `OutputHandling` trait making `listen_for_outputs()` available on `Environment`.
- Introduce the `calloop` cargo feature, enabled by default, controlling the support for the calloop event
  loop
- Introduce the `frames` cargo feature, enabled by default, controlling the existence of provided `Frame`
  implementations (currently `ConceptFrame`) and the dependency on `andrew`

## 0.8.0 -- 2020-02-27

#### Breaking Changes

- `Frame` configuration is now done through a `Frame::Config` associated type and the `Theme` trait is removed.
- Merge `Frame::set_active` and `Frame::set_maximized` into `Frame::set_states`

#### Additions

- HiDPI scaling for decorations

#### Bugfixes

- HiDPI cursor icon position
- Fix graphical glitches in `ConceptFrame` decoration drawing
- Black pixel on left-bottom corner on CSD
- Remove a deadlock when trying to access the seat data from within the seat callback

## 0.7.0 -- 2020-02-07

#### Breaking changes

- Upgrade to `wayland-client` 0.25. This changes the prototype of most callbacks by
  adding the `DispatchData` mechanism for state sharing
- Re-structure the lib API around the new `Environment` type as an entry point (breaks a lot of things).
  This makes the crate follow a monolithic-modular structure centered on this type.
- `keyboard` is now a submodule of `seat`
- `keyboard` key repetition is now handled as a calloop event source
- `pointer` is now a submodule of `seat`
- The initialization of `pointer` theming utilities now require a `ThemeSpec` argument
  instead of just a theme name, allowing control over the size of the cursors as well
- Pointer theming utilities can no longer be shared across threads, as it was racy.
- `Window` now tracks new seats automatically (the `new_seat` method is removed)
- `Window` can no longer be shared across threads, as it was racy.
- Decorations management is now handled with the `Decorations` enum, for full control to clients.

#### Additions

- The `pointer` theming will now read the `XCURSOR_THEME` and `XCURSOR_SIZE` environment
  variables to figure the default theme
- `pointer` theming utilities now handle HiDPI monitors
- SCTK now uses the `log` crate to log its warning and error messages
- Data offers `ReadPipe`scan be inserted in a calloop event loop as an event source
- The `WaylandSource` wrapper allows a `wayland-client` `EventQueue` to be inserted into
  a calloop event source.

## 0.6.4 -- 2019-08-27

#### Bugfixes

- Keyboard input breaking when `LC_ALL`, `LC_CTYPE` or `LANG` are set to an empty string
- UTF8 interpretation no longer stops working if loading the compose table failed

## 0.6.3 -- 2019-06-29

- Keyboard: fix extra key repeat when using also releasing a modifier

## 0.6.2 -- 2019-06-13

- Update `Nix` to 0.14

## 0.6.1 -- 2019-04-07

- Additional theming capability on `ConceptFrame` via the `Theme` trait:
 optional methods `get_<button-name>_button_icon_color` allows the stroke
 color on the buttons to be customized beyond what the secondary color allows.
 Button color methods now affect the `ConceptFrame`'s fill behind the buttons.
- Fix the firing of `Configure` events in window abstraction.

## 0.6.0 -- 2019-02-18

#### Breaking changes

- Upgrade to `wayland-client` version 0.23

## 0.5.0 -- 2019-02-05

#### Breaking changes

- Update the crate to `wayland-client` version 0.22
- Window: `set_title()` now requires a manual `refresh()` for the change to take effect

#### Bugfixes

- Keyboard: fix system repeat rate as repeats per second rather then millisecond delay between repeats
- Surface: fix panic in `compute_dpi_factor()` by only computing the dpi factor on surfaces known to the OutputMgr

## 0.4.4 -- 2018-12-27

- Shell: expose shell interface and add `create_shell_surface` to `Environment`.
- Fix build failure on big endian targets

## 0.4.3 -- 2018-12-03

- Update dependencies: rand, memmap, nix and image
- Surface: `create_surface` and `get_dpi_factor` utilities for creating dpi aware surfaces.

## 0.4.2 -- 2018-11-14

- Fix compilation on BSD systems

## 0.4.1 -- 2018-11-06

- Window: always request server-side decorations if available, otherwise ther compositor never configures us
- keyboard: only compute utf8 value on keypress, not key release. Otherwise it confuses `xkb_compose`.

## 0.4.0 -- 2018-10-09

- BasicFrame: Display the title of the window in the window header
- Pass `set_selection()` `Option<DataSource>` and `AutoThemer::init()` `Proxy<WlShm>` by reference
- Window: add `set_theme()` function which takes an object implementing the trait `Theme` to adjust the look of window decorations
- Window: add new `ConceptFrame` which provides an alternative to the `BasicFrame` window decorations
- MemPool: add `mmap` method
- **[Breaking]** Keyboard: remove `modifiers` field from `keyboard::Event::Enter`, `keyboard::Event::Key` and `keyboard::KeyRepeatEvent`
- **[Breaking]** Keyboard: add `keyboard::Event::Modifiers`
- **[Breaking]** Upgrade to wayland-rs 0.21
- Keyboard: end key repetition when the keyboard loses focus

## 0.3.0 -- 2018-08-17

- Window: the minimum window width is set to 2 pixels to circumvent a bug in mutter - https://gitlab.gnome.org/GNOME/mutter/issues/259
- **[Breaking]** MemPool: MemPool now requires an implementation to be called when the pool becomes free
- **[Breaking]** DoubleMemPool: DoubleMemPool now requires an implementation to be called when one of its pools becomes free
- **[Breaking]** DoubleMemPool: `swap()` is removed as `pool()` will now automatically track and return any free pools avaliable or return None
- Keyboard: add key repetition with 'map_keyboard_auto_with_repeat' and 'map_keyboard_rmlvo_with_repeat'
- Window: add `init_with_decorations` to allow the use of server-side decorations

## 0.2.6 -- 2018-07-14

Big thanks to @trimental for improving the visual look of the window decorations:

- BasicFrame: remove side and bottom border decorations
- BasicFrame: round window corners

## 0.2.5 -- 2018-07-10

- Keyboard: try to load `libxkbcommon.so.0` as well to improve compatibility

## 0.2.4 -- 2018-06-26

- Window: notify the compositor of our dimensions to avoid placement glitches

## 0.2.3 -- 2018-06-08

- Update `nix` dependency to be fix build on FreeBSD (even if we can't run)

## 0.2.2 -- 2018-06-08

- BasicFrame: don't desync the subsurface from the main one. This avoids
  graphical glitches where the borders are not drawn exactly the same size
  as the contents.
- Window: add `set_resizable`, (minor **breaking change** of the `Frame` trait by
  adding a new method)

## 0.2.1 -- 2018-05-03

- Add `DoubleMemPool` for double buffering, and use it to
  improve the drawing performance of `BasicFrame`.

## 0.2.0 -- 2018-04-29

- *Breaking* OutputMgr: expose wl_output global id

## 0.1.0 -- 2018-04-26

Initial version, including:

- basic environment manager
- keyboard keymap handling
- basic window decoration
