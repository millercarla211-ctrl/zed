use super::text::nonblank_or;

pub(crate) fn provider_state_label(
    active: bool,
    configured: bool,
    local: bool,
    status: &str,
) -> String {
    if active {
        "Active".to_string()
    } else if configured {
        "Configured".to_string()
    } else if local {
        "Local".to_string()
    } else {
        nonblank_or(status, "Unknown")
    }
}

pub(crate) fn model_state_label(active: bool, status: &str) -> String {
    if active {
        "Active".to_string()
    } else {
        nonblank_or(status, "Unknown")
    }
}
