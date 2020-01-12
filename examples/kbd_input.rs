extern crate byteorder;
extern crate smithay_client_toolkit as sctk;

use std::cmp::min;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use byteorder::{NativeEndian, WriteBytesExt};

use sctk::reexports::client::{
    protocol::{wl_shm, wl_surface},
    Display,
};
use sctk::seat::keyboard::{map_keyboard, Event as KbEvent, RepeatKind};
use sctk::shm::MemPool;
use sctk::window::{ConceptFrame, Event as WEvent};

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
     * Create a buffer with window contents
     */

    let mut dimensions = (320u32, 240u32);

    /*
     * Init wayland objects
     */

    let surface = env.create_surface();

    let next_action = Arc::new(Mutex::new(None::<WEvent>));

    let waction = next_action.clone();
    let mut window = env
        .create_window::<ConceptFrame, _>(surface, dimensions, move |evt, _| {
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
        })
        .expect("Failed to create a window !");

    window.set_title("Kbd Input".to_string());

    let mut pools = env
        .create_double_pool(|_| {})
        .expect("Failed to create a memory pool !");

    /*
     * Keyboard initialization
     */

    // initialize a seat to retrieve keyboard events
    let seat = env.manager.instantiate_range(1, 6).unwrap();

    window.new_seat(&seat);

    map_keyboard(
        &seat,
        None,
        RepeatKind::System,
        move |event: KbEvent, _, _| match event {
            KbEvent::Enter { keysyms, .. } => {
                println!("Gained focus while {} keys pressed.", keysyms.len(),);
            }
            KbEvent::Leave { .. } => {
                println!("Lost focus.");
            }
            KbEvent::Key {
                keysym,
                state,
                utf8,
                ..
            } => {
                println!("Key {:?}: {:x}.", state, keysym);
                if let Some(txt) = utf8 {
                    println!(" -> Received text \"{}\".", txt);
                }
            }
            KbEvent::Modifiers { modifiers } => {
                println!("Modifiers changed {:?}", modifiers);
            }
            KbEvent::Repeat { keysym, utf8, .. } => {
                println!("Key repetition {:x}", keysym);
                if let Some(txt) = utf8 {
                    println!(" -> Received text \"{}\".", txt);
                }
            }
        },
    )
    .expect("Failed to map keyboard");

    if !env.get_shell().unwrap().needs_configure() {
        // initial draw to bootstrap on wl_shell
        if let Some(pool) = pools.pool() {
            redraw(pool, window.surface(), dimensions).expect("Failed to draw")
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
                    redraw(pool, window.surface(), dimensions).expect("Failed to draw")
                }
            }
            None => {}
        }

        queue.dispatch(&mut (), |_, _, _| {}).unwrap();
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
