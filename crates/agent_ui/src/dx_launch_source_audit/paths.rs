use super::{DX_STUDIO_QA_LATEST, SOURCE_AUDIT_LATEST, SOURCE_AUDIT_MARKDOWN, SOURCE_AUDIT_ROOT};
use std::path::PathBuf;

pub(super) struct SourceAuditPaths {
    pub root: PathBuf,
    pub latest_path: PathBuf,
    pub markdown_path: PathBuf,
    pub dx_studio_qa_path: PathBuf,
    pub root_exists: bool,
    pub latest_present: bool,
    pub markdown_present: bool,
    pub dx_studio_qa_present: bool,
}

pub(super) fn source_audit_paths() -> SourceAuditPaths {
    let root = PathBuf::from(SOURCE_AUDIT_ROOT);
    let latest_path = root.join(SOURCE_AUDIT_LATEST);
    let markdown_path = root.join(SOURCE_AUDIT_MARKDOWN);
    let dx_studio_qa_path = PathBuf::from(DX_STUDIO_QA_LATEST);

    SourceAuditPaths {
        root_exists: root.is_dir(),
        latest_present: latest_path.is_file(),
        markdown_present: markdown_path.is_file(),
        dx_studio_qa_present: dx_studio_qa_path.is_file(),
        root,
        latest_path,
        markdown_path,
        dx_studio_qa_path,
    }
}
