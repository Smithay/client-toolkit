extern crate smithay_client_toolkit as sctk;

use std::cmp::min;
use std::io::{BufWriter, Seek, SeekFrom, Write};

use sctk::reexports::calloop;
use sctk::reexports::client::protocol::{wl_keyboard, wl_shm, wl_surface};
use sctk::seat::keyboard::{map_keyboard_repeat, Event as KbEvent, RepeatKind};
use sctk::shm::MemPool;
use sctk::window::{ConceptFrame, Event as WEvent};

sctk::default_environment!(KbdInputExample, desktop);

fn main() {
    /*
     * Initial setup
     */
    let (env, display, queue) = sctk::new_default_environment!(KbdInputExample, desktop)
        .expect("Unable to connect to a Wayland compositor");

    /*
     * Prepare a calloop event loop to handle key repetion
     */
    // Here `Option<WEvent>` is the type of a global value that will be shared by
    // all callbacks invoked by the event loop.
    let mut event_loop = calloop::EventLoop::<Option<WEvent>>::new().unwrap();

    /*
     * Create a buffer with window contents
     */

    let mut dimensions = (320u32, 240u32);

    /*
     * Init wayland objects
     */

    let surface = env.create_surface().detach();

    let mut window = env
        .create_window::<ConceptFrame, _>(
            surface,
            None,
            dimensions,
            move |evt, mut dispatch_data| {
                let next_action = dispatch_data.get::<Option<WEvent>>().unwrap();
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
        )
        .expect("Failed to create a window !");

    window.set_title("Kbd Input".to_string());

    let mut pools = env.create_double_pool(|_| {}).expect("Failed to create a memory pool !");

    /*
     * Keyboard initialization
     */

    let mut seats = Vec::<(String, Option<(wl_keyboard::WlKeyboard, calloop::Source<_>)>)>::new();

    // first process already existing seats
    for seat in env.get_all_seats() {
        if let Some((has_kbd, name)) = sctk::seat::with_seat_data(&seat, |seat_data| {
            (seat_data.has_keyboard && !seat_data.defunct, seat_data.name.clone())
        }) {
            if has_kbd {
                let seat_name = name.clone();
                match map_keyboard_repeat(
                    event_loop.handle(),
                    &seat,
                    None,
                    RepeatKind::System,
                    move |event, _, _| print_keyboard_event(event, &seat_name),
                ) {
                    Ok((kbd, repeat_source)) => {
                        seats.push((name, Some((kbd, repeat_source))));
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
                let seat_name = seat_data.name.clone();
                match map_keyboard_repeat(
                    loop_handle.clone(),
                    &seat,
                    None,
                    RepeatKind::System,
                    move |event, _, _| print_keyboard_event(event, &seat_name),
                ) {
                    Ok((kbd, repeat_source)) => {
                        *opt_kbd = Some((kbd, repeat_source));
                    }
                    Err(e) => {
                        eprintln!("Failed to map keyboard on seat {} : {:?}.", seat_data.name, e)
                    }
                }
            }
        } else {
            if let Some((kbd, source)) = opt_kbd.take() {
                // the keyboard has been removed, cleanup
                kbd.release();
                loop_handle.remove(source);
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

    let mut next_action = None;

    sctk::WaylandSource::new(queue).quick_insert(event_loop.handle()).unwrap();

    loop {
        match next_action.take() {
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
                    redraw(pool, window.surface(), dimensions).expect("Failed to draw")
                }
            }
            None => {}
        }

        // always flush the connection before going to sleep waiting for events
        display.flush().unwrap();

        event_loop.dispatch(None, &mut next_action).unwrap();
    }
}

fn print_keyboard_event(event: KbEvent, seat_name: &str) {
    match event {
        KbEvent::Enter { keysyms, .. } => {
            println!("Gained focus on seat '{}' while {} keys pressed.", seat_name, keysyms.len(),);
        }
        KbEvent::Leave { .. } => {
            println!("Lost focus on seat '{}'.", seat_name);
        }
        KbEvent::Key { keysym, state, utf8, .. } => {
            println!("Key {:?}: {:x} on seat '{}'.", state, keysym, seat_name);
            if let Some(txt) = utf8 {
                println!(" -> Received text \"{}\".", txt);
            }
        }
        KbEvent::Modifiers { modifiers } => {
            println!("Modifiers changed to {:?} on seat '{}'.", modifiers, seat_name);
        }
        KbEvent::Repeat { keysym, utf8, .. } => {
            println!("Key repetition {:x} on seat '{}'.", keysym, seat_name);
            if let Some(txt) = utf8 {
                println!(" -> Received text \"{}\".", txt);
            }
        }
    }
}

fn redraw(
    pool: &mut MemPool,
    surface: &wl_surface::WlSurface,
    (buf_x, buf_y): (u32, u32),
) -> Result<(), ::std::io::Error> {
    // resize the pool if relevant
    pool.resize((4 * buf_x * buf_y) as usize).expect("Failed to resize the memory pool.");
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
            let pixel: u32 = (0xFF << 24) + (r << 16) + (g << 8) + b;
            writer.write_all(&pixel.to_ne_bytes())?;
        }
        writer.flush()?;
    }
    // get a buffer and attach it
    let new_buffer =
        pool.buffer(0, buf_x as i32, buf_y as i32, 4 * buf_x as i32, wl_shm::Format::Argb8888);
    surface.attach(Some(&new_buffer), 0, 0);
    surface.commit();
    Ok(())
}
