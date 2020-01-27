use std::{cell::RefCell, io, rc::Rc, sync::Arc};

use calloop::{mio, EventDispatcher, EventSource};

use wayland_client::EventQueue;

/// An adapter to insert a Wayland `EventQueue` into a calloop event loop
///
/// This is a struct that implements `calloop::EventSource`. Its
/// `Event` type is a `Result<u32, std::io::Error>`. Whenever a batch
/// of messages is successfully dispatched, you'll be notified with
/// an `Ok(n)` where `n` is the number of dispatched events.
///
/// In case of error, you'll be notified of the error and dispatching is aborted,
/// as an error in this case is always fatal to the Wayland connection.
///
/// # Usage
///
/// A typical workflow would be to manually handle your `EventQueue` during the
/// initialization of your app, and once this is done insert the `EventQueue`
/// into your event loop and let it drive your app.
pub struct WaylandSource {
    queue: Rc<RefCell<EventQueue>>,
    fd: calloop::generic::SourceRawFd,
}

impl WaylandSource {
    /// Wrap an `EventQueue` as a `WaylandSource`.
    pub fn new(queue: EventQueue) -> WaylandSource {
        let fd = queue.display().get_connection_fd();
        WaylandSource {
            queue: Rc::new(RefCell::new(queue)),
            fd: calloop::generic::SourceRawFd(fd),
        }
    }
}

/// An error that can occur during the dispatching of the event queue
///
/// These are transmitted to your callback for the `WaylandSource`. Receiving
/// any of them is very likely fatal to your Wayland connection.
#[derive(Debug)]
pub enum DispatchError {
    /// A protocol error was triggered
    ///
    /// This means something your app did was considered a violation of the
    /// protocol by the server. The inner error contains details.
    Protocol(wayland_client::ProtocolError),
    /// An IO error occured
    ///
    /// This very likely means your connection to the server was unexpectedly lost.
    Io(io::Error),
    /// An orphan event was received during the dispatch
    ///
    /// While `wayland-client` supports the handling of events from the fallback
    /// closure during dispatching, this adapter does not. If you want to handle
    /// them you cannot use the `WaylandSource`.
    OrphanEvent {
        /// Interface of the object that received the event
        interface: String,
        /// ID of the object that received the event
        id: u32,
        /// Name of the event
        event_name: String,
    },
}

impl std::error::Error for DispatchError {}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::Io(e) => write!(f, "IO error: {}", e),
            DispatchError::Protocol(e) => write!(f, "Protocol error: {}", e),
            DispatchError::OrphanEvent {
                interface,
                id,
                event_name,
            } => write!(f, "Orphan event on {}@{}: {}", interface, id, event_name),
        }
    }
}

impl EventSource for WaylandSource {
    type Event = Result<u32, DispatchError>;

    fn as_mio_source(&mut self) -> Option<&mut dyn mio::event::Source> {
        Some(&mut self.fd)
    }

    fn make_dispatcher<Data: 'static, F: FnMut(Self::Event, &mut Data) + 'static>(
        &mut self,
        callback: F,
        _: &Arc<mio::Waker>,
    ) -> Rc<RefCell<dyn EventDispatcher<Data>>> {
        Rc::new(RefCell::new(WaylandDispatcher {
            queue: self.queue.clone(),
            callback,
        }))
    }
}

struct WaylandDispatcher<F> {
    queue: Rc<RefCell<EventQueue>>,
    callback: F,
}

impl<Data, F> EventDispatcher<Data> for WaylandDispatcher<F>
where
    F: FnMut(Result<u32, DispatchError>, &mut Data),
    Data: 'static,
{
    fn ready(&mut self, _: Option<&mio::event::Event>, data: &mut Data) {
        let mut queue = self.queue.borrow_mut();
        // in case of readiness of the wayland socket we do the following in a loop, until nothing
        // more can be read:
        let mut dispatched = 0;
        loop {
            // 1. read events from the socket if any are available
            if let Some(guard) = queue.prepare_read() {
                // might be None if some other thread read events before us, concurently
                if let Err(e) = guard.read_events() {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        // in case of error, forward it and fast-exit
                        if let Some(perr) = queue.display().protocol_error() {
                            (self.callback)(Err(DispatchError::Protocol(perr)), data);
                        } else {
                            (self.callback)(Err(DispatchError::Io(e)), data);
                        }
                        return;
                    }
                }
            }
            // 2. dispatch any pending event in the queue
            // Abort when receiving an orphan even, this adapter
            // does not support them.
            let mut orphan = None;
            let ret = queue.dispatch_pending(data, |evt, object, _| {
                // only store & report the first orphan event
                if orphan.is_none() {
                    orphan = Some(DispatchError::OrphanEvent {
                        interface: evt.interface.into(),
                        id: object.as_ref().id(),
                        event_name: evt.name.into(),
                    });
                }
            });
            if let Some(orphan) = orphan {
                (self.callback)(Err(orphan), data);
                return;
            }
            match ret {
                Ok(0) => {
                    // no events were dispatched even after reading the socket,
                    // nothing more to do, stop here
                    (self.callback)(Ok(dispatched), data);
                    break;
                }
                Ok(n) => {
                    dispatched += n;
                }
                Err(e) => {
                    // in case of error, forward it and fast-exit
                    if let Some(perr) = queue.display().protocol_error() {
                        (self.callback)(Err(DispatchError::Protocol(perr)), data);
                    } else {
                        (self.callback)(Err(DispatchError::Io(e)), data);
                    }
                    return;
                }
            }
        }
        // 3. Once dispatching is finished, flush the responses to the compositor
        if let Err(e) = queue.display().flush() {
            if e.kind() != io::ErrorKind::WouldBlock {
                // in case of error, forward it and fast-exit
                if let Some(perr) = queue.display().protocol_error() {
                    (self.callback)(Err(DispatchError::Protocol(perr)), data);
                } else {
                    (self.callback)(Err(DispatchError::Io(e)), data);
                }
                return;
            }
            // WouldBlock error means the compositor could not process all our messages
            // quickly. Either it is slowed down or we are a spammer.
            // Should not really happen, if it does we do nothing and will flush again later
        }
    }
}
