use smithay_client_toolkit::{
    delegate_registry,
    registry::{ProvidesRegistryState, RegistryState},
    seat::{Capability, SeatHandler, SeatState},
};
use wayland_client::{
    delegate_dispatch, protocol::wl_seat, Connection, ConnectionHandle, QueueHandle,
};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();
    let display = conn.handle().display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let registry = display.get_registry(&mut conn.handle(), &qh, ()).unwrap();

    let mut list_seats =
        ListSeats { registry_state: RegistryState::new(registry), seat_state: SeatState::new() };

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
    seat_state: SeatState,
    registry_state: RegistryState,
}

impl SeatHandler for ListSeats {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &mut ConnectionHandle, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        // Not applicable
    }

    fn new_capability(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
        // Not applicable
    }

    fn remove_capability(
        &mut self,
        _: &mut ConnectionHandle,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
        // Not applicable
    }

    fn remove_seat(&mut self, _: &mut ConnectionHandle, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        // Not applicable
    }
}

delegate_registry!(ListSeats: [
    SeatState,
]);

impl ProvidesRegistryState for ListSeats {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
}

delegate_dispatch!(ListSeats: [wl_seat::WlSeat] => SeatState);
