use gpui::App;
#[cfg(target_os = "macos")]
use gpui::actions;
#[cfg(target_os = "macos")]
use workspace::Workspace;

#[cfg(target_os = "macos")]
pub(crate) mod macos_host;
#[cfg(target_os = "macos")]
pub mod web_preview_view;

#[cfg(target_os = "macos")]
actions!(
    web_preview,
    [
        /// Opens a web preview for the current workspace.
        OpenPreview,
        /// Opens a web preview in a split pane.
        OpenPreviewToTheSide,
    ]
);

#[cfg(target_os = "macos")]
pub fn init(cx: &mut App) {
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

#[cfg(not(target_os = "macos"))]
pub fn init(_cx: &mut App) {}
