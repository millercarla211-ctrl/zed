use gpui::{Action, AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};
use zed_actions::dx_style::OpenGeneratorPreview;

use crate::dx_style_panel::{DxStylePanelRow, DxStylePanelSnapshot};

use super::{bounded_items, metric_row, muted_card, signal_row};

pub(super) fn dx_style_panel_state(snapshot: &DxStylePanelSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(metric_row("Next", snapshot.next_action.clone()))
        .child(metric_row(
            "Root",
            if snapshot.root_exists {
                snapshot.root.display().to_string()
            } else {
                "missing".to_string()
            },
        ))
        .child(metric_row(
            "Generators",
            format!("{} planned", snapshot.visual_generator_count),
        ))
        .child(metric_row(
            "Host",
            if snapshot.web_preview_bridge_ready {
                "Web Preview ready"
            } else if snapshot.web_preview_host_present {
                "Web Preview host present"
            } else {
                "Web Preview host missing"
            },
        ))
        .child(metric_row("Readiness", snapshot.readiness.status.clone()))
        .child(
            Label::new(snapshot.readiness.summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Docs",
            format!(
                "{} / {}",
                snapshot.readiness.docs_ready, snapshot.readiness.docs_expected
            ),
        ))
        .child(metric_row(
            "Contracts",
            format!(
                "{} / {}",
                snapshot.readiness.contracts_ready, snapshot.readiness.contracts_expected
            ),
        ))
        .child(metric_row(
            "Fixtures",
            format!(
                "{} / {}",
                snapshot.readiness.fixtures_ready, snapshot.readiness.fixtures_expected
            ),
        ))
        .child(metric_row(
            "Artifacts",
            format!(
                "{} / {}",
                snapshot.readiness.artifacts_ready, snapshot.readiness.artifacts_expected
            ),
        ))
        .child(metric_row(
            "Receipts",
            snapshot.readiness.receipt_count.to_string(),
        ))
        .child(
            Button::new(
                "dx-style-open-generator-preview",
                "Open Web Preview Generators",
            )
            .full_width()
            .label_size(LabelSize::XSmall)
            .color(Color::Muted)
            .start_icon(Icon::new(IconName::Sliders).size(IconSize::XSmall))
            .disabled(!snapshot.web_preview_bridge_ready)
            .on_click(|_, window, cx| {
                window.dispatch_action(OpenGeneratorPreview.boxed_clone(), cx);
            }),
        );

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing dx-style root: {}", snapshot.root.display()),
            cx,
        ));
    } else if !snapshot.grouped_contract_ready {
        stack = stack.child(signal_row(
            "dx-style-contract-warning".into(),
            IconName::Warning,
            Color::Warning,
            "Grouped-class contract is not ready for editor writes",
        ));
    }

    stack = stack
        .child(style_contract_row(
            "Plan",
            snapshot.plan_present,
            &snapshot.plan_path,
        ))
        .child(style_contract_row(
            "Group Contract",
            snapshot.grouped_contract_present,
            &snapshot.grouped_contract_path,
        ))
        .child(style_contract_row(
            "Generator Catalog",
            snapshot.generator_catalog_present,
            &snapshot.generator_catalog_path,
        ))
        .child(style_contract_row(
            "Editor Contract",
            snapshot.editor_contract_present,
            &snapshot.editor_contract_path,
        ))
        .child(style_contract_row(
            "Web Preview Host",
            snapshot.web_preview_host_present,
            &snapshot.web_preview_host_path,
        ));

    for (ix, row) in snapshot.rows.iter().take(7).enumerate() {
        stack = stack.child(dx_style_row(
            SharedString::from(format!("dx-style-row-{ix}")),
            row,
            cx,
        ));
    }

    stack = stack
        .child(metric_row(
            "Readiness Contracts",
            bounded_items(
                &snapshot.readiness.contract_rows,
                3,
                "No DX Style contract rows",
            ),
        ))
        .child(metric_row(
            "Readiness Fixtures",
            bounded_items(
                &snapshot.readiness.fixture_rows,
                3,
                "No DX Style fixture rows",
            ),
        ))
        .child(metric_row(
            "Receipt Roots",
            bounded_items(
                &snapshot.readiness.receipt_rows,
                2,
                "No DX Style receipt roots",
            ),
        ))
        .child(metric_row(
            "Readiness Next",
            snapshot.readiness.next_action.clone(),
        ));

    if snapshot.readiness.receipt_count == 0 {
        stack = stack.child(muted_card(
            "No dx style build/check receipt has been read by Zed.",
            cx,
        ));
    }

    if !snapshot.readiness.missing_rows.is_empty() {
        stack = stack.child(metric_row(
            "Missing",
            bounded_items(
                &snapshot.readiness.missing_rows,
                3,
                "No missing DX Style readiness files",
            ),
        ));
    }

    for (ix, warning) in snapshot.warnings.iter().take(2).enumerate() {
        stack = stack.child(signal_row(
            SharedString::from(format!("dx-style-warning-{ix}")),
            IconName::Info,
            Color::Muted,
            warning.clone(),
        ));
    }

    stack.into_any_element()
}

fn style_contract_row(label: &'static str, present: bool, path: &std::path::Path) -> AnyElement {
    metric_row(
        label,
        if present {
            path.display().to_string()
        } else {
            "missing".to_string()
        },
    )
}

fn dx_style_row(id: SharedString, row: &DxStylePanelRow, cx: &App) -> AnyElement {
    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(row.label.clone(), row.state.clone()))
        .child(
            Label::new(row.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}
