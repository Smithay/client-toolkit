# Change Log

## Unreleased

- Keyboard: fix system repeat rate as repeats per second rather then millisecond delay between repeats
- Surface: fix panic in `compute_dpi_factor()` by only computing the dpi factor on surfaces known to the OutputMgr
- Window: `set_title()` now requires a manual `refresh()` for the change to take effect

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
