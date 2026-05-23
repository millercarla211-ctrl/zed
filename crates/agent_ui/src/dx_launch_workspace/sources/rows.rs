use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{Color, IconName, prelude::*};

use crate::dx_source_sets::{DxSourceItem, DxSourceKind, DxSourceReceiptDrilldown};

use super::super::{metric_row, signal_row};

pub(super) fn source_item_row(
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

fn source_kind_icon(kind: DxSourceKind) -> IconName {
    match kind {
        DxSourceKind::WorkspaceRoot => IconName::Folder,
        DxSourceKind::MetasearchSourcePack => IconName::FileTextOutlined,
        DxSourceKind::ReducedContextReceipt => IconName::FileTextOutlined,
        DxSourceKind::MediaOutput => IconName::File,
        DxSourceKind::ForgeRestorePreview => IconName::Archive,
    }
}
