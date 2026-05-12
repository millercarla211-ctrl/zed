#![cfg(target_os = "windows")]

mod clipboard;
mod destination_list;
mod direct_manipulation;
mod direct_write;
mod directx_atlas;
mod directx_devices;
mod directx_renderer;
mod dispatcher;
mod display;
mod events;
mod keyboard;
mod platform;
mod system_settings;
mod util;
mod vsync;
mod window;
mod wrapper;

pub(crate) use clipboard::*;
pub(crate) use destination_list::*;
pub(crate) use direct_write::*;
pub(crate) use directx_atlas::*;
pub(crate) use directx_devices::*;
pub(crate) use directx_renderer::*;
pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub(crate) use events::*;
pub(crate) use keyboard::*;
pub(crate) use platform::*;
pub(crate) use system_settings::*;
pub(crate) use util::*;
pub(crate) use vsync::*;
pub(crate) use window::*;
pub(crate) use wrapper::*;

pub use platform::WindowsPlatform;
pub use window::{
    any_window_has_focused_webview, clear_webview_passthrough_target,
    clear_webview_passthrough_target_for_controller, create_webview_composition_visual,
    register_webview_passthrough_target, remove_webview_composition_visual,
    set_webview_composition_visual_offset, update_webview_passthrough_cursor,
    update_webview_passthrough_cursor_for_controller, update_webview_passthrough_focus,
    update_webview_passthrough_focus_for_controller, window_has_focused_webview,
};

pub(crate) use windows::Win32::Foundation::HWND;
