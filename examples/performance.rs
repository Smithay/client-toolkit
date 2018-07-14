extern crate smithay_client_toolkit as sctk;

use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sctk::keyboard::{map_keyboard_auto, Event as KbEvent};
use sctk::utils::{DoubleMemPool, MemPool};
use sctk::window::{BasicFrame, Event as WEvent, Window};
use sctk::Environment;

use sctk::reexports::client::protocol::wl_buffer::RequestsTrait as BufferRequests;
use sctk::reexports::client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use sctk::reexports::client::protocol::wl_display::RequestsTrait as DisplayRequests;
use sctk::reexports::client::protocol::wl_seat::RequestsTrait as SeatRequests;
use sctk::reexports::client::protocol::wl_surface::RequestsTrait as SurfaceRequests;
use sctk::reexports::client::protocol::{wl_buffer, wl_seat, wl_shm, wl_surface};
use sctk::reexports::client::{Display, Proxy};

fn main() {
    let (display, mut event_queue) =
        Display::connect_to_env().expect("Failed to connect to the wayland server.");
    let env =
        Environment::from_registry(display.get_registry().unwrap(), &mut event_queue).unwrap();

    let seat = env.manager
        .instantiate_auto::<wl_seat::WlSeat>()
        .unwrap()
        .implement(move |_, _| {});


    // we need a window to receive things actually
    let dimensions = Arc::new(Mutex::new((320u32, 240u32)));
    let surface = env.compositor
        .create_surface()
        .unwrap()
        .implement(|_, _| {});

    let next_action = Arc::new(Mutex::new(None::<WEvent>));

    let waction = next_action.clone();
    let window = Arc::new(Mutex::new(Window::<BasicFrame>::init(
        surface,
        *dimensions.lock().unwrap(),
        &env.compositor,
        &env.subcompositor,
        &env.shm,
        &env.shell,
        move |evt, ()| {
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
        },
    ).expect("Failed to create a window !")));

    window.lock().unwrap().new_seat(&seat);

    let pools = Arc::new(Mutex::new(DoubleMemPool::new(&env.shm).expect("Failed to create a memory pool !")));
    let buffer = Arc::new(Mutex::new(None));

    let window_clone = window.clone();
    let pools_clone = pools.clone();
    let buffer_clone = buffer.clone();
    let dimensions_clone = dimensions.clone();
    let _keyboard = map_keyboard_auto(seat.get_keyboard().unwrap(), move |event: KbEvent, _| {
        match event {
            KbEvent::Key {
                utf8: Some(text), ..
            } => {
                if text == " " {
                    // Redraw and measure performance on spacebar
                    let mut start = Instant::now();
                    let mut window_clone = window_clone.lock().unwrap();
                    window_clone.refresh();
                    let window_dur = start.elapsed();
                    let mut pools_clone = pools_clone.lock().unwrap();

                    start = Instant::now();
                    redraw(pools_clone.pool(), &mut buffer_clone.lock().unwrap(), window_clone.surface(), *dimensions_clone.lock().unwrap());
                    let content_dur = start.elapsed();
                    
                    println!("-- Performance Report --");
                    if window_dur.subsec_millis() > 5 {
                        println!("Window decorations redrew in: {} millisecs", window_dur.subsec_millis())
                    } else {
                        println!("Window decorations redrew in: {} microsecs", window_dur.subsec_micros())
                    }

                    if content_dur.subsec_millis() > 5 {
                        println!("Contents redrew in: {} millisecs", content_dur.subsec_millis())
                    } else {
                        println!("Contents redrew in: {} microsecs", content_dur.subsec_micros())
                    }

                    pools_clone.swap();
                }
            }
            _ => (),
        }
    });

    loop {
        match next_action.lock().unwrap().take() {
            Some(WEvent::Close) => break,
            Some(WEvent::Refresh) => {
                let mut window = window.lock().unwrap();
                window.refresh();
                window.surface().commit();
            }
            Some(WEvent::Configure { new_size, .. }) => {
                let mut window = window.lock().unwrap();
                let mut pools = pools.lock().unwrap();
                if let Some((w, h)) = new_size {
                    window.resize(w, h);
                    *dimensions.lock().unwrap() = (w, h)
                }
                window.refresh();
                redraw(pools.pool(), &mut buffer.lock().unwrap(), window.surface(), *dimensions.lock().unwrap());
                pools.swap();
            }
            None => {}
        }

        display.flush().unwrap();
        event_queue.dispatch().unwrap();
    }
}

fn redraw(
    pool: &mut MemPool,
    buffer: &mut Option<Proxy<wl_buffer::WlBuffer>>,
    surface: &Proxy<wl_surface::WlSurface>,
    (buf_x, buf_y): (u32, u32),
) {
    // destroy the old buffer if any
    if let Some(b) = buffer.take() {
        b.destroy();
    }
    // resize the pool if relevant
    pool.resize((4 * buf_x * buf_y) as usize)
        .expect("Failed to resize the memory pool.");
    // write the contents, a nice color gradient =)
    let _ = pool.seek(SeekFrom::Start(0));
    {
        let mut writer = BufWriter::new(&mut *pool);
        for y in 0..buf_y {
            for x in 0..buf_x {
                if (x / 10) % 2 != (y / 10) % 2 {
                   writer.write(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
                } else {
                   writer.write(&[0x00, 0x00, 0x00, 0xFF]).unwrap(); 
                }
            }
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
    ).implement(|_, _| {});
    surface.attach(Some(&new_buffer), 0, 0);
    surface.commit();
    *buffer = Some(new_buffer);
}

