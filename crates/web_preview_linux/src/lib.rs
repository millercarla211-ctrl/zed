use gpui::App;
#[cfg(target_os = "linux")]
use gpui::actions;
#[cfg(target_os = "linux")]
use workspace::Workspace;

#[cfg(target_os = "linux")]
pub(crate) mod wayland_host;
#[cfg(target_os = "linux")]
pub mod web_preview_view;
#[cfg(target_os = "linux")]
pub(crate) mod x11_host;

#[cfg(target_os = "linux")]
actions!(
    web_preview,
    [
        /// Opens a web preview for the current workspace.
        OpenPreview,
        /// Opens a web preview in a split pane.
        OpenPreviewToTheSide,
    ]
);

#[cfg(target_os = "linux")]
pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        web_preview_view::WebPreviewView::register(workspace, window, cx);
    })
    .detach();
}

#[cfg(not(target_os = "linux"))]
pub fn init(_cx: &mut App) {}
