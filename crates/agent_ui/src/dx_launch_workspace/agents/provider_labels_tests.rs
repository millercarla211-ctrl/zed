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
