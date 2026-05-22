use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::dx_deploy_local_files::{display_name, read_json_limited, relative_label};

#[derive(Clone)]
pub(crate) struct DxDeployTarget {
    pub label: String,
    pub platform: &'static str,
    pub detail: String,
    pub path: String,
}

pub(crate) fn scan_deploy_targets_for_roots(workspace_roots: &[PathBuf]) -> Vec<DxDeployTarget> {
    let mut targets = Vec::new();

    for root in workspace_roots {
        push_vercel_project_target(root, &mut targets);
        push_file_target(
            root,
            &["vercel.json"],
            "Vercel",
            "Vercel config",
            "Project-level Vercel configuration",
            &mut targets,
        );
        push_file_target(
            root,
            &["netlify.toml"],
            "Netlify",
            "Netlify site",
            "Netlify build and deploy configuration",
            &mut targets,
        );
        push_file_target(
            root,
            &["wrangler.toml"],
            "Cloudflare",
            "Cloudflare Worker",
            "Wrangler deploy configuration",
            &mut targets,
        );
        push_file_target(
            root,
            &["fly.toml"],
            "Fly.io",
            "Fly app",
            "Fly deploy configuration",
            &mut targets,
        );
        push_file_target(
            root,
            &["Dockerfile"],
            "Docker",
            "Container image",
            "Dockerfile build target",
            &mut targets,
        );
    }

    targets.truncate(6);
    targets
}

fn push_vercel_project_target(root: &Path, targets: &mut Vec<DxDeployTarget>) {
    let path = root.join(".vercel").join("project.json");
    if !path.is_file() {
        return;
    }

    let detail = read_json_limited(&path)
        .and_then(|value| {
            let project_id = value.get("projectId").and_then(Value::as_str)?;
            let org_id = value
                .get("orgId")
                .and_then(Value::as_str)
                .unwrap_or("unknown org");
            Some(format!("{project_id} - {org_id}"))
        })
        .unwrap_or_else(|| "Linked Vercel project".to_string());

    targets.push(DxDeployTarget {
        label: format!("Vercel: {}", display_name(root)),
        platform: "Vercel",
        detail,
        path: relative_label(root, &path),
    });
}

fn push_file_target(
    root: &Path,
    relative_path: &[&str],
    platform: &'static str,
    label: &'static str,
    detail: &'static str,
    targets: &mut Vec<DxDeployTarget>,
) {
    let path = relative_path
        .iter()
        .fold(root.to_path_buf(), |path, segment| path.join(segment));
    if !path.is_file() {
        return;
    }

    targets.push(DxDeployTarget {
        label: format!("{label}: {}", display_name(root)),
        platform,
        detail: detail.to_string(),
        path: relative_label(root, &path),
    });
}
