//! Shared application state.

use std::{fs, path::Path, sync::Arc};

use metasearch_core::config::Settings;
use metasearch_engine::EngineRegistry;
use reqwest;

use crate::cache::SearchCache;
use crate::health::EngineHealthTracker;
use crate::orchestrator::SearchOrchestrator;
use crate::templates::TemplateEngine;

/// Shared state available to all request handlers.
pub struct AppState {
    pub settings: Settings,
    pub template_dir: String,
    pub static_dir: String,
    pub engine_registry: Arc<EngineRegistry>,
    pub cache: SearchCache,
    pub templates: TemplateEngine,
    /// High-level search coordinator (cache + health + fan-out).
    pub orchestrator: Arc<SearchOrchestrator>,
    /// Per-engine health tracker shared with the orchestrator.
    pub health: Arc<EngineHealthTracker>,
    /// Shared HTTP client for outbound requests (e.g. autocomplete).
    pub http_client: reqwest::Client,
}

const REQUIRED_TEMPLATE_FILES: &[&str] = &[
    "index.html",
    "results.html",
    "about.html",
    "preferences.html",
    "status.html",
];

const REQUIRED_STATIC_FILES: &[&str] = &[
    "css/style.css",
    "css/shadcn-vercel.css",
    "js/lucide-local.js",
];

impl AppState {
    pub fn runtime_warnings(&self) -> Vec<String> {
        let mut warnings = self.settings.runtime_warnings();
        warnings.extend(self.asset_warnings());
        warnings.sort();
        warnings.dedup();
        warnings
    }

    pub fn asset_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        warnings.extend(collect_asset_warnings(
            &self.template_dir,
            "template",
            REQUIRED_TEMPLATE_FILES,
        ));
        warnings.extend(collect_asset_warnings(
            &self.static_dir,
            "static",
            REQUIRED_STATIC_FILES,
        ));
        warnings.sort();
        warnings.dedup();
        warnings
    }

    pub fn validate_assets(&self) -> anyhow::Result<()> {
        validate_runtime_dir(&self.template_dir, "template")?;
        validate_runtime_dir(&self.static_dir, "static")?;
        validate_required_files(&self.template_dir, "template", REQUIRED_TEMPLATE_FILES)?;
        validate_required_files(&self.static_dir, "static", REQUIRED_STATIC_FILES)?;
        Ok(())
    }
}

fn collect_asset_warnings(root: &str, label: &str, relative_paths: &[&str]) -> Vec<String> {
    let mut warnings = Vec::new();
    match fs::metadata(root) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                warnings.push(format!("{} directory `{}` is not a directory.", label, root));
                return warnings;
            }
        }
        Err(error) => {
            warnings.push(format!(
                "{} directory `{}` is not accessible: {}",
                label, root, error
            ));
            return warnings;
        }
    }

    for relative_path in relative_paths {
        let full_path = Path::new(root).join(relative_path);
        match fs::metadata(&full_path) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    warnings.push(format!(
                        "{} asset `{}` under `{}` is not a file.",
                        label, relative_path, root
                    ));
                }
            }
            Err(error) => warnings.push(format!(
                "{} asset `{}` is not accessible under `{}`: {}",
                label, relative_path, root, error
            )),
        }
    }

    warnings
}

fn validate_runtime_dir(path: &str, label: &str) -> anyhow::Result<()> {
    let metadata = fs::metadata(path)
        .map_err(|error| anyhow::anyhow!("{} directory `{}` is not accessible: {}", label, path, error))?;
    if !metadata.is_dir() {
        anyhow::bail!("{} directory `{}` is not a directory", label, path);
    }
    Ok(())
}

fn validate_required_files(root: &str, label: &str, relative_paths: &[&str]) -> anyhow::Result<()> {
    for relative_path in relative_paths {
        let full_path = Path::new(root).join(relative_path);
        let metadata = fs::metadata(&full_path).map_err(|error| {
            anyhow::anyhow!(
                "{} asset `{}` is not accessible under `{}`: {}",
                label,
                relative_path,
                root,
                error
            )
        })?;
        if !metadata.is_file() {
            anyhow::bail!(
                "{} asset `{}` under `{}` is not a file",
                label,
                relative_path,
                root
            );
        }
    }
    Ok(())
}
