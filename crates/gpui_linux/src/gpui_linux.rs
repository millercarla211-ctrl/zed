#![cfg(any(target_os = "linux", target_os = "freebsd"))]
mod linux;

pub use linux::current_platform;
#[cfg(all(target_os = "linux", feature = "wayland"))]
pub use linux::exported_wayland_window_handle;
