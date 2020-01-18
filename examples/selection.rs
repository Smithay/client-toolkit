extern crate byteorder;
extern crate smithay_client_toolkit as sctk;

use std::io::{BufWriter, Read, Seek, SeekFrom, Write};

use byteorder::{NativeEndian, WriteBytesExt};

use sctk::{
    data_device::ReadPipeSource,
    environment::Environment,
    seat::keyboard::{map_keyboard, Event as KbEvent, KeyState, RepeatKind},
    shm::MemPool,
    window::{ConceptFrame, Event as WEvent},
};

use sctk::reexports::{
    calloop::{LoopHandle, Source},
    client::{
        protocol::{wl_keyboard, wl_seat, wl_shm, wl_surface},
        DispatchData, Display,
    },
};

sctk::default_environment!(SelectionExample);

// Here the type parameter is a global value that will be shared by
// all callbacks invoked by the event loop.
type DData = (
    Environment<SelectionExample>,
    Option<WEvent>,
    Option<Source<ReadPipeSource>>,
);

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
        SelectionExample,
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
     * Prepare a calloop event loop to handle clipboard reading
     */
    let mut event_loop = sctk::reexports::calloop::EventLoop::<DData>::new().unwrap();

    // we need a window to receive things actually
    let mut dimensions = (320u32, 240u32);
    let surface = env.create_surface();

    let mut window = env
        .create_window::<ConceptFrame, _>(surface, dimensions, move |evt, mut dispatch_data| {
            let (_, next_action, _) = dispatch_data.get::<DData>().unwrap();
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
        })
        .expect("Failed to create a window !");
    window.set_title("Selection".to_string());

    let mut pools = env
        .create_double_pool(|_| {})
        .expect("Failed to create a memory pool !");

    let mut seats = Vec::<(String, Option<wl_keyboard::WlKeyboard>)>::new();

    // first process already existing seats
    for seat in env.get_all_seats() {
        if let Some((has_kbd, name)) = sctk::seat::with_seat_data(&seat, |seat_data| {
            (
                seat_data.has_keyboard && !seat_data.defunct,
                seat_data.name.clone(),
            )
        }) {
            if has_kbd {
                let my_seat = seat.clone();
                let handle = event_loop.handle();
                match map_keyboard(&seat, None, RepeatKind::System, move |event, _, ddata| {
                    process_keyboard_event(event, &my_seat, &handle, ddata)
                }) {
                    Ok((kbd, _)) => {
                        seats.push((name, Some(kbd)));
                    }
                    Err(e) => {
                        eprintln!("Failed to map keyboard on seat {} : {:?}.", name, e);
                        seats.push((name, None));
                    }
                }
            } else {
                seats.push((name, None));
            }
        }
    }

    // then setup a listener for changes
    let loop_handle = event_loop.handle();
    let _seat_listener = env.listen_for_seats(move |seat, seat_data, _| {
        // find the seat in the vec of seats, or insert it if it is unknown
        let idx = seats.iter().position(|(name, _)| name == &seat_data.name);
        let idx = idx.unwrap_or_else(|| {
            seats.push((seat_data.name.clone(), None));
            seats.len() - 1
        });

        let (_, ref mut opt_kbd) = &mut seats[idx];
        // we should map a keyboard if the seat has the capability & is not defunct
        if seat_data.has_keyboard && !seat_data.defunct {
            if opt_kbd.is_none() {
                // we should initalize a keyboard
                let my_seat = seat.clone();
                let handle = loop_handle.clone();
                match map_keyboard(&seat, None, RepeatKind::System, move |event, _, ddata| {
                    process_keyboard_event(event, &my_seat, &handle, ddata)
                }) {
                    Ok((kbd, _)) => {
                        *opt_kbd = Some(kbd);
                    }
                    Err(e) => eprintln!(
                        "Failed to map keyboard on seat {} : {:?}.",
                        seat_data.name, e
                    ),
                }
            }
        } else {
            if let Some(kbd) = opt_kbd.take() {
                // the keyboard has been removed, cleanup
                kbd.release();
            }
        }
    });

    if !env.get_shell().unwrap().needs_configure() {
        // initial draw to bootstrap on wl_shell
        if let Some(pool) = pools.pool() {
            redraw(pool, window.surface(), dimensions).expect("Failed to draw")
        }
        window.refresh();
    }

    // the data that will be shared to all callbacks
    let mut data: DData = (env, None, None);

    let _source_queue = event_loop
        .handle()
        .insert_source(sctk::WaylandSource::new(queue), |ret, _| {
            if let Err(e) = ret {
                panic!("Wayland connection lost: {:?}", e);
            }
        })
        .unwrap();

    loop {
        match data.1.take() {
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
                    redraw(pool, window.surface(), dimensions).expect("Failed to draw")
                }
            }
            None => {}
        }

        // always flush the connection before going to sleep waiting for events
        display.flush().unwrap();

        event_loop.dispatch(None, &mut data).unwrap();
    }
}

fn process_keyboard_event(
    event: KbEvent,
    seat: &wl_seat::WlSeat,
    handle: &LoopHandle<DData>,
    mut ddata: DispatchData,
) {
    let (env, _, opt_source) = ddata.get::<DData>().unwrap();
    match event {
        KbEvent::Key {
            state,
            utf8: Some(text),
            ..
        } => {
            if text == "p" && state == KeyState::Pressed {
                // pressed the 'p' key, try to read contents !
                env.with_data_device(seat, |device| {
                    device.with_selection(|offer| {
                        if let Some(offer) = offer {
                            let seat_name =
                                sctk::seat::with_seat_data(seat, |data| data.name.clone()).unwrap();
                            print!(
                                "Current selection buffer mime types on seat '{}': [ ",
                                seat_name
                            );
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
                                let reader =
                                    offer.receive("text/plain;charset=utf-8".into()).unwrap();
                                let source = handle
                                    .insert_source(reader.into_source(), |event, ddata| {
                                        // a sub-scope because we cannot be borrowing the source (pipe variable)
                                        // when we try to remove() it later.
                                        {
                                            let mut pipe = event.source.borrow_mut();
                                            let mut txt = String::new();
                                            pipe.0.read_to_string(&mut txt).unwrap();
                                            println!("Selection contents are: \"{}\"", txt);
                                        }
                                        if let Some(src) = ddata.2.take() {
                                            src.remove();
                                        }
                                    })
                                    .unwrap();
                                *opt_source = Some(source);
                            }
                        } else {
                            println!("No current selection buffer!");
                        }
                    });
                })
                .unwrap();
            }
        }
        _ => (),
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
        for _ in 0..(buf_x * buf_y) {
            writer.write_u32::<NativeEndian>(0xFF000000)?;
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
