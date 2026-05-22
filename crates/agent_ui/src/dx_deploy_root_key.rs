use std::path::Path;

pub(crate) fn deploy_root_key(path: &Path) -> String {
    deploy_root_key_for_platform(path)
}

#[cfg(windows)]
fn deploy_root_key_for_platform(path: &Path) -> String {
    let mut key = path.to_string_lossy().replace('/', "\\");
    while key.ends_with('\\') && key.len() > 3 {
        key.pop();
    }
    key.to_ascii_lowercase()
}

#[cfg(not(windows))]
fn deploy_root_key_for_platform(path: &Path) -> String {
    let mut key = path.to_string_lossy().into_owned();
    while key.ends_with('/') && key.len() > 1 {
        key.pop();
    }
    key
}
