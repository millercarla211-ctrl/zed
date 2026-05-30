use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

const PROJECT_RECEIPT_ANCESTOR_LIMIT: usize = 8;

pub(super) fn active_style_receipt_roots(
    source_path: Option<&str>,
    workspace_root: Option<&str>,
) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut roots = Vec::new();

    let Some(source_path) = source_path.map(Path::new).filter(|path| path.is_absolute()) else {
        return roots;
    };
    let Some(workspace_root) = workspace_root
        .map(Path::new)
        .filter(|root| root.is_absolute() && source_path.starts_with(root))
    else {
        return roots;
    };

    for ancestor in source_path
        .parent()
        .into_iter()
        .flat_map(Path::ancestors)
        .take(PROJECT_RECEIPT_ANCESTOR_LIMIT)
    {
        if !ancestor.starts_with(workspace_root) {
            break;
        }
        push_unique_root(
            &mut roots,
            &mut seen,
            ancestor.join(".dx").join("receipts").join("style"),
        );
        if ancestor == workspace_root {
            break;
        }
    }

    roots
}

fn push_unique_root(roots: &mut Vec<PathBuf>, seen: &mut HashSet<String>, root: PathBuf) {
    if seen.insert(receipt_root_key(&root)) {
        roots.push(root);
    }
}

fn receipt_root_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}
