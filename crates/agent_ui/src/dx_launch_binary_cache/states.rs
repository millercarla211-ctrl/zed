use dx_catalog::DxReceiptCacheHealth;

pub(super) fn cache_health_state(health: DxReceiptCacheHealth) -> &'static str {
    match health {
        DxReceiptCacheHealth::Ready => "ready",
        DxReceiptCacheHealth::Partial => "partial",
        DxReceiptCacheHealth::Stale => "stale",
        DxReceiptCacheHealth::Expired => "expired",
        DxReceiptCacheHealth::Malformed => "malformed",
        DxReceiptCacheHealth::MissingRoots => "missing-roots",
        DxReceiptCacheHealth::Empty => "empty",
        DxReceiptCacheHealth::Unknown => "unknown",
    }
}

pub(super) fn binary_cache_state_from_artifact(state: &str) -> bool {
    matches!(
        state,
        "ready" | "partial" | "stale" | "expired" | "malformed" | "missing-roots" | "empty"
    )
}

pub(super) fn binary_cache_state_needs_attention(state: &str) -> bool {
    matches!(
        state,
        "partial" | "expired" | "malformed" | "missing-roots" | "unknown"
    )
}

pub(super) fn provider_catalog_state(present: bool, stale: bool) -> &'static str {
    if present && stale {
        "stale"
    } else if present {
        "ready"
    } else {
        "waiting"
    }
}
