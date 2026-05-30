use gpui::{
    Action, App, Context, EventEmitter, FocusHandle, Focusable, IntoElement, Render, WeakEntity,
};
use gpui::{Window, px};
use ui::IconName;
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};
use zed_actions::dx_style::TogglePanel;

use super::{active_context, dx_style_panel_snapshot, panel_view};

const DX_STYLE_PANEL_KEY: &str = "dx_style_panel";
const DEFAULT_PANEL_WIDTH: gpui::Pixels = px(360.0);
const MIN_PANEL_WIDTH: gpui::Pixels = px(280.0);

pub(crate) fn init(cx: &mut App) {
    cx.observe_new(
        |workspace: &mut Workspace, window, cx: &mut Context<Workspace>| {
            let Some(window) = window else {
                return;
            };

            workspace.register_action(|workspace, _: &TogglePanel, window, cx| {
                ensure_panel(workspace, window, cx);
                workspace.toggle_panel_focus::<DxStylePanel>(window, cx);
            });

            ensure_panel(workspace, window, cx);
        },
    )
    .detach();
}

fn ensure_panel(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    if workspace.panel::<DxStylePanel>(cx).is_some() {
        return;
    }

    let weak_workspace = workspace.weak_handle();
    let panel = cx.new(|cx| DxStylePanel::new(weak_workspace, cx));
    workspace.add_panel(panel, window, cx);
}

pub(crate) struct DxStylePanel {
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
}

impl DxStylePanel {
    fn new(workspace: WeakEntity<Workspace>, cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
        }
    }

    fn active_style_context(&self, cx: &App) -> active_context::ActiveStyleContextSnapshot {
        active_context::active_style_context(&self.workspace, cx)
    }
}

impl Focusable for DxStylePanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for DxStylePanel {}

impl Panel for DxStylePanel {
    fn persistent_name() -> &'static str {
        "DxStylePanel"
    }

    fn panel_key() -> &'static str {
        DX_STYLE_PANEL_KEY
    }

    fn position(&self, _: &Window, _: &App) -> DockPosition {
        DockPosition::Right
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        position == DockPosition::Right
    }

    fn set_position(&mut self, _: DockPosition, _: &mut Window, _: &mut Context<Self>) {}

    fn default_size(&self, _: &Window, _: &App) -> gpui::Pixels {
        DEFAULT_PANEL_WIDTH
    }

    fn min_size(&self, _: &Window, _: &App) -> Option<gpui::Pixels> {
        Some(MIN_PANEL_WIDTH)
    }

    fn icon(&self, _: &Window, _: &App) -> Option<IconName> {
        Some(IconName::Sliders)
    }

    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("DX Style")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        TogglePanel.boxed_clone()
    }

    fn starts_open(&self, _: &Window, _: &App) -> bool {
        false
    }

    fn activation_priority(&self) -> u32 {
        80
    }
}

impl Render for DxStylePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = dx_style_panel_snapshot();
        let active_context = self.active_style_context(cx);

        panel_view::render_panel(&snapshot, &active_context, cx)
    }
}
