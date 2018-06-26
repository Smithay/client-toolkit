# Change Log

## Unreleased

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
