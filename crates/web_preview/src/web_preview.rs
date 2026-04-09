#[cfg(all(unix, not(target_os = "macos"), not(target_os = "windows")))]
pub use web_preview_linux::init;
#[cfg(target_os = "macos")]
pub use web_preview_macos::init;
#[cfg(target_os = "windows")]
pub use web_preview_windows::init;

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    all(unix, not(target_os = "macos"), not(target_os = "windows"))
)))]
pub fn init(_: &mut gpui::App) {}
