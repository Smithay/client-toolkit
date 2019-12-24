extern crate smithay_client_toolkit as sctk;

use sctk::reexports::client::Display;

// This is a small program that queries the compositor for
// various information and prints them on the console before exiting.

fn main() -> Result<(), ()> {
    let display = match Display::connect_to_env() {
        Ok(d) => d,
        Err(e) => {
            println!("Unable to connect to a Wayland compositor: {}", e);
            return Err(());
        }
    };

    let mut queue = display.create_event_queue();

    let env = sctk::default_environment!(CompInfo, &display, &mut queue, singles = [], multis = []);

    println!("== Smithay's compositor info tool ==\n");
    /*
        // print the best supported shell
        println!(
            "-> Most recent shell supported by the compositor is {}.\n",
            match env.shell {
                Shell::Wl(_) => "the legacy wl_shell",
                Shell::Zxdg(_) => "the old unstable xdg_shell (zxdg_shell_v6)",
                Shell::Xdg(_) => "the current xdg_shell",
            }
        );
    */
    let outputs = env.get_all_outputs();
    println!("-> Compositor advertised {} outputs:", outputs.len());
    for output in outputs {
        sctk::output::with_output_info(&output, |info| {
            println!(
                "  -> #{}: {} ({}), with scale factor of {}",
                info.id, info.model, info.make, info.scale_factor
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
        });
    }
    /*
        if env.decorations_mgr.is_some() {
            println!("-> Compositor supports server-side decorations.")
        } else {
            println!("-> Compositor does not support server-side decorations.")
        }
    */
    Ok(())
}
