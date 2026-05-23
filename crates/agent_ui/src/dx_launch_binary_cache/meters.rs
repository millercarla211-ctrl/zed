use super::{DxBinaryCacheRow, artifacts::ReceiptCacheArtifactState, states::cache_health_state};
use dx_catalog::{DxReceiptCacheEntryKind, DxReceiptCacheHealth, DxReceiptCacheKindSummary};
use std::path::Path;

pub(super) fn metering_row_from_artifact(
    path: &Path,
    artifact: &ReceiptCacheArtifactState,
) -> Option<DxBinaryCacheRow> {
    let ReceiptCacheArtifactState::Ready(manifest) = artifact else {
        return None;
    };

    let token = manifest.kind_summary(DxReceiptCacheEntryKind::Tokens);
    let rlm = manifest.kind_summary(DxReceiptCacheEntryKind::Rlm);
    let serializer = manifest.kind_summary(DxReceiptCacheEntryKind::Serializer);
    let state = combined_meter_health(&[&token, &rlm, &serializer]);

    Some(DxBinaryCacheRow {
        label: "Token/tool meters".to_string(),
        state: cache_health_state(state).to_string(),
        path: path.display().to_string(),
        detail: format!(
            "tokens: {}; rlm: {}; serializer: {}",
            kind_summary_detail(&token),
            kind_summary_detail(&rlm),
            kind_summary_detail(&serializer)
        ),
        present: true,
    })
}

pub(super) fn kind_summary_detail(summary: &DxReceiptCacheKindSummary) -> String {
    format!(
        "{} entry(s), {} fresh, {} stale, {} malformed",
        summary.entry_count,
        summary.fresh_entry_count,
        summary.stale_entry_count,
        summary.malformed_entry_count
    )
}

fn combined_meter_health(summaries: &[&DxReceiptCacheKindSummary]) -> DxReceiptCacheHealth {
    if summaries
        .iter()
        .any(|summary| summary.health() == DxReceiptCacheHealth::Malformed)
    {
        return DxReceiptCacheHealth::Malformed;
    }
    if summaries.iter().all(|summary| summary.entry_count == 0) {
        return DxReceiptCacheHealth::Empty;
    }
    if summaries
        .iter()
        .any(|summary| summary.health() == DxReceiptCacheHealth::Expired)
    {
        return DxReceiptCacheHealth::Expired;
    }
    if summaries
        .iter()
        .any(|summary| summary.health() == DxReceiptCacheHealth::Stale)
    {
        return DxReceiptCacheHealth::Stale;
    }
    DxReceiptCacheHealth::Ready
}
