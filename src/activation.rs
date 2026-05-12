use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::{wl_seat, wl_surface},
    Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::xdg::activation::v1::client::{xdg_activation_token_v1, xdg_activation_v1};

use crate::{
    error::GlobalError,
    globals::{GlobalData, ProvidesBoundGlobal},
};

/// Minimal implementation of [`RequestDataExt`].
///
/// Use a custom type implementing [`RequestDataExt`] to store more data with a token request
/// e.g. to identify which request produced which token.
#[derive(Debug, Clone)]
pub struct RequestData<U> {
    /// App_id of the application requesting the token, if applicable
    pub app_id: Option<String>,
    /// Seat and serial of the window requesting the token, if applicable.
    ///
    /// *Warning*: Many compositors will issue invalid tokens for requests without
    /// recent serials. There is no way to detect this from the client-side.
    pub seat_and_serial: Option<(wl_seat::WlSeat, u32)>,
    /// Surface of the window requesting the token, if applicable.
    ///
    /// *Warning*: Many compositors will issue invalid tokens for requests from
    /// unfocused surfaces. There is no way to detect this from the client-side.
    pub surface: Option<wl_surface::WlSurface>,

    pub udata: U,
}

/// Handler for xdg-activation
pub trait ActivationHandler: Sized {
    /// Data type used for requesting activation tokens
    // TODO: Default to `()` if default associated types are ever supported
    type RequestUdata;
    /// A token was issued for a previous request with `data`.
    fn new_token(&mut self, token: String, data: &RequestData<Self::RequestUdata>);
}

/// State for xdg-activation
#[derive(Debug)]
pub struct ActivationState {
    xdg_activation: xdg_activation_v1::XdgActivationV1,
}

impl ActivationState {
    /// Bind the `xdg-activation` global
    pub fn bind<State>(
        globals: &GlobalList,
        qh: &QueueHandle<State>,
    ) -> Result<ActivationState, BindError>
    where
        State: Dispatch<xdg_activation_v1::XdgActivationV1, GlobalData, State> + 'static,
    {
        let xdg_activation = globals.bind(qh, 1..=1, GlobalData)?;
        Ok(ActivationState { xdg_activation })
    }

    /// Activate a surface with the provided token.
    pub fn activate<D>(&self, surface: &wl_surface::WlSurface, token: String) {
        self.xdg_activation.activate(token, surface)
    }

    /// Request a token for surface activation.
    ///
    /// To attach custom data to the request implement [`RequestDataExt`] on a custom type
    /// and use [`Self::request_token_with_data`] instead.
    pub fn request_token<D, U>(&self, qh: &QueueHandle<D>, request_data: RequestData<U>)
    where
        D: ActivationHandler<RequestUdata = U>,
        D: Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, RequestData<U>> + 'static,
        U: Send + Sync + 'static,
    {
        let token = self.xdg_activation.get_activation_token(qh, request_data);
        let data = token.data::<RequestData<U>>().unwrap();
        if let Some(app_id) = &data.app_id {
            token.set_app_id(String::from(app_id));
        }
        if let Some((seat, serial)) = &data.seat_and_serial {
            token.set_serial(*serial, seat);
        }
        if let Some(surface) = &data.surface {
            token.set_surface(surface);
        }
        token.commit();
    }
}

impl<D> Dispatch<xdg_activation_v1::XdgActivationV1, GlobalData, D> for ActivationState
where
    D: Dispatch<xdg_activation_v1::XdgActivationV1, GlobalData> + ActivationHandler,
{
    fn event(
        _: &mut D,
        _: &xdg_activation_v1::XdgActivationV1,
        _: <xdg_activation_v1::XdgActivationV1 as Proxy>::Event,
        _: &GlobalData,
        _: &wayland_client::Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("xdg_activation_v1 has no events");
    }
}

impl ProvidesBoundGlobal<xdg_activation_v1::XdgActivationV1, 1> for ActivationState {
    fn bound_global(&self) -> Result<xdg_activation_v1::XdgActivationV1, GlobalError> {
        Ok(self.xdg_activation.clone())
    }
}

impl<D, U> Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, RequestData<U>, D>
    for ActivationState
where
    D: Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, RequestData<U>>
        + ActivationHandler<RequestUdata = U>,
{
    fn event(
        state: &mut D,
        _proxy: &xdg_activation_token_v1::XdgActivationTokenV1,
        event: <xdg_activation_token_v1::XdgActivationTokenV1 as Proxy>::Event,
        data: &RequestData<U>,
        _conn: &wayland_client::Connection,
        _qhandle: &QueueHandle<D>,
    ) {
        if let xdg_activation_token_v1::Event::Done { token } = event {
            state.new_token(token, data);
        }
    }
}

#[macro_export]
macro_rules! delegate_activation {
   ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
       $crate::delegate_activation!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty, ());
   };
   ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, $data: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1: $crate::globals::GlobalData
            ] => $crate::activation::ActivationState
        );
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                $crate::reexports::protocols::xdg::activation::v1::client::xdg_activation_token_v1::XdgActivationTokenV1: $crate::activation::RequestData<$data>
            ] => $crate::activation::ActivationState
        );
    };
}
