pub(crate) fn check_duration_label(duration_ms: Option<u64>) -> String {
    match duration_ms {
        Some(0) => "0 ms".to_string(),
        Some(value) if value < 1_000 => format!("{value} ms"),
        Some(value) => format!("{:.1} s", value as f64 / 1_000.0),
        None => "No duration in receipt".to_string(),
    }
}

pub(crate) fn last_run_label_with_generated_at(
    last_run_label: &str,
    generated_at_unix_ms: Option<u64>,
) -> String {
    let label = last_run_label.trim();
    if let Some(generated_at) = generated_at_unix_ms {
        let generated_at = generated_at.to_string();
        if label.is_empty() {
            format!("Last run Unix ms: {generated_at}")
        } else if label.contains(&generated_at) {
            label.to_string()
        } else {
            format!("{label} ({generated_at})")
        }
    } else if label.is_empty() {
        "Never".to_string()
    } else {
        label.to_string()
    }
}
