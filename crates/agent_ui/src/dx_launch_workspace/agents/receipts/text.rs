pub(super) fn receipt_optional_label(value: &str) -> Option<String> {
    let label = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if label.is_empty() { None } else { Some(label) }
}

pub(super) fn receipt_status_text(status: &str) -> String {
    receipt_label_text(status, "unknown")
}

pub(super) fn receipt_label_text(value: &str, fallback: &'static str) -> String {
    receipt_optional_label(value).unwrap_or_else(|| fallback.to_string())
}
