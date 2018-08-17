# Change Log

## Unreleased

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
