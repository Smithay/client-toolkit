use wayland_client::{Attached, Interface};

/// An utility for lazy-loading globals.
pub enum LazyGlobal<I: Interface> {
    Unknown,
    Seen { id: u32, version: u32 },
    Bound(Attached<I>),
}
