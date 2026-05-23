use super::{RECEIPT_EXPIRED_AFTER_MS, RECEIPT_STALE_AFTER_MS};

pub(super) fn launch_receipt_operator_summary(
    status: &str,
    malformed_count: usize,
    latest_present: bool,
    snapshot_count: usize,
    latest_freshness: Option<&str>,
) -> String {
    if status == "ready" {
        return format!(
            "Launch receipts ready: latest present, {snapshot_count} snapshots retained."
        );
    }

    if malformed_count > 0 {
        return format!(
            "Launch receipts warning: {malformed_count} malformed, latest_present={latest_present}."
        );
    }

    if let Some(freshness) = latest_freshness {
        return format!("Launch receipts warning: latest freshness={freshness}.");
    }

    "Launch receipts warning: review cached launch status metadata before handoff.".to_string()
}

pub(super) fn freshness_state(malformed: bool, age_ms: Option<u64>) -> &'static str {
    if malformed {
        return "malformed";
    }

    match age_ms {
        Some(age_ms) if age_ms > RECEIPT_EXPIRED_AFTER_MS => "expired",
        Some(age_ms) if age_ms > RECEIPT_STALE_AFTER_MS => "stale",
        Some(_) => "fresh",
        None => "unknown",
    }
}
