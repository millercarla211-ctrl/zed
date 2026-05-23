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
