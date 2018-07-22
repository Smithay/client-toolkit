[![crates.io](http://meritbadge.herokuapp.com/smithay-client-toolkit)](https://crates.io/crates/smithay-client-toolkit)
[![Build Status](https://travis-ci.org/Smithay/client-toolkit.svg?branch=master)](https://travis-ci.org/Smithay/client-toolkit)


# Smithay's Client Toolkit

This crate is a toolkit for writing wayland clients in rust, on top of [wayland-client](https://crates.io/crates/wayland-client).

Currently a work in progress, it currently provides the following utilities:

- Automatic binding of general wayland globals (`wl_compositor`, `wl_shm`, etc..)
- Abstraction to create windows (aka toplevel surfaces), abstracting the interaction
  with the shell (`xdg_shell` or `wl_shell`) and the drawing of decorations
- Wrapper for `wl_keyboard` for automatic keymap interpretation using `libxkbcommon.so`.

## Documentation

The documentation for the master branch is [available online](https://smithay.github.io/client-toolkit/).

The documentation for the releases can be found on [docs.rs](https://docs.rs/smithay-client-toolkit).

## Requirements

Requires at least rust 1.22 to be used (using bitflags 1.0 for associated constants), and version 1.12 of the
wayland system libraries.
