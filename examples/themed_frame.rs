extern crate smithay_client_toolkit as sctk;

use std::cmp::min;
use std::io::{BufWriter, Seek, SeekFrom, Write};

use sctk::reexports::client::protocol::{wl_shm, wl_surface};
use sctk::shm::MemPool;
use sctk::window::{ButtonColorSpec, ColorSpec, ConceptConfig, ConceptFrame, Event as WEvent};

sctk::default_environment!(ThemedFrameExample, desktop);

// The frame configuration we will use in this example
fn create_frame_config() -> ConceptConfig {
    let icon_spec = ButtonColorSpec {
        hovered: ColorSpec::identical([0xFF, 0x22, 0x22, 0x22].into()),
        idle: ColorSpec::identical([0xFF, 0xff, 0xff, 0xff].into()),
        disabled: ColorSpec::invisible(),
    };

    ConceptConfig {
        // dark theme
        primary_color: ColorSpec {
            active: [0xFF, 0x22, 0x22, 0x22].into(),
            inactive: [0xFF, 0x33, 0x33, 0x33].into(),
        },
        // white separation line
        secondary_color: ColorSpec::identical([0xFF, 0xFF, 0xFF, 0xFF].into()),
        // red close button
        close_button: Some((
            // icon
            icon_spec,
            // button background
            ButtonColorSpec {
                hovered: ColorSpec::identical([0xFF, 0xFF, 0x00, 0x00].into()),
                idle: ColorSpec::identical([0xFF, 0x88, 0x00, 0x00].into()),
                disabled: ColorSpec::invisible(),
            },
        )),
        // green maximize button
        maximize_button: Some((
            // icon
            icon_spec,
            // button background
            ButtonColorSpec {
                hovered: ColorSpec::identical([0xFF, 0x00, 0xFF, 0x00].into()),
                idle: ColorSpec::identical([0xFF, 0x00, 0x88, 0x00].into()),
                disabled: ColorSpec::invisible(),
            },
        )),
        // blue minimize button
        minimize_button: Some((
            // icon
            icon_spec,
            // button background
            ButtonColorSpec {
                hovered: ColorSpec::identical([0xFF, 0x00, 0x00, 0xFF].into()),
                idle: ColorSpec::identical([0xFF, 0x00, 0x00, 0x88].into()),
                disabled: ColorSpec::invisible(),
            },
        )),
        // same font as default
        title_font: Some(("sans".into(), 17.0)),
        // clear text over dark background
        title_color: ColorSpec::identical([0xFF, 0xD0, 0xD0, 0xD0].into()),
    }
}

fn main() {
    /*
     * Initial setup
     */
    let (env, _display, mut queue) = sctk::new_default_environment!(ThemedFrameExample, desktop)
        .expect("Unable to connect to a Wayland compositor");
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

    window.set_title("Themed frame".to_string());
    window.set_frame_config(create_frame_config());

    let mut pools = env.create_double_pool(|_| {}).expect("Failed to create a memory pool !");

    /*
     * Keyboard initialization
     */

    if !env.get_shell().unwrap().needs_configure() {
        // initial draw to bootstrap on wl_shell
        if let Some(pool) = pools.pool() {
            redraw(pool, window.surface(), dimensions).expect("Failed to draw")
        }
        window.refresh();
    }

    let mut next_action = None;

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

        queue.dispatch(&mut next_action, |_, _, _| {}).unwrap();
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
