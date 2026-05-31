use gpui::{Action, App, IntoElement, SharedString};
use ui::{IconName, prelude::*};
use zed_actions::dx_style::OpenGeneratorPreviewForContext;

use super::{
    DxStylePanelRow, DxStylePanelSnapshot, active_context::ActiveStyleContextSnapshot,
    panel_metric::metric,
};
const STYLE_PANEL_ROW_LIMIT: usize = 13;
pub(super) fn render_panel(
    snapshot: &DxStylePanelSnapshot,
    active_context: &ActiveStyleContextSnapshot,
    cx: &App,
) -> impl IntoElement {
    let source_context_json = active_context.web_preview_context_json();
    let can_open_generator =
        snapshot.web_preview_bridge_ready && active_context.can_open_generator();
    v_flex()
        .id("dx-style-panel")
        .size_full()
        .min_w_0()
        .gap_2()
        .p_2()
        .bg(cx.theme().colors().panel_background)
        .child(panel_header())
        .child(style_summary(snapshot, active_context, cx))
        .child(
            Button::new(
                "dx-style-panel-open-generator-preview",
                "Open Web Preview Generators",
            )
            .full_width()
            .label_size(LabelSize::Small)
            .color(Color::Muted)
            .start_icon(Icon::new(IconName::Sliders).size(IconSize::Small))
            .disabled(!can_open_generator)
            .on_click(move |_, window, cx| {
                window.dispatch_action(
                    OpenGeneratorPreviewForContext {
                        source_context_json: source_context_json.clone(),
                    }
                    .boxed_clone(),
                    cx,
                );
            }),
        )
        .child(style_rows(snapshot, cx))
}
fn panel_header() -> impl IntoElement {
    h_flex().justify_between().gap_2().child(
        h_flex()
            .gap_1()
            .child(Icon::new(IconName::Sliders).size(IconSize::Small))
            .child(Label::new("DX Style").size(LabelSize::Small)),
    )
}
fn style_summary(
    snapshot: &DxStylePanelSnapshot,
    active_context: &ActiveStyleContextSnapshot,
    cx: &App,
) -> impl IntoElement {
    let gate = &active_context.apply_gate;
    let generator_count = format!("{} planned", snapshot.visual_generator_count);
    let preflight_source = gate.editor_write_bridge.preflight_source_label.clone();
    let mismatch = gate
        .receipt_mismatch
        .as_ref()
        .and_then(|summary| summary.reasons.first())
        .cloned();
    v_flex()
        .gap_1()
        .rounded_sm()
        .p_2()
        .bg(cx.theme().colors().element_background)
        .child(metric("Status", snapshot.status.clone()))
        .child(metric("Next", snapshot.next_action.clone()))
        .child(metric(
            "Host",
            if snapshot.web_preview_bridge_ready {
                "Web Preview ready".to_string()
            } else if snapshot.web_preview_host_present {
                "Web Preview host present".to_string()
            } else {
                "Web Preview host missing".to_string()
            },
        ))
        .child(metric("Generators", generator_count))
        .child(metric("Context", active_context.status.clone()))
        .when_some(active_context.source_state.clone(), |this, state| {
            this.child(metric("Source", state))
        })
        .when_some(active_context.context_kind.clone(), |this, kind| {
            this.child(metric("Kind", kind))
        })
        .when_some(active_context.token.clone(), |this, token| {
            this.child(metric("Token", token))
        })
        .when_some(active_context.css_property.clone(), |this, property| {
            this.child(metric("CSS", property))
        })
        .when_some(active_context.css_generator.clone(), |this, generator| {
            this.child(metric("Generator", generator))
        })
        .when_some(
            active_context.css_source_edit_safety.clone(),
            |this, safety| this.child(metric("CSS safety", safety)),
        )
        .when(active_context.attribute_tokens.len() > 1, |this| {
            this.child(metric(
                "Class list",
                format!("{} token(s)", active_context.attribute_tokens.len()),
            ))
        })
        .when_some(active_context.group_context.summary(), |this, group| {
            this.child(metric("Group", group))
        })
        .when_some(active_context.source_path.clone(), |this, path| {
            this.child(metric("Path", path))
        })
        .when_some(active_context.span.clone(), |this, span| {
            this.child(metric("Span", span))
        })
        .when_some(active_context.span_byte_range(), |this, span| {
            this.child(metric("Span bytes", span))
        })
        .child(metric("Apply", gate.state.clone()))
        .child(metric("Match", gate.receipt_match.clone()))
        .child(metric("Bridge", gate.editor_write_bridge.summary.clone()))
        .child(metric("Preflight", preflight_source))
        .child(metric("Gate", gate.reason.clone()))
        .when_some(mismatch, |this, reason| {
            this.child(metric("Mismatch", reason))
        })
        .when_some(gate.receipt_summary.clone(), |this, receipt| {
            this.child(metric(
                "Receipt",
                format!("{} / {} edit(s)", receipt.intent, receipt.edit_count),
            ))
            .child(metric("Review", receipt.message))
            .when_some(receipt.edits.first().cloned(), |this, edit| {
                this.child(metric("Patch", edit))
            })
        })
        .child(
            Label::new(active_context.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
}
fn style_rows(snapshot: &DxStylePanelSnapshot, cx: &App) -> impl IntoElement {
    let mut stack = v_flex().gap_1().min_w_0();
    for (ix, row) in snapshot.rows.iter().take(STYLE_PANEL_ROW_LIMIT).enumerate() {
        stack = stack.child(style_row(
            SharedString::from(format!("dx-style-panel-row-{ix}")),
            row,
            cx,
        ));
    }
    for (ix, warning) in snapshot.warnings.iter().take(3).enumerate() {
        stack = stack.child(
            h_flex()
                .id(SharedString::from(format!("dx-style-panel-warning-{ix}")))
                .gap_1()
                .rounded_sm()
                .p_1()
                .bg(cx.theme().colors().element_background)
                .child(
                    Icon::new(IconName::Info)
                        .size(IconSize::XSmall)
                        .color(Color::Muted),
                )
                .child(
                    Label::new(warning.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .truncate(),
                ),
        );
    }

    stack
}
fn style_row(id: SharedString, row: &DxStylePanelRow, cx: &App) -> impl IntoElement {
    v_flex()
        .id(id)
        .gap_0p5()
        .rounded_sm()
        .p_1()
        .bg(cx.theme().colors().element_background)
        .child(metric(row.label.clone(), row.state.clone()))
        .child(
            Label::new(row.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
}
