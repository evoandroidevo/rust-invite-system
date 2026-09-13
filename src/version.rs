pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn app_version() -> &'static str {
    APP_VERSION
}
