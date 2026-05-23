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
