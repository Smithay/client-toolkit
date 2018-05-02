extern crate byteorder;
extern crate image;
extern crate smithay_client_toolkit as sctk;
extern crate tempfile;

use std::env;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use byteorder::{NativeEndian, WriteBytesExt};

use sctk::Environment;
use sctk::reexports::client::{Display, Proxy};
use sctk::reexports::client::protocol::{wl_buffer, wl_seat, wl_shm, wl_surface};
use sctk::reexports::client::protocol::wl_display::RequestsTrait as DisplayRequests;
use sctk::reexports::client::protocol::wl_compositor::RequestsTrait as CompositorRequests;
use sctk::reexports::client::protocol::wl_surface::RequestsTrait as SurfaceRequests;
use sctk::reexports::client::protocol::wl_buffer::RequestsTrait as BufferRequests;
use sctk::window::{BasicFrame, Event as WEvent, State, Window};
use sctk::utils::{DoubleMemPool, MemPool};

fn main() {
    let (display, mut event_queue) = Display::connect_to_env().unwrap();
    let env =
        Environment::from_registry(display.get_registry().unwrap(), &mut event_queue).unwrap();

    /*
     * Load the requested image
     */
    let path = match env::args_os().skip(1).next() {
        Some(p) => p,
        None => {
            println!("USAGE: ./image_wiewer <PATH>");
            return;
        }
    };
    let image = match image::open(&path).map(|i| i.to_rgba()) {
        Ok(i) => i,
        Err(e) => {
            println!("Failed to open image {}.", path.to_string_lossy());
            println!("Error was: {:?}", e);
            return;
        }
    };

    let mut dimensions = image.dimensions();

    /*
     * Init wayland objects
     */

    let surface = env.compositor
        .create_surface()
        .unwrap()
        .implement(|_, _| {});

    let next_action = Arc::new(Mutex::new(None::<WEvent>));

    let waction = next_action.clone();
    let mut window = Window::<BasicFrame>::init(
        surface,
        dimensions,
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
    ).expect("Failed to create a window !");

    let mut pools = DoubleMemPool::new(&env.shm).expect("Failed to create a memory pool !");
    let mut buffer = None;
    let mut resizing = false;

    // initialize a seat to allow resizing
    let seat = env.manager
        .instanciate_auto::<wl_seat::WlSeat>()
        .unwrap()
        .implement(move |_, _| {});

    window.new_seat(&seat);

    if !env.shell.needs_configure() {
        // initial draw to bottstrap on wl_shell
        buffer = Some(redraw(
            pools.pool(),
            window.surface(),
            Some(&image),
            dimensions,
        ));
        window.refresh();
        window.surface().commit();
    }

    loop {
        match next_action.lock().unwrap().take() {
            Some(WEvent::Close) => break,
            Some(WEvent::Refresh) => window.refresh(),
            Some(WEvent::Configure { new_size, states }) => {
                if let Some((w, h)) = new_size {
                    if dimensions != (w, h) {
                        // we need to redraw
                        window.resize(w, h);
                        dimensions = (w, h);
                        if let Some(b) = buffer.take() {
                            b.destroy();
                        }
                    }
                }
                let new_resizing = states.contains(&State::Resizing);
                if new_resizing != resizing {
                    // we started or stopped resizing
                    if let Some(b) = buffer.take() {
                        b.destroy();
                    }
                }
                window.refresh();
                resizing = new_resizing;
                if buffer.is_none() {
                    // either we need to redraw or we have not drawn yet
                    buffer = Some(redraw(
                        pools.pool(),
                        window.surface(),
                        if resizing { None } else { Some(&image) },
                        dimensions,
                    ));
                    pools.swap();
                }
                window.surface().commit();
            }
            None => {}
        }

        display.flush().unwrap();
        event_queue.dispatch().unwrap();
    }
}

fn redraw(
    pool: &mut MemPool,
    surface: &Proxy<wl_surface::WlSurface>,
    base_image: Option<&image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>,
    (buf_x, buf_y): (u32, u32),
) -> Proxy<wl_buffer::WlBuffer> {
    // resize the pool if relevant
    pool.resize((4 * buf_x * buf_y) as usize)
        .expect("Failed to resize the memory pool.");
    // write the contents
    let _ = pool.seek(SeekFrom::Start(0));
    {
        let mut writer = BufWriter::new(&mut *pool);
        if let Some(base_image) = base_image {
            // resize the image
            let image =
                image::imageops::resize(base_image, buf_x, buf_y, image::FilterType::Nearest);
            // need to write pixel by pixel to the SHM pool, as
            // our image is RGBA8888 and we chose ARGB888 with the compositor
            for pixel in image.pixels() {
                let _ = writer.write_u32::<NativeEndian>(
                    ((pixel.data[3] as u32) << 24) // A
                        + ((pixel.data[0] as u32) << 16) // R
                        + ((pixel.data[1] as u32) << 8 ) // G
                        + ((pixel.data[2] as u32)      ), // B
                );
            }
        } else {
            // no image, draw black contents
            for _ in 0..(buf_x * buf_y) {
                let _ = writer.write_u32::<NativeEndian>(0xFF000000);
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

    // Damage the surface
    if surface.version() >= 4 {
        surface.damage_buffer(0, 0, buf_x as i32, buf_y as i32);
    } else {
        // surface is old and does not support damage_buffer, so we damage
        // in surface coordinates and hope it is not rescaled
        surface.damage(0, 0, buf_x as i32, buf_y as i32);
    }

    return new_buffer;
}
