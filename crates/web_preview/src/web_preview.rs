#![recursion_limit = "1024"]

#[cfg(target_os = "windows")]
use gpui::{App, actions};
#[cfg(target_os = "windows")]
use workspace::{Workspace, register_project_item};

#[cfg(target_os = "windows")]
pub(crate) mod agent_browser_contracts;
#[cfg(target_os = "windows")]
pub mod dx_studio;
#[cfg(target_os = "windows")]
pub(crate) mod dx_studio_bridge;
#[cfg(target_os = "windows")]
pub(crate) mod dx_studio_session;
#[cfg(target_os = "windows")]
pub(crate) mod dx_studio_source_edit;
#[cfg(target_os = "windows")]
pub(crate) mod dx_style_generator_surface;
#[cfg(target_os = "windows")]
pub(crate) mod dx_style_native_writer_replay;
#[cfg(target_os = "windows")]
pub(crate) mod dx_style_source_apply;
#[cfg(target_os = "windows")]
pub mod web_preview_view;
#[cfg(target_os = "windows")]
pub(crate) mod windows_visual_webview;

#[cfg(target_os = "windows")]
actions!(
    web_preview,
    [
        /// Opens a web preview for the current workspace.
        OpenPreview,
        /// Opens a web preview in a split pane.
        OpenPreviewToTheSide,
    ]
);

#[cfg(target_os = "windows")]
pub fn init(cx: &mut App) {
    register_project_item::<web_preview_view::WebPreviewView>(cx);
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        web_preview_view::WebPreviewView::register(workspace, window, cx);
        cx.defer_in(window, |workspace, window, cx| {
            web_preview_view::WebPreviewView::ensure_startup_preview(workspace, window, cx);
        });
    })
    .detach();
}

#[cfg(all(
    unix,
    not(target_os = "linux"),
    not(target_os = "macos"),
    not(target_os = "windows")
))]
pub use web_preview_linux::init;
#[cfg(target_os = "linux")]
pub use web_preview_linux::{OpenPreview, OpenPreviewToTheSide, init, web_preview_view};
#[cfg(target_os = "macos")]
pub use web_preview_macos::{OpenPreview, OpenPreviewToTheSide, init, web_preview_view};

#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    all(
        unix,
        not(target_os = "linux"),
        not(target_os = "macos"),
        not(target_os = "windows")
    )
)))]
pub fn init(_: &mut gpui::App) {}
