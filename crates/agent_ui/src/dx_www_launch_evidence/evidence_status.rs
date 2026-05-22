pub(crate) fn evidence_status_label(passed: Option<bool>, finding_count: usize) -> &'static str {
    match passed {
        Some(false) => "blocked",
        Some(true) if finding_count > 0 => "warning",
        Some(true) => "ready",
        None if finding_count > 0 => "warning",
        None => "ready",
    }
}

#[cfg(test)]
mod tests {
    use super::evidence_status_label;

    #[test]
    fn status_label_blocks_failed_packets() {
        assert_eq!(evidence_status_label(Some(false), 0), "blocked");
        assert_eq!(evidence_status_label(Some(false), 2), "blocked");
    }

    #[test]
    fn status_label_warns_when_passed_packet_has_findings() {
        assert_eq!(evidence_status_label(Some(true), 1), "warning");
    }

    #[test]
    fn status_label_warns_when_findings_exist_without_passed_flag() {
        assert_eq!(evidence_status_label(None, 3), "warning");
    }

    #[test]
    fn status_label_defaults_ready_for_empty_metadata() {
        assert_eq!(evidence_status_label(Some(true), 0), "ready");
        assert_eq!(evidence_status_label(None, 0), "ready");
    }
}
