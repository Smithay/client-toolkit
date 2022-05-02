use wayland_backend::client::InvalidId;

/// An error that may occur when creating objects using a global.
#[derive(Debug, thiserror::Error)]
pub enum GlobalError {
    /// Some compositor globals were not available
    ///
    /// The value of this variant should contain the names of the missing globals.
    #[error("the following globals are not available: {0:?}")]
    MissingGlobals(&'static [&'static str]),

    /// An invalid id was acted upon
    ///
    /// This likely means a request was sent to a dead protocol object.
    #[error(transparent)]
    Id(#[from] InvalidId),
}
