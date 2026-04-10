mod backgrounds;
mod element;
mod liquid_glass_view;
mod ui_state;

use gpui::App;
use workspace::Workspace;

pub use element::{LiquidGlassStyle, liquid_glass_layer, paint_liquid_glass_layer};
pub use liquid_glass_view::LiquidGlassView;

pub fn init(cx: &mut App) {
    workspace::register_serializable_item::<LiquidGlassView>(cx);

    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };

        LiquidGlassView::register(workspace, window, cx);
    })
    .detach();
}
