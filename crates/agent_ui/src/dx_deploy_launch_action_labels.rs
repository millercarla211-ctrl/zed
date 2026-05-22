pub(crate) fn launch_action_detail_parts(
    id: Option<&str>,
    risk_level: Option<&str>,
    requires_user_approval: Option<bool>,
    writes_receipts: Option<bool>,
    has_command: bool,
) -> Vec<String> {
    let mut detail = Vec::new();

    if let Some(id) = id {
        detail.push(format!("id {id}"));
    }

    detail.push(format!("risk {}", risk_level.unwrap_or("unknown")));
    detail.push(approval_state_label(requires_user_approval).to_string());
    detail.push(receipt_write_state_label(writes_receipts).to_string());

    if !has_command {
        detail.push("metadata only".to_string());
    }

    detail
}

fn approval_state_label(requires_user_approval: Option<bool>) -> &'static str {
    match requires_user_approval {
        Some(true) => "approval required",
        Some(false) => "no approval required",
        None => "approval unknown",
    }
}

fn receipt_write_state_label(writes_receipts: Option<bool>) -> &'static str {
    match writes_receipts {
        Some(true) => "writes receipts",
        Some(false) => "read-only",
        None => "receipt write unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_parts_show_safe_read_only_actions_explicitly() {
        assert_eq!(
            launch_action_detail_parts(
                Some("quick-fix-1"),
                Some("receipt-write"),
                Some(false),
                Some(false),
                true
            ),
            vec![
                "id quick-fix-1",
                "risk receipt-write",
                "no approval required",
                "read-only"
            ]
        );
    }

    #[test]
    fn detail_parts_show_unknown_metadata_actions_explicitly() {
        assert_eq!(
            launch_action_detail_parts(None, None, None, None, false),
            vec![
                "risk unknown",
                "approval unknown",
                "receipt write unknown",
                "metadata only"
            ]
        );
    }
}
