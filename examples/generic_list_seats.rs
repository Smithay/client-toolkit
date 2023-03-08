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
        _dummy: MyTest {},
    };

    event_queue.roundtrip(&mut list_seats).unwrap();

    println!("Available seats:");

    for seat in list_seats.seat_state.seats() {
        if let Some(info) = list_seats.seat_state.info(&seat) {
            println!("{info}");
        }
    }
}

pub trait Test {
    fn test() {
        println!("Test");
    }
}

pub struct MyTest {}

impl Test for MyTest {}

struct ListSeats<T: Test + 'static> {
    seat_state: SeatState,
    registry_state: RegistryState,
    _dummy: T,
}

impl<T: Test + 'static> SeatHandler for ListSeats<T> {
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

delegate_seat!(@<T: Test + 'static> ListSeats<T>);

delegate_registry!(@<T: Test + 'static> ListSeats<T>);

impl<T: Test + 'static> ProvidesRegistryState for ListSeats<T> {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(SeatState);
}
