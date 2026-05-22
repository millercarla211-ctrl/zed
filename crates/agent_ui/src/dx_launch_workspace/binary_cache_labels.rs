pub(crate) fn binary_cache_summary_label(summary: &str) -> String {
    nonblank_or(summary, "No binary cache summary")
}

pub(crate) fn binary_cache_next_action_label(next_action: &str) -> String {
    nonblank_or(next_action, "No binary cache next action")
}

pub(crate) fn binary_cache_row_detail_label(detail: &str) -> String {
    nonblank_or(detail, "No binary cache detail")
}

pub(crate) fn binary_cache_row_path_label(path: &str) -> String {
    nonblank_or(path, "No binary cache path")
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
        binary_cache_next_action_label, binary_cache_row_detail_label, binary_cache_row_path_label,
        binary_cache_summary_label,
    };

    #[test]
    fn labels_trim_nonblank_receipt_text() {
        assert_eq!(
            binary_cache_summary_label("  JSON receipts authoritative  "),
            "JSON receipts authoritative"
        );
        assert_eq!(
            binary_cache_next_action_label("  Materialize artifacts  "),
            "Materialize artifacts"
        );
    }

    #[test]
    fn row_labels_fall_back_for_blank_receipt_fields() {
        assert_eq!(
            binary_cache_row_detail_label("  "),
            "No binary cache detail"
        );
        assert_eq!(binary_cache_row_path_label("\t"), "No binary cache path");
        assert_eq!(
            binary_cache_next_action_label(""),
            "No binary cache next action"
        );
    }

    #[test]
    fn labels_preserve_nonblank_row_fields() {
        assert_eq!(binary_cache_row_detail_label("3 receipts"), "3 receipts");
        assert_eq!(
            binary_cache_row_path_label(r"G:\Dx\.dx\receipts\receipt-cache.dxrc"),
            r"G:\Dx\.dx\receipts\receipt-cache.dxrc"
        );
    }
}
