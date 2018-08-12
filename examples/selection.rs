extern crate byteorder;
extern crate smithay_client_toolkit as sctk;

use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use byteorder::{NativeEndian, WriteBytesExt};

use sctk::data_device::{DataDevice, DndEvent, ReadPipe};
use sctk::keyboard::{map_keyboard_auto, Event as KbEvent, KeyState};
use sctk::utils::{DoubleMemPool, MemPool};
use sctk::window::{BasicFrame, Event as WEvent, Window};
use sctk::Environment;

use sctk::reexports::client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use sctk::reexports::client::protocol::wl_display::RequestsTrait as DisplayRequests;
use sctk::reexports::client::protocol::wl_seat::RequestsTrait as SeatRequests;
use sctk::reexports::client::protocol::wl_surface::RequestsTrait as SurfaceRequests;
use sctk::reexports::client::protocol::{wl_seat, wl_shm, wl_surface};
use sctk::reexports::client::{Display, Proxy};

fn main() {
    let (display, mut event_queue) =
        Display::connect_to_env().expect("Failed to connect to the wayland server.");
    let env =
        Environment::from_registry(display.get_registry().unwrap(), &mut event_queue).unwrap();

    let seat = env
        .manager
        .instantiate_auto::<wl_seat::WlSeat>()
        .unwrap()
        .implement(move |_, _| {});

    let device = DataDevice::init_for_seat(
        &env.data_device_manager,
        &seat,
        |event: DndEvent, ()| match event {
            // we don't accept drag'n'drop
            DndEvent::Enter {
                offer: Some(offer), ..
            } => offer.accept(None),
            _ => (),
        },
    );

    // we need a window to receive things actually
    let mut dimensions = (320u32, 240u32);
    let surface = env
        .compositor
        .create_surface()
        .unwrap()
        .implement(|_, _| {});

    let next_action = Arc::new(Mutex::new(None::<WEvent>));

    let waction = next_action.clone();
    let mut window =
        Window::<BasicFrame>::init_from_env(&env, surface, dimensions, move |evt, ()| {
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

    window.new_seat(&seat);

    let mut pools =
        DoubleMemPool::new(&env.shm, |_, _| {}).expect("Failed to create a memory pool !");

    let reader = Arc::new(Mutex::new(None::<ReadPipe>));

    let reader2 = reader.clone();
    let _keyboard = map_keyboard_auto(seat.get_keyboard().unwrap(), move |event: KbEvent, _| {
        match event {
            KbEvent::Key {
                state,
                utf8: Some(text),
                ..
            } => {
                if text == "p" && state == KeyState::Pressed {
                    // pressed the 'p' key, try to read contents !
                    device.with_selection(|offer| {
                        if let Some(offer) = offer {
                            print!("Current selection buffer mime types: [ ");
                            let mut has_text = false;
                            offer.with_mime_types(|types| {
                                for t in types {
                                    print!("\"{}\", ", t);
                                    if t == "text/plain;charset=utf-8" {
                                        has_text = true;
                                    }
                                }
                            });
                            println!("]");
                            if has_text {
                                println!("Buffer contains text, going to read it...");
                                let mut reader = reader2.lock().unwrap();
                                *reader =
                                    Some(offer.receive("text/plain;charset=utf-8".into()).unwrap());
                            }
                        } else {
                            println!("No current selection buffer!");
                        }
                    });
                }
            }
            _ => (),
        }
    });

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
            Some(WEvent::Configure { new_size, .. }) => {
                if let Some((w, h)) = new_size {
                    window.resize(w, h);
                    dimensions = (w, h)
                }
                window.refresh();
                if let Some(pool) = pools.pool() {
                    redraw(pool, window.surface(), dimensions)
                }
            }
            None => {}
        }

        display.flush().unwrap();

        if let Some(mut reader) = reader.lock().unwrap().take() {
            // we have something to read
            let mut text = String::new();
            reader.read_to_string(&mut text).unwrap();
            println!("The selection buffer contained: \"{}\"", text);
        }

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
        for _ in 0..(buf_x * buf_y) {
            let _ = writer.write_u32::<NativeEndian>(0xFF000000);
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
