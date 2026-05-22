use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_receipts::DxReceiptSnapshot;
use crate::dx_source_sets::{
    DxSourceAttachmentSummary, DxSourceItem, DxSourceKind, DxSourceReceiptDrilldown, DxSourceSet,
    DxSourceSetSnapshot,
};

use super::{DxSourceRowControl, metric_row, muted_card, signal_row, source_row};

pub(super) fn source_set_stack(
    snapshot: &DxSourceSetSnapshot,
    mut source_row_controls: Vec<DxSourceRowControl>,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex().gap_1();

    if snapshot.total_sources == 0 {
        stack = stack.child(muted_card("No workspace source", cx));
    } else {
        for (ix, set) in snapshot.sets.iter().enumerate() {
            stack = stack.child(source_set_card(
                SharedString::from(format!("source-set-{ix}")),
                set,
                &mut source_row_controls,
                cx,
            ));
        }
    }

    stack.into_any_element()
}

pub(super) fn source_attachment_state(summary: &DxSourceAttachmentSummary, cx: &App) -> AnyElement {
    let state = if summary.attachable_sources == 0 {
        "No attach-ready sources".to_string()
    } else {
        format!("{} ready", summary.attachable_sources)
    };

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Attach-ready", state))
        .child(metric_row(
            "Workspace roots",
            summary.workspace_roots.to_string(),
        ))
        .child(metric_row(
            "Managed receipts",
            summary.managed_receipts.to_string(),
        ));

    if summary.produced_files > 0 {
        stack = stack.child(metric_row(
            "Produced media",
            summary.produced_files.to_string(),
        ));
    }

    if summary.restore_previews > 0 {
        stack = stack.child(metric_row(
            "Restore previews",
            summary.restore_previews.to_string(),
        ));
    }

    if summary.attachable_sources == 0 {
        stack = stack.child(muted_card(
            "Create a source-pack or media receipt first",
            cx,
        ));
    }

    stack.into_any_element()
}

fn source_set_card(
    id: SharedString,
    set: &DxSourceSet,
    source_row_controls: &mut Vec<DxSourceRowControl>,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .id(id)
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .px_2()
        .py_1()
        .child(metric_row(set.label, set.status.clone()));

    if set.sources.is_empty() {
        return stack.into_any_element();
    }

    let set_id = set.label.to_ascii_lowercase().replace(' ', "-");
    for (ix, source) in set.sources.iter().take(3).enumerate() {
        let source_row_control = take_source_row_control(source_row_controls, &source.path);
        stack = stack.child(source_item_row(
            SharedString::from(format!("{set_id}-source-{ix}")),
            source,
            source_row_control,
            cx,
        ));
    }

    stack.into_any_element()
}

fn source_item_row(
    id: SharedString,
    source: &DxSourceItem,
    source_row_control: Option<AnyElement>,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(
            h_flex()
                .gap_1()
                .min_w_0()
                .items_center()
                .child(
                    Icon::new(source_kind_icon(source.kind))
                        .size(IconSize::XSmall)
                        .color(Color::Muted),
                )
                .child(
                    Label::new(source.label.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Default)
                        .truncate(),
                ),
        )
        .child(
            Label::new(source.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(source.path.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );

    if let Some(source_row_control) = source_row_control {
        stack = stack.child(source_row_control);
    }

    for (ix, receipt) in source.receipt_drilldowns.iter().take(2).enumerate() {
        stack = stack.child(source_receipt_drilldown_row(
            SharedString::from(format!("source-receipt-{}-{ix}", source.path)),
            receipt,
            cx,
        ));
    }

    for (ix, proof) in source.proofs.iter().take(2).enumerate() {
        stack = stack.child(signal_row(
            SharedString::from(format!("source-proof-{}-{ix}", source.path)),
            IconName::Check,
            Color::Success,
            proof.clone(),
        ));
    }

    for (ix, warning) in source.warnings.iter().take(2).enumerate() {
        stack = stack.child(signal_row(
            SharedString::from(format!("source-warning-{}-{ix}", source.path)),
            IconName::Warning,
            Color::Warning,
            warning.clone(),
        ));
    }

    stack.into_any_element()
}

fn source_receipt_drilldown_row(
    id: SharedString,
    receipt: &DxSourceReceiptDrilldown,
    cx: &App,
) -> AnyElement {
    let label_id = SharedString::from(format!("source-receipt-label-{}", receipt.detail));

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().editor_background)
        .child(signal_row(
            label_id,
            IconName::FileTextOutlined,
            Color::Muted,
            receipt.label.clone(),
        ))
        .child(
            Label::new(receipt.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn take_source_row_control(
    source_row_controls: &mut Vec<DxSourceRowControl>,
    source_path: &str,
) -> Option<AnyElement> {
    source_row_controls
        .iter()
        .position(|control| control.source_path == source_path)
        .map(|index| source_row_controls.remove(index).element)
}

fn source_kind_icon(kind: DxSourceKind) -> IconName {
    match kind {
        DxSourceKind::WorkspaceRoot => IconName::Folder,
        DxSourceKind::MetasearchSourcePack => IconName::FileTextOutlined,
        DxSourceKind::ReducedContextReceipt => IconName::FileTextOutlined,
        DxSourceKind::MediaOutput => IconName::File,
        DxSourceKind::ForgeRestorePreview => IconName::Archive,
    }
}

pub(super) fn receipt_source_state(snapshot: &DxReceiptSnapshot, cx: &mut App) -> AnyElement {
    if !snapshot.root_exists {
        return muted_card(
            format!("Receipts not found: {}", snapshot.root.display()),
            cx,
        );
    }

    let total = snapshot
        .buckets
        .iter()
        .map(|bucket| bucket.count)
        .sum::<usize>();
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Receipt files", total.to_string()));

    if snapshot.latest.is_empty() {
        stack = stack.child(muted_card("Waiting for first DX receipt", cx));
    } else {
        for (ix, label) in snapshot.latest.iter().enumerate() {
            stack = stack.child(source_row(
                SharedString::from(format!("latest-receipt-{ix}")),
                IconName::FileTextOutlined,
                label.clone(),
                cx,
            ));
        }
    }

    stack.into_any_element()
}
