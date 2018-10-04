extern crate byteorder;
extern crate smithay_client_toolkit as sctk;

use std::cmp::min;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use byteorder::{NativeEndian, WriteBytesExt};

use sctk::reexports::client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use sctk::reexports::client::protocol::wl_seat::RequestsTrait as SeatRequests;
use sctk::reexports::client::protocol::wl_surface::RequestsTrait as SurfaceRequests;
use sctk::reexports::client::protocol::{wl_pointer, wl_shm, wl_surface};
use sctk::reexports::client::{Display, Proxy};
use sctk::utils::{DoubleMemPool, MemPool};
use sctk::window::{ConceptFrame, Event as WEvent, Window};
use sctk::Environment;

fn main() {
    let (display, mut event_queue) = Display::connect_to_env().unwrap();
    let env = Environment::from_display(&*display, &mut event_queue).unwrap();

    /*
     * Create a buffer with window contents
     */

    let mut dimensions = (320u32, 240u32);

    /*
     * Init wayland objects
     */

    let surface = env
        .compositor
        .create_surface(|surface| surface.implement(|_, _| {}, ()))
        .unwrap();

    let next_action = Arc::new(Mutex::new(None::<WEvent>));

    let waction = next_action.clone();
    let mut window = Window::<ConceptFrame>::init_from_env(&env, surface, dimensions, move |evt| {
        let mut next_action = waction.lock().unwrap();
        // Keep last event in priority order : Close > Configure > Refresh
        let replace = match (&evt, &*next_action) {
            (_, &None)
            | (_, &Some(WEvent::Refresh))
            | (&WEvent::Configure { .. }, &Some(WEvent::Configure { .. }))
            | (&WEvent::Close, _) => true,
            _ => false,
        };
        if replace {
            *next_action = Some(evt);
        }
    }).expect("Failed to create a window !");

    let mut pools = DoubleMemPool::new(&env.shm, || {}).expect("Failed to create a memory pool !");

    /*
     * Pointer initialization
     */

    // initialize a seat to retrieve keyboard events
    let seat = env
        .manager
        .instantiate_auto(|seat| seat.implement(|_, _| {}, ()))
        .unwrap();

    window.new_seat(&seat);

    let main_surface = window.surface().clone();
    seat.get_pointer(move |ptr| {
        ptr.implement(
            move |evt, _| match evt {
                wl_pointer::Event::Enter {
                    surface,
                    surface_x,
                    surface_y,
                    ..
                } => {
                    if main_surface == surface {
                        println!("Pointer entered at ({}, {})", surface_x, surface_y);
                    }
                }
                wl_pointer::Event::Leave { surface, .. } => {
                    if main_surface == surface {
                        println!("Pointer left");
                    }
                }
                wl_pointer::Event::Button { button, state, .. } => {
                    println!("Button {:?} was {:?}", button, state);
                }
                wl_pointer::Event::Motion {
                    surface_x,
                    surface_y,
                    ..
                } => println!("Pointer motion to ({}, {})", surface_x, surface_y),
                _ => {}
            },
            (),
        )
    }).unwrap();

    if !env.shell.needs_configure() {
        // initial draw to bootstrap on wl_shell
        if let Some(pool) = pools.pool() {
            redraw(pool, window.surface(), dimensions)
        }
        window.refresh();
    }

    loop {
        match next_action.lock().unwrap().take() {
            Some(WEvent::Close) => break,
            Some(WEvent::Refresh) => {
                window.refresh();
                window.surface().commit();
            }
            Some(WEvent::Configure { new_size, states }) => {
                if let Some((w, h)) = new_size {
                    window.resize(w, h);
                    dimensions = (w, h)
                }
                println!("Window states: {:?}", states);
                window.refresh();
                if let Some(pool) = pools.pool() {
                    redraw(pool, window.surface(), dimensions)
                }
            }
            None => {}
        }

        display.flush().unwrap();
        event_queue.dispatch().unwrap();
    }
}

fn redraw(pool: &mut MemPool, surface: &Proxy<wl_surface::WlSurface>, (buf_x, buf_y): (u32, u32)) {
    // resize the pool if relevant
    pool.resize((4 * buf_x * buf_y) as usize)
        .expect("Failed to resize the memory pool.");
    // write the contents, a nice color gradient =)
    let _ = pool.seek(SeekFrom::Start(0));
    {
        let mut writer = BufWriter::new(&mut *pool);
        for i in 0..(buf_x * buf_y) {
            let x = (i % buf_x) as u32;
            let y = (i / buf_x) as u32;
            let r: u32 = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let g: u32 = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let b: u32 = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
            let _ = writer.write_u32::<NativeEndian>((0xFF << 24) + (r << 16) + (g << 8) + b);
        }
        let _ = writer.flush();
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
}
