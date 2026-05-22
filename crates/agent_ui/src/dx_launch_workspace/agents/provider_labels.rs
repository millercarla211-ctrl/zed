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

pub(crate) fn provider_detail_label(id: &str, compatibility: &[String]) -> String {
    let id = nonblank_or(id, "unknown-provider");
    let compatibility = compatibility_label(compatibility);

    if compatibility.is_empty() {
        id
    } else {
        format!("{id} - {compatibility}")
    }
}

pub(crate) fn model_detail_label(provider_id: &str, id: &str, compatibility: &[String]) -> String {
    let provider_id = nonblank_or(provider_id, "unknown-provider");
    let id = nonblank_or(id, "unknown-model");
    let compatibility = compatibility_label(compatibility);

    if compatibility.is_empty() {
        format!("{provider_id} / {id}")
    } else {
        format!("{provider_id} / {id} - {compatibility}")
    }
}

fn compatibility_label(compatibility: &[String]) -> String {
    let values = compatibility
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let shown = values
        .iter()
        .take(3)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");

    match values.len().saturating_sub(3) {
        0 => shown,
        hidden => format!("{shown} (+{hidden} more)"),
    }
}

fn nonblank_or(value: &str, fallback: &'static str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn provider_state_label_uses_priority_order() {
        assert_eq!(
            provider_state_label(true, false, false, "offline"),
            "Active"
        );
        assert_eq!(
            provider_state_label(false, true, false, "offline"),
            "Configured"
        );
        assert_eq!(provider_state_label(false, false, true, "offline"), "Local");
        assert_eq!(
            provider_state_label(false, false, false, "  offline "),
            "offline"
        );
        assert_eq!(provider_state_label(false, false, false, "  "), "Unknown");
    }

    #[test]
    fn model_state_label_uses_active_or_status() {
        assert_eq!(model_state_label(true, "offline"), "Active");
        assert_eq!(model_state_label(false, "  available "), "available");
        assert_eq!(model_state_label(false, ""), "Unknown");
    }

    #[test]
    fn provider_detail_label_trims_blank_compatibility() {
        assert_eq!(
            provider_detail_label(" openai ", &strings(&["", " chat ", "  ", "tools"])),
            "openai - chat, tools"
        );
        assert_eq!(
            provider_detail_label("", &strings(&["  "])),
            "unknown-provider"
        );
    }

    #[test]
    fn model_detail_label_falls_back_for_blank_ids() {
        assert_eq!(
            model_detail_label(" openai ", " gpt ", &strings(&["chat"])),
            "openai / gpt - chat"
        );
        assert_eq!(
            model_detail_label(" ", "", &strings(&["", "  "])),
            "unknown-provider / unknown-model"
        );
    }

    #[test]
    fn detail_labels_disclose_compatibility_overflow() {
        assert_eq!(
            provider_detail_label(
                "openai",
                &strings(&["chat", "tools", "vision", "local", ""])
            ),
            "openai - chat, tools, vision (+1 more)"
        );
    }
}
