use std::marker::PhantomData;

use smithay_client_toolkit::{
    delegate_registry,
    registry::RegistryHandle,
    seat::{Capability, SeatData, SeatDispatch, SeatHandler, SeatState},
};
use wayland_client::{
    delegate_dispatch,
    protocol::{wl_keyboard, wl_seat, wl_surface},
    Connection, ConnectionHandle, QueueHandle,
};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();
    let display = conn.handle().display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let registry = display.get_registry(&mut conn.handle(), &qh, ()).unwrap();

    let mut list_seats = ListSeats {
        inner: InnerApp,

        registry_handle: RegistryHandle::new(registry),
        seat_state: SeatState::new(),
    };

    event_queue.blocking_dispatch(&mut list_seats).unwrap();
    event_queue.blocking_dispatch(&mut list_seats).unwrap();

    println!("Available seats:");

    for seat in list_seats.seat_state.seats() {
        if let Some(info) = list_seats.seat_state.info(&seat) {
            println!("{}", info);
        }
    }
}

struct ListSeats {
    inner: InnerApp,

    seat_state: SeatState,
    registry_handle: RegistryHandle,
}

struct InnerApp;

impl SeatHandler<ListSeats> for InnerApp {
    fn new_seat(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: wl_seat::WlSeat,
    ) {
        // Not applicable
    }

    fn new_capability(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
        // Not applicable
    }

    fn remove_capability(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
        // Not applicable
    }

    fn remove_seat(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: wl_seat::WlSeat,
    ) {
        // Not applicable
    }

    // Functions not needed for the tests

    fn keyboard_focus(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
    ) {
        unreachable!()
    }

    fn keyboard_release_focus(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
    ) {
        unreachable!()
    }

    fn keyboard_press_key(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: u32,
    ) {
        unreachable!()
    }

    fn keyboard_release_key(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: u32,
    ) {
        unreachable!()
    }

    fn keyboard_update_modifiers(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: &wl_keyboard::WlKeyboard,
        // TODO: Other params
    ) {
        unreachable!()
    }

    fn keyboard_update_repeat_info(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: u32,
    ) {
        unreachable!()
    }

    fn pointer_focus(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: &wayland_client::protocol::wl_pointer::WlPointer,
        _: &wl_surface::WlSurface,
        _: (f64, f64),
    ) {
        unreachable!()
    }

    fn pointer_release_focus(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<ListSeats>,
        _: &mut SeatState,
        _: &wayland_client::protocol::wl_pointer::WlPointer,
        _: &wl_surface::WlSurface,
    ) {
        unreachable!()
    }
}

delegate_registry!(ListSeats:
    |app| {
        &mut app.registry_handle
    },
    handlers = [
        { &mut SeatDispatch(&mut app.seat_state, &mut app.inner, PhantomData) }
    ]
);

delegate_dispatch!(ListSeats: <UserData = SeatData> [wl_seat::WlSeat] => SeatDispatch<'_, ListSeats, InnerApp> ; |app| {
    &mut SeatDispatch(&mut app.seat_state, &mut app.inner, PhantomData)
});
