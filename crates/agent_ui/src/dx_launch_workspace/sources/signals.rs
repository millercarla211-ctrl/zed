use gpui::{AnyElement, SharedString};
use ui::{Color, IconName};

use crate::dx_source_sets::DxSourceItem;

use super::super::signal_row;

pub(super) fn source_signal_rows(source: &DxSourceItem) -> Vec<AnyElement> {
    let mut rows = Vec::new();

    for (ix, proof) in source.proofs.iter().take(2).enumerate() {
        rows.push(signal_row(
            SharedString::from(format!("source-proof-{}-{ix}", source.path)),
            IconName::Check,
            Color::Success,
            proof.clone(),
        ));
    }

    for (ix, warning) in source.warnings.iter().take(2).enumerate() {
        rows.push(signal_row(
            SharedString::from(format!("source-warning-{}-{ix}", source.path)),
            IconName::Warning,
            Color::Warning,
            warning.clone(),
        ));
    }

    rows
}
