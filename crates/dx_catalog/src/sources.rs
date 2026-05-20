use crate::CatalogSourceKind;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
};

const DX_TOOLS_ROOT_ENV: &str = "DX_TOOLS_ROOT";
const DX_FLOW_ROOT_ENV: &str = "DX_FLOW_ROOT";
const DX_CATALOG_ROOTS_ENV: &str = "DX_CATALOG_SOURCE_ROOTS";
const DX_CATALOG_MODEL_ROOTS_ENV: &str = "DX_CATALOG_MODEL_ROOTS";
const DX_CATALOG_AUTH_ROOTS_ENV: &str = "DX_CATALOG_AUTH_ROOTS";
const ZEROCLAW_HOME_ENV: &str = "ZEROCLAW_HOME";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSourcePurpose {
    FlowLocalRoles,
    ProviderCatalog,
    LocalModels,
    AuthProfiles,
    MetasearchTool,
    ForgeTool,
    SerializerTool,
    RlmTool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogSourceDiscoveryConfig {
    pub candidate_roots: Vec<PathBuf>,
    pub model_roots: Vec<PathBuf>,
    pub auth_roots: Vec<PathBuf>,
    pub extra_candidates: Vec<CatalogSourceCandidate>,
}

impl CatalogSourceDiscoveryConfig {
    pub fn new() -> Self {
        Self {
            candidate_roots: Vec::new(),
            model_roots: Vec::new(),
            auth_roots: Vec::new(),
            extra_candidates: Vec::new(),
        }
    }

    pub fn from_environment() -> Self {
        let mut config = Self::new().with_common_windows_roots();

        push_env_paths(&mut config.candidate_roots, DX_TOOLS_ROOT_ENV);
        push_env_paths(&mut config.candidate_roots, DX_FLOW_ROOT_ENV);
        push_env_paths(&mut config.candidate_roots, DX_CATALOG_ROOTS_ENV);
        push_env_paths(&mut config.model_roots, DX_CATALOG_MODEL_ROOTS_ENV);
        push_env_paths(&mut config.auth_roots, DX_CATALOG_AUTH_ROOTS_ENV);
        push_env_paths(&mut config.auth_roots, ZEROCLAW_HOME_ENV);

        if let Some(home) = env::var_os("USERPROFILE").map(PathBuf::from) {
            push_unique_path(&mut config.auth_roots, home.join(".zeroclaw"));
            push_unique_path(
                &mut config.auth_roots,
                home.join(".config").join("zeroclaw"),
            );
        }

        config
    }

    pub fn with_common_windows_roots(mut self) -> Self {
        if cfg!(windows) {
            for root in [
                r"G:\Workspaces\flow",
                r"G:\Flow",
                r"G:\Dx",
                r"F:\Zed",
                r"G:\Zed",
            ] {
                push_unique_path(&mut self.candidate_roots, PathBuf::from(root));
            }
        }
        self
    }

    pub fn with_candidate_root(mut self, root: impl Into<PathBuf>) -> Self {
        push_unique_path(&mut self.candidate_roots, root.into());
        self
    }

    pub fn with_model_root(mut self, root: impl Into<PathBuf>) -> Self {
        push_unique_path(&mut self.model_roots, root.into());
        self
    }

    pub fn with_auth_root(mut self, root: impl Into<PathBuf>) -> Self {
        push_unique_path(&mut self.auth_roots, root.into());
        self
    }

    pub fn with_extra_candidate(mut self, candidate: CatalogSourceCandidate) -> Self {
        self.extra_candidates.push(candidate);
        self
    }

    pub fn candidates(&self) -> Vec<CatalogSourceCandidate> {
        let mut candidates = Vec::new();

        for root in &self.candidate_roots {
            extend_root_candidates(&mut candidates, root);
        }

        for root in &self.model_roots {
            candidates.push(
                CatalogSourceCandidate::new(
                    "configured-llama-cpp-models",
                    CatalogSourceKind::LlamaCppScan,
                    CatalogSourcePurpose::LocalModels,
                    root.clone(),
                )
                .with_optional_marker(".gitkeep"),
            );
        }

        for root in &self.auth_roots {
            candidates.push(
                CatalogSourceCandidate::new(
                    "configured-auth-profiles",
                    CatalogSourceKind::UserAuthProfiles,
                    CatalogSourcePurpose::AuthProfiles,
                    root.clone(),
                )
                .with_optional_marker("config.toml"),
            );
        }

        candidates.extend(self.extra_candidates.clone());
        dedupe_candidates(candidates)
    }

    pub fn discover(&self) -> CatalogSourceDiscoveryReport {
        let candidates = self
            .candidates()
            .into_iter()
            .map(CatalogSourceCandidate::resolve)
            .collect::<Vec<_>>();

        let available_count = candidates
            .iter()
            .filter(|candidate| candidate.available)
            .count() as u32;

        CatalogSourceDiscoveryReport {
            candidate_count: candidates.len() as u32,
            available_count,
            candidates,
        }
    }
}

impl Default for CatalogSourceDiscoveryConfig {
    fn default() -> Self {
        Self::from_environment()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogSourceCandidate {
    pub id: String,
    pub kind: CatalogSourceKind,
    pub purpose: CatalogSourcePurpose,
    pub root: PathBuf,
    pub required_markers: Vec<PathBuf>,
    pub optional_markers: Vec<PathBuf>,
}

impl CatalogSourceCandidate {
    pub fn new(
        id: impl Into<String>,
        kind: CatalogSourceKind,
        purpose: CatalogSourcePurpose,
        root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            purpose,
            root: root.into(),
            required_markers: Vec::new(),
            optional_markers: Vec::new(),
        }
    }

    pub fn with_required_marker(mut self, marker: impl Into<PathBuf>) -> Self {
        self.required_markers.push(marker.into());
        self
    }

    pub fn with_optional_marker(mut self, marker: impl Into<PathBuf>) -> Self {
        self.optional_markers.push(marker.into());
        self
    }

    pub fn resolve(self) -> CatalogSourceCandidateStatus {
        let root_exists = self.root.exists();
        let matched_required_markers = existing_markers(&self.root, &self.required_markers);
        let missing_required_markers = missing_markers(&self.root, &self.required_markers);
        let matched_optional_markers = existing_markers(&self.root, &self.optional_markers);
        let available = root_exists && missing_required_markers.is_empty();

        CatalogSourceCandidateStatus {
            id: self.id,
            kind: self.kind,
            purpose: self.purpose,
            root: self.root,
            available,
            root_exists,
            matched_required_markers,
            missing_required_markers,
            matched_optional_markers,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogSourceCandidateStatus {
    pub id: String,
    pub kind: CatalogSourceKind,
    pub purpose: CatalogSourcePurpose,
    pub root: PathBuf,
    pub available: bool,
    pub root_exists: bool,
    pub matched_required_markers: Vec<PathBuf>,
    pub missing_required_markers: Vec<PathBuf>,
    pub matched_optional_markers: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogSourceDiscoveryReport {
    pub candidate_count: u32,
    pub available_count: u32,
    pub candidates: Vec<CatalogSourceCandidateStatus>,
}

impl CatalogSourceDiscoveryReport {
    pub fn available_sources(&self) -> impl Iterator<Item = &CatalogSourceCandidateStatus> {
        self.candidates.iter().filter(|source| source.available)
    }

    pub fn missing_sources(&self) -> impl Iterator<Item = &CatalogSourceCandidateStatus> {
        self.candidates.iter().filter(|source| !source.available)
    }

    pub fn first_available(
        &self,
        kind: CatalogSourceKind,
    ) -> Option<&CatalogSourceCandidateStatus> {
        self.available_sources().find(|source| source.kind == kind)
    }

    pub fn available_for_purpose(
        &self,
        purpose: CatalogSourcePurpose,
    ) -> impl Iterator<Item = &CatalogSourceCandidateStatus> {
        self.available_sources()
            .filter(move |source| source.purpose == purpose)
    }
}

fn extend_root_candidates(candidates: &mut Vec<CatalogSourceCandidate>, root: &Path) {
    candidates.push(
        CatalogSourceCandidate::new(
            "flow-local-roles",
            CatalogSourceKind::FlowLocalRoles,
            CatalogSourcePurpose::FlowLocalRoles,
            root.to_path_buf(),
        )
        .with_required_marker("Cargo.toml")
        .with_required_marker("providers")
        .with_optional_marker("models"),
    );

    let providers_root = root.join("providers");
    candidates.push(
        CatalogSourceCandidate::new(
            "zeroclaw-providers",
            CatalogSourceKind::ZeroclawProviders,
            CatalogSourcePurpose::ProviderCatalog,
            providers_root.clone(),
        )
        .with_required_marker("src")
        .with_optional_marker("integrations"),
    );
    candidates.push(
        CatalogSourceCandidate::new(
            "models-dev",
            CatalogSourceKind::ModelsDev,
            CatalogSourcePurpose::ProviderCatalog,
            providers_root.clone(),
        )
        .with_required_marker("src"),
    );
    candidates.push(
        CatalogSourceCandidate::new(
            "openrouter",
            CatalogSourceKind::OpenRouter,
            CatalogSourcePurpose::ProviderCatalog,
            providers_root.clone(),
        )
        .with_required_marker("src"),
    );
    candidates.push(
        CatalogSourceCandidate::new(
            "lite-llm-aliases",
            CatalogSourceKind::LiteLlmAliases,
            CatalogSourcePurpose::ProviderCatalog,
            providers_root,
        )
        .with_required_marker("src"),
    );

    candidates.push(
        CatalogSourceCandidate::new(
            "local-llama-cpp-models",
            CatalogSourceKind::LlamaCppScan,
            CatalogSourcePurpose::LocalModels,
            root.join("models").join("llm"),
        )
        .with_optional_marker(".gitkeep"),
    );

    candidates.push(
        CatalogSourceCandidate::new(
            "metasearch-tool",
            CatalogSourceKind::Manual,
            CatalogSourcePurpose::MetasearchTool,
            root.join("metasearch"),
        )
        .with_required_marker("Cargo.toml")
        .with_optional_marker("crates"),
    );
    candidates.push(
        CatalogSourceCandidate::new(
            "forge-tool",
            CatalogSourceKind::Manual,
            CatalogSourcePurpose::ForgeTool,
            root.join("forge"),
        )
        .with_required_marker("Cargo.toml"),
    );
    candidates.push(
        CatalogSourceCandidate::new(
            "serializer-tool",
            CatalogSourceKind::Manual,
            CatalogSourcePurpose::SerializerTool,
            root.join("serializer"),
        )
        .with_required_marker("Cargo.toml"),
    );
    candidates.push(
        CatalogSourceCandidate::new(
            "rlm-tool",
            CatalogSourceKind::Manual,
            CatalogSourcePurpose::RlmTool,
            root.join("rlm"),
        )
        .with_required_marker("Cargo.toml"),
    );
}

fn existing_markers(root: &Path, markers: &[PathBuf]) -> Vec<PathBuf> {
    markers
        .iter()
        .filter(|marker| root.join(marker).exists())
        .cloned()
        .collect()
}

fn missing_markers(root: &Path, markers: &[PathBuf]) -> Vec<PathBuf> {
    markers
        .iter()
        .filter(|marker| !root.join(marker).exists())
        .cloned()
        .collect()
}

fn push_env_paths(paths: &mut Vec<PathBuf>, key: &str) {
    if let Some(value) = env::var_os(key) {
        for path in env::split_paths(&value) {
            push_unique_path(paths, path);
        }
    }
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.as_os_str().is_empty() {
        return;
    }

    let key = path_key(&path);
    if !paths.iter().any(|existing| path_key(existing) == key) {
        paths.push(path);
    }
}

fn dedupe_candidates(candidates: Vec<CatalogSourceCandidate>) -> Vec<CatalogSourceCandidate> {
    let mut deduped = Vec::new();
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if seen.insert(candidate_key(&candidate)) {
            deduped.push(candidate);
        }
    }
    deduped
}

fn candidate_key(candidate: &CatalogSourceCandidate) -> String {
    format!(
        "{}::{:?}::{:?}::{}",
        candidate.id,
        candidate.kind,
        candidate.purpose,
        path_key(&candidate.root)
    )
}

fn path_key(path: &Path) -> String {
    if cfg!(windows) {
        path.to_string_lossy().to_ascii_lowercase()
    } else {
        path.to_string_lossy().into_owned()
    }
}
