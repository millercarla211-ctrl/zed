pub(super) fn nonblank_or(value: &str, fallback: &'static str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}
