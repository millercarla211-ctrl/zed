use std::path::Path;

pub(crate) fn receipt_file_label(root: &Path, path: &Path) -> Option<String> {
    if !has_json_extension(path) {
        return None;
    }

    Some(
        path.strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string(),
    )
}

fn has_json_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn receipt_file_label_accepts_uppercase_json_extension() {
        assert_eq!(
            receipt_file_label(Path::new("root"), Path::new("root/status-latest.JSON")),
            Some("status-latest.JSON".to_string())
        );
    }

    #[test]
    fn receipt_file_label_rejects_non_json_extension() {
        assert_eq!(
            receipt_file_label(Path::new("root"), Path::new("root/status-latest.txt")),
            None
        );
    }

    #[test]
    fn receipt_file_label_falls_back_to_absolute_path_when_outside_root() {
        assert_eq!(
            receipt_file_label(Path::new("root"), Path::new("other/status-latest.json")),
            Some("other/status-latest.json".to_string())
        );
    }
}
