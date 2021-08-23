extern crate pkg_config;

fn main() {
    #[cfg(not(feature = "dlopen"))]
    if pkg_config::Config::new().find("xkbcommon").is_ok() {
        return;
    }
}
