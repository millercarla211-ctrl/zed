use gpui::{App, actions};
use workspace::Workspace;

pub mod web_preview_view;
#[cfg(target_os = "windows")]
pub(crate) mod windows_visual_webview;

actions!(
    web_preview,
    [
        /// Opens a web preview for the current workspace.
        OpenPreview,
        /// Opens a web preview in a split pane.
        OpenPreviewToTheSide,
    ]
);

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        web_preview_view::WebPreviewView::register(workspace, window, cx);
    })
    .detach();
}
