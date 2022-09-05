/// An error that may occur when creating objects using a global.
#[derive(Debug, thiserror::Error)]
pub enum GlobalError {
    /// A compositor global was not available
    #[error("the '{0}' global was not available")]
    MissingGlobal(&'static str),

    /// A compositor global was available, but did not support the given minimum version
    #[error("the '{name}' global does not support interface version {required} (using version {available})")]
    InvalidVersion { name: &'static str, required: u32, available: u32 },

    /// This is likely a programming error due to a missing entry in the registry_handlers! macro.
    #[error("global enumeration is not finished or missing ProvidesRegistryState delegation")]
    NotReady,
}
