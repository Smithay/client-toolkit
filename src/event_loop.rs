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
        let fd = queue.get_connection_fd();
        WaylandSource {
            queue: Rc::new(RefCell::new(queue)),
            fd: calloop::generic::SourceRawFd(fd),
        }
    }
}

impl EventSource for WaylandSource {
    type Event = io::Result<u32>;

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
    F: FnMut(io::Result<u32>, &mut Data),
    Data: 'static,
{
    fn ready(&mut self, _: Option<&mio::event::Event>, data: &mut Data) {
        let mut queue = self.queue.borrow_mut();
        // in case of readiness of the wayland socket we do the following in a loop, until nothing
        // more can be read:
        let mut dispatched = 0;
        loop {
            // 1. dispatch any pending event in the queue
            let ret = queue.dispatch_pending(data, |evt, object, _| {
                panic!(
                    "[SCTK] Orphan event reached the event queue: {}@{} -> {}",
                    evt.interface,
                    object.as_ref().id(),
                    evt.name
                );
            });
            match ret {
                Ok(n) => {
                    dispatched += n;
                }
                Err(e) => {
                    // in case of error, forward it and fast-exit
                    (self.callback)(Err(e), data);
                    return;
                }
            }
            // 2. flush the socket to sent requests generated in response of the dispatched events
            if let Err(e) = queue.flush() {
                if e.kind() != io::ErrorKind::WouldBlock {
                    // in case of error, forward it and fast-exit
                    (self.callback)(Err(e), data);
                    return;
                }
                // WouldBlock error means the compositor could not process all our messages
                // quickly. Either it is slowed down or we are a spammer.
                // Should not really happen, if it does we do nothing and will flush again later
            }
            // 3. read events from the socket
            if let Some(guard) = queue.prepare_read() {
                // might be None if some other thread read events before us, concurently
                if let Err(e) = guard.read_events() {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        // in case of error, forward it and fast-exit
                        (self.callback)(Err(e), data);
                        return;
                    } else {
                        // There was nothing to read and all our events are
                        // dispatched, let's stop here
                        (self.callback)(Ok(dispatched), data);
                        break;
                    }
                }
            }
        }
    }
}
