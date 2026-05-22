pub(crate) fn checked_paths_label(paths: &[String]) -> String {
    match nonblank_count(paths) {
        0 => "No checked paths in receipt".to_string(),
        1 => "1 path".to_string(),
        count => format!("{count} paths"),
    }
}

pub(crate) fn skipped_checks_label(skipped: &[String]) -> String {
    match nonblank_count(skipped) {
        0 => "No skipped expensive checks".to_string(),
        1 => "1 skipped".to_string(),
        count => format!("{count} skipped"),
    }
}

fn nonblank_count(values: &[String]) -> usize {
    values
        .iter()
        .filter(|value| !value.trim().is_empty())
        .count()
}

pub(crate) fn check_outcome_label(
    pass_count: Option<u32>,
    fail_count: Option<u32>,
    warn_count: Option<u32>,
    skipped_count: Option<u32>,
) -> String {
    let mut parts = Vec::new();
    if let Some(count) = pass_count {
        parts.push(format!("{count} pass"));
    }
    if let Some(count) = fail_count {
        parts.push(format!("{count} fail"));
    }
    if let Some(count) = warn_count {
        parts.push(format!("{count} warn"));
    }
    if let Some(count) = skipped_count {
        parts.push(format!("{count} skipped"));
    }

    if parts.is_empty() {
        "No outcome counts in receipt".to_string()
    } else {
        parts.join(" / ")
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn path_and_skip_labels_cover_empty_single_and_plural() {
        let no_paths = "No checked paths in receipt";
        let no_skips = "No skipped expensive checks";

        assert_eq!(checked_paths_label(&[]), no_paths);
        assert_eq!(checked_paths_label(&strings(&["", "  "])), no_paths);
        assert_eq!(checked_paths_label(&strings(&["G:/Dx"])), "1 path");
        assert_eq!(checked_paths_label(&strings(&[" G:/Dx ", ""])), "1 path");
        assert_eq!(
            checked_paths_label(&strings(&["G:/Dx", "G:/Dx/www"])),
            "2 paths"
        );

        assert_eq!(skipped_checks_label(&[]), no_skips);
        assert_eq!(skipped_checks_label(&strings(&["", "  "])), no_skips);
        assert_eq!(skipped_checks_label(&strings(&["lighthouse"])), "1 skipped");
        assert_eq!(
            skipped_checks_label(&strings(&[" lighthouse ", ""])),
            "1 skipped"
        );
        assert_eq!(
            skipped_checks_label(&strings(&["lighthouse", "e2e"])),
            "2 skipped"
        );
    }

    #[test]
    fn outcome_label_preserves_zero_counts_and_missing_counts() {
        assert_eq!(
            check_outcome_label(None, None, None, None),
            "No outcome counts in receipt"
        );
        assert_eq!(
            check_outcome_label(Some(7), Some(0), Some(2), Some(1)),
            "7 pass / 0 fail / 2 warn / 1 skipped"
        );
    }

    #[test]
    fn duration_label_has_millisecond_and_second_boundaries() {
        assert_eq!(check_duration_label(None), "No duration in receipt");
        assert_eq!(check_duration_label(Some(0)), "0 ms");
        assert_eq!(check_duration_label(Some(999)), "999 ms");
        assert_eq!(check_duration_label(Some(1_500)), "1.5 s");
    }

    #[test]
    fn last_run_label_does_not_duplicate_generated_timestamp() {
        assert_eq!(
            last_run_label_with_generated_at(
                "Last run Unix ms: 1779400000000",
                Some(1_779_400_000_000)
            ),
            "Last run Unix ms: 1779400000000"
        );
        assert_eq!(
            last_run_label_with_generated_at("2 minutes ago", Some(1_779_400_000_000)),
            "2 minutes ago (1779400000000)"
        );
    }

    #[test]
    fn last_run_label_uses_generated_timestamp_when_label_is_blank() {
        assert_eq!(
            last_run_label_with_generated_at("   ", Some(1_779_400_000_000)),
            "Last run Unix ms: 1779400000000"
        );
        assert_eq!(last_run_label_with_generated_at("   ", None), "Never");
    }

    #[test]
    fn last_run_label_trims_nonblank_receipt_labels() {
        assert_eq!(
            last_run_label_with_generated_at("  2 minutes ago  ", None),
            "2 minutes ago"
        );
    }
}
