# Change Log

## Unreleased

- BasicFrame: don't desync the subsurface from the main one. This avoids
  graphical glitches where the borders are not drawn exactly the same size
  as the contents.
- Window: add `set_resizable`, **breaking change** of the `Frame` trait.

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
