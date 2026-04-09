#[cfg(target_os = "windows")]
use gpui::{App, actions};
#[cfg(target_os = "windows")]
use workspace::Workspace;

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
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        web_preview_view::WebPreviewView::register(workspace, window, cx);
    })
    .detach();
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "windows")))]
pub use web_preview_linux::init;
#[cfg(target_os = "macos")]
pub use web_preview_macos::init;

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    all(unix, not(target_os = "macos"), not(target_os = "windows"))
)))]
pub fn init(_: &mut gpui::App) {}
