pub(crate) fn launch_status_summary_label(summary: &str) -> String {
    nonblank_or(summary, "No launch status summary")
}

pub(crate) fn launch_status_next_action_label(next_action: &str) -> String {
    nonblank_or(next_action, "No launch status next action")
}

pub(crate) fn launch_status_command_label(command: &str, fallback: &'static str) -> String {
    nonblank_or(command, fallback)
}

pub(crate) fn launch_status_optional_summary(summary: &str) -> Option<String> {
    let summary = summary.trim();
    if summary.is_empty() {
        None
    } else {
        Some(summary.to_string())
    }
}

fn nonblank_or(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        launch_status_command_label, launch_status_next_action_label,
        launch_status_optional_summary, launch_status_summary_label,
    };

    #[test]
    fn launch_status_labels_trim_nonblank_receipt_text() {
        assert_eq!(
            launch_status_summary_label("  Ready for launch review  "),
            "Ready for launch review"
        );
        assert_eq!(
            launch_status_command_label("  dx www templates --json  ", "No template command"),
            "dx www templates --json"
        );
    }

    #[test]
    fn launch_status_labels_fall_back_for_blank_receipt_text() {
        assert_eq!(launch_status_summary_label(""), "No launch status summary");
        assert_eq!(
            launch_status_next_action_label("  "),
            "No launch status next action"
        );
        assert_eq!(
            launch_status_command_label("\t", "No launch status command"),
            "No launch status command"
        );
    }

    #[test]
    fn launch_status_optional_summary_ignores_blank_text() {
        assert_eq!(launch_status_optional_summary("  "), None);
        assert_eq!(
            launch_status_optional_summary("  Review redaction flags  "),
            Some("Review redaction flags".to_string())
        );
    }
}
