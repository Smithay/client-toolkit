use crate::error::GlobalError;
use wayland_client::Proxy;

/// A trait implemented by types that provide access to capability globals.
///
/// The returned global must be fully compatible with the provided `API_COMPAT_VERSION` generic
/// argument.  For example:
///
/// - A global that binds to `wl_compositor` with maximum version 4 could implement
///   `ProvidesBoundGlobal<WlCompositor, 4>`, `ProvidesBoundGlobal<WlCompositor, 3>`,
///   `ProvidesBoundGlobal<WlCompositor, 2>`, and `ProvidesBoundGlobal<WlCompositor, 1>` because
///   versions 2-4 only add additional requests to the `wl_surface` API.
/// - A global that binds to `wl_compositor` with maximum version 5 may only implement
///   `ProvidesBoundGlobal<WlCompositor, 5>` because version 5 makes using `wl_surface::attach` with
///   a nonzero offset a protocol error.  A caller who is only aware of the version 4 API risks
///   causing these protocol errors if it uses surfaces created by such a global.
///
/// Changes that cause compatibility breaks include:
///
/// - Adding a new event to the global or to any object created by the global.
/// - Adding a new requirement to an existing request.
///
/// The resulting global may have a version lower than `API_COMPAT_VERSION` if, at runtime, the
/// compositor does not support the new version.  Clients should either be prepared to handle
/// earlier versions of the protocol or use [`ProvidesBoundGlobal::with_min_version`] to produce an
/// error in this case.
///
/// It is permitted to implement `ProvidesBoundGlobal` for versions that are higher than the
/// maximum version you bind.  When rustc gains the ability to constrain const parameters with
/// integer bounds (`where API_COMPAT_VERSION >= 5`), implementations of this trait should be
/// provided by specifying a lower bound for the compat version in order to avoid requiring version
/// updates be done in lock-step.
pub trait ProvidesBoundGlobal<I: Proxy, const API_COMPAT_VERSION: u32> {
    fn bound_global(&self) -> Result<I, GlobalError>;
    fn with_min_version(&self, version: u32) -> Result<I, GlobalError> {
        let proxy = self.bound_global()?;
        if proxy.version() < version {
            Err(GlobalError::InvalidVersion {
                name: I::interface().name,
                required: version,
                available: proxy.version(),
            })
        } else {
            Ok(proxy)
        }
    }
}

/// A struct used as the UserData field for globals bound by SCTK.
///
/// This is used instead of `()` to allow multiple `Dispatch` impls on the same object.
#[derive(Debug)]
pub struct GlobalData;
