extern crate byteorder;
extern crate smithay_client_toolkit as sctk;

use std::cmp::min;
use std::io::{BufWriter, Seek, SeekFrom, Write};

use byteorder::{NativeEndian, WriteBytesExt};

use sctk::reexports::client::protocol::{wl_shm, wl_surface};
use sctk::reexports::client::Display;
use sctk::shm::MemPool;
use sctk::window::{ButtonState, ConceptFrame, Event as WEvent, Theme};

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

    let mut window = env
        .create_window::<ConceptFrame, _>(surface, dimensions, move |evt, mut dispatch_data| {
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
        })
        .expect("Failed to create a window !");

    let scaled_bg = [0xFF, 0x22, 0x22, 0x22];
    let vscaled_bg = [0xFF, 0x33, 0x33, 0x33];

    window.set_title("Kbd Input".to_string());
    window.set_theme(WaylandTheme {
        primary_active: scaled_bg,
        primary_inactive: vscaled_bg,
        secondary_active: [0xFF, 0xFF, 0xFF, 0xFF],
        secondary_inactive: [0xFF, 0xFF, 0xFF, 0xFF],
        close_button_hovered: [0xFF, 0xFF, 0x00, 0x00],
        close_button: [0xFF, 0x88, 0x00, 0x00],
        close_button_icon_hovered: scaled_bg,
        close_button_icon: [0xFF, 0xff, 0xff, 0xff],
        maximize_button_hovered: [0xFF, 0x00, 0xFF, 0x00],
        maximize_button: [0xFF, 0x00, 0x88, 0x00],
        minimize_button_hovered: [0xFF, 0x00, 0x00, 0xFF],
        minimize_button: [0xFF, 0x00, 0x00, 0x88],
    });

    let mut pools = env
        .create_double_pool(|_| {})
        .expect("Failed to create a memory pool !");

    /*
     * Keyboard initialization
     */

    // initialize a seat to retrieve keyboard events
    let seat = env.manager.instantiate_range(1, 6).unwrap();

    window.new_seat(&seat);

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

pub struct WaylandTheme {
    /// Primary color when the window is focused
    pub primary_active: [u8; 4],
    /// Primary color when the window is unfocused
    pub primary_inactive: [u8; 4],
    /// Secondary color when the window is focused
    pub secondary_active: [u8; 4],
    /// Secondary color when the window is unfocused
    pub secondary_inactive: [u8; 4],
    /// Close button color when hovered over
    pub close_button_hovered: [u8; 4],
    /// Close button color
    pub close_button: [u8; 4],
    /// Close button fg color when hovered over
    pub close_button_icon_hovered: [u8; 4],
    /// Close button fg color
    pub close_button_icon: [u8; 4],
    /// Close button color when hovered over
    pub maximize_button_hovered: [u8; 4],
    /// Maximize button color
    pub maximize_button: [u8; 4],
    /// Minimize button color when hovered over
    pub minimize_button_hovered: [u8; 4],
    /// Minimize button color
    pub minimize_button: [u8; 4],
}

impl Theme for WaylandTheme {
    fn get_primary_color(&self, active: bool) -> [u8; 4] {
        if active {
            self.primary_active
        } else {
            self.primary_inactive
        }
    }

    // Used for division line
    fn get_secondary_color(&self, active: bool) -> [u8; 4] {
        if active {
            self.secondary_active
        } else {
            self.secondary_inactive
        }
    }

    fn get_close_button_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => self.close_button_hovered,
            ButtonState::Idle => self.close_button,
            _ => self.close_button,
        }
    }

    fn get_close_button_icon_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => self.close_button_icon_hovered,
            ButtonState::Idle => self.close_button_icon,
            _ => self.close_button,
        }
    }

    fn get_maximize_button_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => self.maximize_button_hovered,
            _ => self.maximize_button,
        }
    }

    fn get_minimize_button_color(&self, state: ButtonState) -> [u8; 4] {
        match state {
            ButtonState::Hovered => self.minimize_button_hovered,
            _ => self.minimize_button,
        }
    }
}
