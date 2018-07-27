extern crate smithay_client_toolkit as sctk;

use sctk::reexports::client::Display;
use sctk::{Environment, Shell};

use sctk::reexports::client::protocol::wl_display::RequestsTrait as DisplayRequests;

// This is a small program that queries the compositor for
// various information and prints them on the console before exiting.

fn main() {
    let (display, mut event_queue) = Display::connect_to_env().unwrap();
    let env =
        Environment::from_registry(display.get_registry().unwrap(), &mut event_queue).unwrap();

    println!("== Smithay's compositor info tool ==\n");

    // print the best supported shell
    println!(
        "-> Most recent shell supported by the compositor is {}.\n",
        match env.shell {
            Shell::Wl(_) => "the legacy wl_shell",
            Shell::Zxdg(_) => "the old unstable xdg_shell (zxdg_shell_v6)",
            Shell::Xdg(_) => "the current xdg_shell",
        }
    );

    env.outputs.with_all(|outputs| {
        println!("-> Compositor advertized {} outputs:", outputs.len());
        for &(id, _, ref info) in outputs {
            println!(
                "  -> #{}: {} ({}), with scale factor of {}",
                id, info.model, info.make, info.scale_factor
            );
            println!("     Possible modes are:");
            for mode in &info.modes {
                println!(
                    "     -> [{}{}] {} x {} @ {}.{} Hz",
                    if mode.is_preferred { "p" } else { " " },
                    if mode.is_current { "c" } else { " " },
                    mode.dimensions.0,
                    mode.dimensions.1,
                    mode.refresh_rate / 1000,
                    mode.refresh_rate % 1000
                );
            }
        }
    });

    if env.decorations_mgr.is_some() {
        println!("-> Compositor supports server-side decorations.")
    } else {
        println!("-> Compositor does not support server-side decorations.")
    }
}
