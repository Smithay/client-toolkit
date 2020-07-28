use std::io;

use calloop::{
    generic::{Fd, Generic},
    EventSource, InsertError, Interest, LoopHandle, Mode, Source,
};

use wayland_client::EventQueue;

/// An adapter to insert a Wayland `EventQueue` into a calloop event loop
///
/// This is a struct that implements `calloop::EventSource`. It generates an
/// event whenever events need to be dispatched. At this point your calloop callback
/// will be given access to the `EventQueue` and you should call `.dispatch_pending()`
/// and forward its return value, allowing you to handle orphan events as you prefer.
///
/// If you don't use orphan events, the `quick_insert` method will firectly
/// insert the source into a provided `LoopHandle` with an adapter which will panic
/// whenever an oprhan event is encountered.
pub struct WaylandSource {
    queue: EventQueue,
    fd: Generic<Fd>,
}

impl WaylandSource {
    /// Wrap an `EventQueue` as a `WaylandSource`.
    pub fn new(queue: EventQueue) -> WaylandSource {
        let fd = queue.display().get_connection_fd();
        WaylandSource { queue, fd: Generic::from_fd(fd, Interest::Readable, Mode::Level) }
    }

    /// Insert this source into given event loop with an adapter that panics on orphan events
    pub fn quick_insert<Data: 'static>(
        self,
        handle: LoopHandle<Data>,
    ) -> Result<Source<WaylandSource>, InsertError<WaylandSource>> {
        handle.insert_source(self, |(), queue, ddata| {
            queue.dispatch_pending(ddata, |event, object, _| {
                panic!(
                    "[calloop] Encountered an orphan event: {}@{} : {}",
                    event.interface,
                    object.as_ref().id(),
                    event.name
                );
            })
        })
    }

    /// Access the underlying event queue
    ///
    /// This method can be used if you need to access the underlying `EventQueue` while this
    /// `WaylandSource` is currently inserted in an event loop.
    ///
    /// Note that you should be careful when interacting with it if you invoke methods that
    /// interact with the wayland socket (such as `dispatch()` or `prepare_read()`). These may
    /// interefere with the proper waking up of this event source in the event loop.
    pub fn queue(&mut self) -> &mut EventQueue {
        &mut self.queue
    }
}

impl EventSource for WaylandSource {
    type Event = ();
    type Metadata = EventQueue;
    type Ret = std::io::Result<u32>;

    fn process_events<F>(
        &mut self,
        _: calloop::Readiness,
        _: calloop::Token,
        mut callback: F,
    ) -> std::io::Result<()>
    where
        F: FnMut((), &mut EventQueue) -> std::io::Result<u32>,
    {
        // in case of readiness of the wayland socket we do the following in a loop, until nothing
        // more can be read:
        loop {
            // 1. read events from the socket if any are available
            if let Some(guard) = self.queue.prepare_read() {
                // might be None if some other thread read events before us, concurently
                if let Err(e) = guard.read_events() {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        return Err(e);
                    }
                }
            }
            // 2. dispatch any pending event in the queue
            // propagate orphan events to the user
            let ret = callback((), &mut self.queue);
            match ret {
                Ok(0) => {
                    // no events were dispatched even after reading the socket,
                    // nothing more to do, stop here
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    // in case of error, forward it and fast-exit
                    return Err(e);
                }
            }
        }
        // 3. Once dispatching is finished, flush the responses to the compositor
        if let Err(e) = self.queue.display().flush() {
            if e.kind() != io::ErrorKind::WouldBlock {
                // in case of error, forward it and fast-exit
                return Err(e);
            }
            // WouldBlock error means the compositor could not process all our messages
            // quickly. Either it is slowed down or we are a spammer.
            // Should not really happen, if it does we do nothing and will flush again later
        }
        Ok(())
    }

    fn register(&mut self, poll: &mut calloop::Poll, token: calloop::Token) -> std::io::Result<()> {
        self.fd.register(poll, token)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token: calloop::Token,
    ) -> std::io::Result<()> {
        self.fd.reregister(poll, token)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> std::io::Result<()> {
        self.fd.unregister(poll)
    }
}
