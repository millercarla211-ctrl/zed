use std::path::Path;

pub(crate) fn deploy_root_key(path: &Path) -> String {
    deploy_root_key_for_platform(path)
}

#[cfg(windows)]
fn deploy_root_key_for_platform(path: &Path) -> String {
    let mut key = collapse_repeated_windows_separators(&path.to_string_lossy().replace('/', "\\"));
    while key.ends_with('\\') && key.len() > 3 {
        key.pop();
    }
    key.to_ascii_lowercase()
}

#[cfg(windows)]
fn collapse_repeated_windows_separators(path: &str) -> String {
    let mut collapsed = String::with_capacity(path.len());
    let mut previous_was_separator = false;
    let preserve_unc_prefix = path.starts_with(r"\\");

    for ch in path.chars() {
        if ch == '\\' {
            if previous_was_separator {
                if preserve_unc_prefix && collapsed == r"\" {
                    collapsed.push(ch);
                }
                continue;
            }
            collapsed.push(ch);
            previous_was_separator = true;
        } else {
            collapsed.push(ch);
            previous_was_separator = false;
        }
    }

    collapsed
}

#[cfg(not(windows))]
fn deploy_root_key_for_platform(path: &Path) -> String {
    let mut key = path.to_string_lossy().into_owned();
    while key.ends_with('/') && key.len() > 1 {
        key.pop();
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn windows_key_folds_case_slashes_and_trailing_separators() {
        let key = deploy_root_key(Path::new("G:/Dx/.dx/receipts/check///"));

        assert_eq!(key, r"g:\dx\.dx\receipts\check");
    }

    #[cfg(windows)]
    #[test]
    fn windows_key_collapses_repeated_separators() {
        let key = deploy_root_key(Path::new("G://Dx//.dx//receipts//deploy///"));

        assert_eq!(key, r"g:\dx\.dx\receipts\deploy");
    }

    #[cfg(windows)]
    #[test]
    fn windows_key_preserves_unc_prefix_while_collapsing_later_separators() {
        let key = deploy_root_key(Path::new("//Server//Share//.dx//receipts//deploy//"));

        assert_eq!(key, r"\\server\share\.dx\receipts\deploy");
    }

    #[cfg(windows)]
    #[test]
    fn windows_key_preserves_drive_root_separator() {
        let key = deploy_root_key(Path::new("G:/"));

        assert_eq!(key, r"g:\");
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_key_trims_trailing_slashes_without_case_folding() {
        let key = deploy_root_key(Path::new("/Users/DX/.dx/receipts/check///"));

        assert_eq!(key, "/Users/DX/.dx/receipts/check");
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_key_preserves_root_separator() {
        let key = deploy_root_key(Path::new("/"));

        assert_eq!(key, "/");
    }
}
