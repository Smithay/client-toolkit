use std::{
    num::TryFromIntError,
    sync::{Arc, Weak},
    time::Duration,
};

use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::wl_seat::{self, WlSeat},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1, ext_idle_notifier_v1,
};

use crate::globals::{GlobalData, ProvidesBoundGlobal};

#[derive(Debug)]
pub struct IdleNotifier {
    ext_idle_notifier: ext_idle_notifier_v1::ExtIdleNotifierV1,
}

impl IdleNotifier {
    /// Binds the ext idle notification global, `ext_idle_notifier_v1`.
    ///
    /// # Errors
    ///
    /// This functino will return [`Err`] if the `ext_idle_notifier_v1` global is not available.
    pub fn bind<State>(
        globals: &GlobalList,
        qh: &QueueHandle<State>,
    ) -> Result<IdleNotifier, BindError>
    where
        State: Dispatch<ext_idle_notifier_v1::ExtIdleNotifierV1, GlobalData, State>
            + IdleNotifierHandler
            + 'static,
    {
        let ext_idle_notifier = globals.bind(qh, 1..=2, GlobalData)?;
        Ok(IdleNotifier { ext_idle_notifier })
    }

    /// Get a new nodification object which triggers when the provided seat is inactive.
    ///
    /// The `duration` parameter is respected only at millisecond precision.
    ///
    /// Returns an error when the duration can fit in to u32 milliseconds.
    pub fn get_idle_notification<State>(
        &self,
        qh: &QueueHandle<State>,
        timeout: Duration,
        seat: &wl_seat::WlSeat,
    ) -> Result<IdleNotification, TryFromIntError>
    where
        State: Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, IdleNotificationData>
            + 'static,
    {
        let freeze = qh.freeze();
        let timeout = timeout.as_millis().try_into()?;

        let inner = Arc::new_cyclic(|weak| {
            self.ext_idle_notifier.get_idle_notification(
                timeout,
                seat,
                qh,
                IdleNotificationData { inner: weak.clone() },
            );

            IdleNotificationInner {
                wl_seat: seat.clone(),
                respects_idle_inhibitors: RespectsIdleInhibitors::Yes,
            }
        });
        drop(freeze);

        Ok(IdleNotification(inner))
    }

    /// Get a new nodification object which triggers when the provided seat has not seen input for a
    /// certain amount if time. This ignores idle inhibitors.
    ///
    /// The `duration` parameter is respected only at millisecond precision.
    ///
    /// Returns an error when the duration can fit in to u32 milliseconds.
    pub fn get_input_idle_notification<State>(
        &self,
        qh: &QueueHandle<State>,
        timeout: Duration,
        seat: &wl_seat::WlSeat,
    ) -> Result<IdleNotification, TryFromIntError>
    where
        State: Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, IdleNotificationData>
            + 'static,
    {
        let freeze = qh.freeze();
        let timeout = timeout.as_millis().try_into()?;

        let inner = Arc::new_cyclic(|weak| {
            self.ext_idle_notifier.get_input_idle_notification(
                timeout,
                seat,
                qh,
                IdleNotificationData { inner: weak.clone() },
            );

            IdleNotificationInner {
                wl_seat: seat.clone(),
                respects_idle_inhibitors: RespectsIdleInhibitors::No,
            }
        });
        drop(freeze);

        Ok(IdleNotification(inner))
    }
}

#[derive(Debug, Clone)]
pub struct IdleNotification(Arc<IdleNotificationInner>);

impl PartialEq for IdleNotification {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for IdleNotification {}

impl IdleNotification {
    pub fn respects_idle(&self) -> RespectsIdleInhibitors {
        self.0.respects_idle_inhibitors
    }

    pub fn seat(&self) -> &WlSeat {
        &self.0.wl_seat
    }
}

/// Whether or not a given idle notifier respect idle inhibitors.
///
/// # See also
/// [`IdleNotifier::get_idle_notification`] - Construct a notification which respects idle inhibitors.
/// [`IdleNotifier::get_input_idle_notification`] - Construct a notification which does not.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RespectsIdleInhibitors {
    /// This is the case for idle notifications constructued through [`IdleNotifier::get_idle_notification`].
    Yes,
    /// For notifications constructed through [`IdleNotifier::get_input_idle_notification`].
    No,
}

/// Client-side idle notification state.
#[derive(Debug)]
pub struct IdleNotificationData {
    inner: Weak<IdleNotificationInner>,
}

#[derive(Debug)]
struct IdleNotificationInner {
    /// Which seat this was constructed for
    wl_seat: wl_seat::WlSeat,

    /// Whether this notifier ignores inhibitors
    respects_idle_inhibitors: RespectsIdleInhibitors,
}

impl IdleNotification {
    pub fn from_ext_idle_notifier(
        notifier: &ext_idle_notification_v1::ExtIdleNotificationV1,
    ) -> Option<IdleNotification> {
        notifier
            .data::<IdleNotificationData>()
            .and_then(|data| data.inner.upgrade())
            .map(IdleNotification)
    }
}

/// Handler for operations on a [`IdleNotification`]
pub trait IdleNotifierHandler: Sized {
    /// Sent when the seat goes idle
    fn idled(&mut self, conn: &Connection, qh: &QueueHandle<Self>, notifier: &IdleNotification);
    /// Sent when the seat is no longer idle
    fn resumed(&mut self, conn: &Connection, qh: &QueueHandle<Self>, notifier: &IdleNotification);
}

#[macro_export]
macro_rules! delegate_idle {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1: $crate::globals::GlobalData
        ] => $crate::idle::IdleNotifier);
        $crate::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::reexports::protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::ExtIdleNotificationV1: $crate::idle::IdleNotificationData
        ] => $crate::idle::IdleNotification);
    };

}

impl ProvidesBoundGlobal<ext_idle_notifier_v1::ExtIdleNotifierV1, 1> for IdleNotifier {
    fn bound_global(
        &self,
    ) -> Result<ext_idle_notifier_v1::ExtIdleNotifierV1, crate::error::GlobalError> {
        log::trace!(target:"sctk", "providing global v1");
        Ok(self.ext_idle_notifier.clone())
    }
}

impl ProvidesBoundGlobal<ext_idle_notifier_v1::ExtIdleNotifierV1, 2> for IdleNotifier {
    fn bound_global(
        &self,
    ) -> Result<ext_idle_notifier_v1::ExtIdleNotifierV1, crate::error::GlobalError> {
        log::trace!(target:"sctk", "providing global v2");
        Ok(self.ext_idle_notifier.clone())
    }
}

impl<D> Dispatch<ext_idle_notifier_v1::ExtIdleNotifierV1, GlobalData, D> for IdleNotifier
where
    D: Dispatch<ext_idle_notifier_v1::ExtIdleNotifierV1, GlobalData> + 'static,
{
    fn event(
        _: &mut D,
        _: &ext_idle_notifier_v1::ExtIdleNotifierV1,
        _: ext_idle_notifier_v1::Event,
        _: &GlobalData,
        _: &wayland_client::Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("ext_idle_notifier_v1 has no events");
    }
}

impl<D> Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, IdleNotificationData, D>
    for IdleNotification
where
    D: Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, IdleNotificationData>
        + IdleNotifierHandler
        + 'static,
{
    fn event(
        state: &mut D,
        proxy: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _data: &IdleNotificationData,
        conn: &wayland_client::Connection,
        qh: &QueueHandle<D>,
    ) {
        if let Some(notifier) = IdleNotification::from_ext_idle_notifier(proxy) {
            match event {
                ext_idle_notification_v1::Event::Idled => state.idled(conn, qh, &notifier),
                ext_idle_notification_v1::Event::Resumed => state.resumed(conn, qh, &notifier),
                _ => unreachable!(),
            }
        }
    }
}
