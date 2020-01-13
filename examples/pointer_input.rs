extern crate byteorder;
extern crate smithay_client_toolkit as sctk;

use std::cmp::min;
use std::io::{BufWriter, Seek, SeekFrom, Write};

use byteorder::{NativeEndian, WriteBytesExt};

use sctk::reexports::client::protocol::{wl_pointer, wl_shm, wl_surface};
use sctk::reexports::client::Display;
use sctk::shm::MemPool;
use sctk::window::{ConceptFrame, Event as WEvent};

#[derive(Debug)]
enum NextAction {
    Refresh,
    Redraw,
    Exit,
}

struct WindowConfig {
    width: u32,
    height: u32,
    dpi_scale: i32,
    next_action: Option<NextAction>,
}

impl WindowConfig {
    pub fn new() -> Self {
        WindowConfig {
            width: 320,
            height: 240,
            dpi_scale: 1,
            next_action: None,
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (
            self.width * self.dpi_scale as u32,
            self.height * self.dpi_scale as u32,
        )
    }

    pub fn handle_action(&mut self, new_action: NextAction) {
        let replace = match (&self.next_action, &new_action) {
            (&None, _)
            | (&Some(NextAction::Refresh), _)
            | (&Some(NextAction::Redraw), &NextAction::Exit) => true,
            _ => false,
        };
        if replace {
            self.next_action = Some(new_action);
        }
    }
}

sctk::default_environment!(CompInfo, fields = [], singles = [], multis = []);

fn main() {
    /*
     * Initial setup
     */
    let display = match Display::connect_to_env() {
        Ok(d) => d,
        Err(e) => {
            panic!("Unable to connect to a Wayland compositor: {}", e);
        }
    };

    let mut queue = display.create_event_queue();

    let env = sctk::init_default_environment!(
        CompInfo,
        &(*display).clone().attach(queue.token()),
        fields = []
    );

    // two roundtrips to init the environment
    queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .unwrap();
    queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .unwrap();

    /*
     * Init wayland objects
     */

    let mut window_config = WindowConfig::new();

    let surface = env.create_surface_with_scale_callback(move |dpi, surface, mut dispatch_data| {
        let config = dispatch_data.get::<WindowConfig>().unwrap();
        surface.set_buffer_scale(dpi);
        config.dpi_scale = dpi;
        config.handle_action(NextAction::Redraw);
    });

    let mut window = env
        .create_window::<ConceptFrame, _>(
            surface,
            window_config.dimensions(),
            move |event, mut dispatch_data| {
                let mut config = dispatch_data.get::<WindowConfig>().unwrap();
                if let WEvent::Configure { new_size, .. } = event {
                    if let Some((width, height)) = new_size {
                        config.width = width;
                        config.height = height;
                    }
                }
                let next_action = match event {
                    WEvent::Refresh => NextAction::Refresh,
                    WEvent::Configure { .. } => NextAction::Redraw,
                    WEvent::Close => NextAction::Exit,
                };
                config.handle_action(next_action);
            },
        )
        .expect("Failed to create a window !");

    let mut pools = env
        .create_double_pool(|_| {})
        .expect("Failed to create a memory pool !");

    /*
     * Pointer initialization
     */

    let main_surface = window.surface().clone();

    let mut seats = Vec::<(String, Option<wl_pointer::WlPointer>)>::new();
    let _seat_listener = env.listen_for_seats(move |seat, seat_data, _| {
        // find the seat in the vec of seats, or insert it if it is unknown
        let idx = seats.iter().position(|(name, _)| name == &seat_data.name);
        let idx = idx.unwrap_or_else(|| {
            seats.push((seat_data.name.clone(), None));
            seats.len() - 1
        });

        let (_, ref mut opt_ptr) = &mut seats[idx];
        // we should map a keyboard if the seat has the capability & is not defunct
        if seat_data.has_keyboard && !seat_data.defunct {
            if opt_ptr.is_none() {
                // we should initalize a keyboard
                let seat_name = seat_data.name.clone();
                let pointer = seat.get_pointer();
                let surface = main_surface.clone();
                pointer.quick_assign(move |_, event, _| {
                    print_pointer_event(event, &seat_name, &surface)
                });
                *opt_ptr = Some((**pointer).clone());
            }
        } else {
            if let Some(ptr) = opt_ptr.take() {
                // the pointer has been removed, cleanup
                ptr.release();
            }
        }
    });

    if !env.get_shell().unwrap().needs_configure() {
        window_config.handle_action(NextAction::Redraw);
    }

    loop {
        let next_action = window_config.next_action.take();
        println!("{:?}", next_action);
        match next_action {
            Some(NextAction::Exit) => break,
            Some(NextAction::Refresh) => {
                window.refresh();
                window.surface().commit();
            }
            Some(NextAction::Redraw) => {
                window.refresh();
                if let Some(pool) = pools.pool() {
                    redraw(pool, window.surface(), window_config.dimensions())
                        .expect("Failed to draw")
                }
            }
            None => {}
        }

        queue.dispatch(&mut window_config, |_, _, _| {}).unwrap();
    }
}

fn redraw(
    pool: &mut MemPool,
    surface: &wl_surface::WlSurface,
    (buf_x, buf_y): (u32, u32),
) -> Result<(), ::std::io::Error> {
    // resize the pool if relevant
    pool.resize((4 * buf_x * buf_y) as usize)
        .expect("Failed to resize the memory pool.");
    // write the contents, a nice color gradient =)
    pool.seek(SeekFrom::Start(0))?;
    {
        let mut writer = BufWriter::new(&mut *pool);
        for i in 0..(buf_x * buf_y) {
            let x = (i % buf_x) as u32;
            let y = (i / buf_x) as u32;
            let r: u32 = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let g: u32 = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let b: u32 = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
            writer.write_u32::<NativeEndian>((0xFF << 24) + (r << 16) + (g << 8) + b)?;
        }
        writer.flush()?;
    }
    // get a buffer and attach it
    let new_buffer = pool.buffer(
        0,
        buf_x as i32,
        buf_y as i32,
        4 * buf_x as i32,
        wl_shm::Format::Argb8888,
    );
    surface.attach(Some(&new_buffer), 0, 0);
    surface.commit();
    Ok(())
}

fn print_pointer_event(
    event: wl_pointer::Event,
    seat_name: &str,
    main_surface: &wl_surface::WlSurface,
) {
    match event {
        wl_pointer::Event::Enter {
            surface,
            surface_x,
            surface_y,
            ..
        } => {
            if main_surface == &surface {
                println!(
                    "Pointer of seat '{}' entered at ({}, {})",
                    seat_name, surface_x, surface_y
                );
            }
        }
        wl_pointer::Event::Leave { surface, .. } => {
            if main_surface == &surface {
                println!("Pointer of seat '{}' left", seat_name);
            }
        }
        wl_pointer::Event::Button { button, state, .. } => {
            println!(
                "Button {:?} of seat '{}' was {:?}",
                button, seat_name, state
            );
        }
        wl_pointer::Event::Motion {
            surface_x,
            surface_y,
            ..
        } => println!(
            "Pointer motion to ({}, {}) on seat '{}'",
            surface_x, surface_y, seat_name
        ),
        _ => {}
    }
}
