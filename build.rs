fn main() {
    #[cfg(feature = "xkbcommon")]
    pkg_config::Config::new().find("xkbcommon").unwrap();
}
