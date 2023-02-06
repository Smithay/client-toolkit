use smithay_client_toolkit::{
    delegate_registry, delegate_seat,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
};
use wayland_client::{globals::registry_queue_init, protocol::wl_seat, Connection, QueueHandle};

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let mut list_seats = ListSeats {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
    };

    event_queue.blocking_dispatch(&mut list_seats).unwrap();

    println!("Available seats:");

    for seat in list_seats.seat_state.seats() {
        if let Some(info) = list_seats.seat_state.info(&seat) {
            println!("{info}");
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

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        // Not applicable
    }

    fn new_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
        // Not applicable
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
        // Not applicable
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        // Not applicable
    }
}

delegate_seat!(ListSeats);

delegate_registry!(ListSeats);

impl ProvidesRegistryState for ListSeats {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(SeatState);
}
