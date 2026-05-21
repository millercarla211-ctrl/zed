use std::{env, path::PathBuf};

use gpui::{App, SharedString};
use language_model::{LanguageModelRegistry, ZED_CLOUD_PROVIDER_ID};

const DX_AGENT_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\agents";
const DX_PROVIDER_CATALOG_PATH: &str = r"G:\Dx\.dx\catalog\agents\provider-model-catalog.rkyv";
const DX_CATALOG_ARTIFACT_ENV: &str = "DX_CATALOG_ARTIFACT";
const DX_CATALOG_PATH_ENV: &str = "DX_CATALOG_PATH";

#[derive(Clone)]
pub(crate) struct DxProviderOnboardingStatus {
    pub state: &'static str,
    pub summary: String,
    pub next_action: String,
    pub native_provider_count: usize,
    pub visible_provider_count: usize,
    pub native_provider_names: Vec<SharedString>,
    pub receipt_root: PathBuf,
    pub receipt_root_exists: bool,
    pub provider_receipt_present: bool,
    pub model_receipt_present: bool,
    pub contract_receipt_present: bool,
    pub catalog_path: PathBuf,
    pub catalog_present: bool,
    pub catalog_source: String,
}

impl DxProviderOnboardingStatus {
    pub(crate) fn detect(cx: &App) -> Self {
        let registry = LanguageModelRegistry::read_global(cx);
        let visible_providers = registry.visible_providers();
        let native_provider_names = visible_providers
            .iter()
            .filter(|provider| {
                provider.is_authenticated(cx) && provider.id() != ZED_CLOUD_PROVIDER_ID
            })
            .map(|provider| provider.name().0)
            .collect::<Vec<_>>();
        let native_provider_count = native_provider_names.len();
        let visible_provider_count = visible_providers.len();

        let receipt_root = PathBuf::from(DX_AGENT_RECEIPT_ROOT);
        let receipt_root_exists = receipt_root.is_dir();
        let provider_receipt_present = receipt_root.join("providers-list-latest.json").is_file();
        let model_receipt_present = receipt_root.join("models-list-latest.json").is_file();
        let contract_receipt_present = receipt_root.join("contract-latest.json").is_file();

        let (catalog_path, catalog_source) = provider_catalog_path();
        let catalog_present = catalog_path.is_file();

        let state = if native_provider_count > 0 {
            "ready"
        } else if catalog_present || provider_receipt_present || model_receipt_present {
            "needs approval"
        } else {
            "missing"
        };
        let summary = provider_summary(
            state,
            native_provider_count,
            &native_provider_names,
            catalog_present,
            provider_receipt_present || model_receipt_present || contract_receipt_present,
        );
        let next_action = if native_provider_count > 0 {
            "Use the model picker or Provider Settings to choose the launch model; no credential import is required."
        } else if catalog_present || provider_receipt_present || model_receipt_present {
            "Review DX catalog/provider receipts, then open Provider Settings to explicitly approve any native provider setup."
        } else {
            "Open Provider Settings to add a provider, or ask Agent to inspect DX catalog provider settings before registration."
        }
        .to_string();

        Self {
            state,
            summary,
            next_action,
            native_provider_count,
            visible_provider_count,
            native_provider_names,
            receipt_root,
            receipt_root_exists,
            provider_receipt_present,
            model_receipt_present,
            contract_receipt_present,
            catalog_path,
            catalog_present,
            catalog_source,
        }
    }

    pub(crate) fn native_provider_label(&self) -> String {
        if self.native_provider_names.is_empty() {
            return format!("0 / {} native providers ready", self.visible_provider_count);
        }

        let names = self
            .native_provider_names
            .iter()
            .take(3)
            .map(|name| name.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} / {} ready: {}",
            self.native_provider_count, self.visible_provider_count, names
        )
    }

    pub(crate) fn receipt_label(&self) -> String {
        if !self.receipt_root_exists {
            return format!("Missing {}", self.receipt_root.display());
        }

        format!(
            "providers {} / models {} / contract {}",
            yes_no(self.provider_receipt_present),
            yes_no(self.model_receipt_present),
            yes_no(self.contract_receipt_present)
        )
    }

    pub(crate) fn catalog_label(&self) -> String {
        if self.catalog_present {
            return format!("Present via {}", self.catalog_source);
        }

        format!("Missing {}", self.catalog_path.display())
    }
}

fn provider_catalog_path() -> (PathBuf, String) {
    if let Some(path) = env_path(DX_CATALOG_ARTIFACT_ENV) {
        return (path, DX_CATALOG_ARTIFACT_ENV.to_string());
    }

    if let Some(path) = env_path(DX_CATALOG_PATH_ENV) {
        return (path, DX_CATALOG_PATH_ENV.to_string());
    }

    (
        PathBuf::from(DX_PROVIDER_CATALOG_PATH),
        "DX default catalog".to_string(),
    )
}

fn env_path(name: &str) -> Option<PathBuf> {
    let value = env::var_os(name)?;
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty() {
        return None;
    }
    Some(path)
}

fn provider_summary(
    state: &str,
    native_provider_count: usize,
    native_provider_names: &[SharedString],
    catalog_present: bool,
    receipts_present: bool,
) -> String {
    if native_provider_count > 0 {
        let names = native_provider_names
            .iter()
            .take(3)
            .map(|name| name.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return format!("DX can launch with native provider readiness from {names}.");
    }

    if catalog_present || receipts_present {
        return "DX provider catalog evidence is visible, but native setup still needs explicit approval."
            .to_string();
    }

    format!("Provider readiness is {state}; DX will not import credentials automatically.")
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
