use crate::{
    CatalogArtifactHeader, CatalogBuildReport, CatalogConflictPolicy, CatalogGeneratorInput,
    CatalogGeneratorOptions, CatalogSourceCandidateStatus, CatalogSourceDiscoveryConfig,
    CatalogSourceDiscoveryReport, CatalogSourceKind, CatalogSourcePurpose, CatalogSourceRecord,
    DxCatalog, DxCatalogError, LocalModelCatalogReadReport, LocalModelSourceReaderOptions,
    MappedCatalogArtifact, ModelCatalogReadReport, ModelCatalogReaderOptions,
    ProviderSourceReadReport, Result, build_catalog_provider_registration_specs,
    build_catalog_with_last_good, read_catalog_artifact, read_local_model_source,
    read_model_catalog_file, read_provider_source, write_catalog_artifact,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const DEFAULT_MODEL_CATALOG_MAX_MODELS: usize = 50_000;
const DEFAULT_LOCAL_MODEL_MAX_DEPTH: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogSourceReadOptions {
    pub source_revision: Option<String>,
    pub generated_unix_ms: Option<u64>,
    pub local_model_max_depth: u8,
    pub model_catalog_max_models: usize,
}

impl CatalogSourceReadOptions {
    pub fn new() -> Self {
        Self {
            source_revision: None,
            generated_unix_ms: None,
            local_model_max_depth: DEFAULT_LOCAL_MODEL_MAX_DEPTH,
            model_catalog_max_models: DEFAULT_MODEL_CATALOG_MAX_MODELS,
        }
    }

    pub fn with_source_revision(mut self, source_revision: impl Into<String>) -> Self {
        self.source_revision = Some(source_revision.into());
        self
    }

    pub fn with_generated_unix_ms(mut self, generated_unix_ms: u64) -> Self {
        self.generated_unix_ms = Some(generated_unix_ms);
        self
    }

    pub fn with_local_model_max_depth(mut self, max_depth: u8) -> Self {
        self.local_model_max_depth = max_depth;
        self
    }

    pub fn with_model_catalog_max_models(mut self, max_models: usize) -> Self {
        self.model_catalog_max_models = max_models.max(1);
        self
    }
}

impl Default for CatalogSourceReadOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogArtifactBuildOptions {
    pub artifact_path: PathBuf,
    pub last_good_artifact_path: Option<PathBuf>,
    pub source_revision: String,
    pub generated_unix_ms: u64,
    pub conflict_policy: CatalogConflictPolicy,
    pub use_last_good_on_invalid: bool,
    pub discovery_config: CatalogSourceDiscoveryConfig,
    pub source_read_options: CatalogSourceReadOptions,
}

impl CatalogArtifactBuildOptions {
    pub fn new(
        artifact_path: impl Into<PathBuf>,
        source_revision: impl Into<String>,
        generated_unix_ms: u64,
    ) -> Self {
        let source_revision = source_revision.into();
        Self {
            artifact_path: artifact_path.into(),
            last_good_artifact_path: None,
            source_revision: source_revision.clone(),
            generated_unix_ms,
            conflict_policy: CatalogConflictPolicy::PreferLatestSource,
            use_last_good_on_invalid: true,
            discovery_config: CatalogSourceDiscoveryConfig::from_environment(),
            source_read_options: CatalogSourceReadOptions::new()
                .with_source_revision(source_revision)
                .with_generated_unix_ms(generated_unix_ms),
        }
    }

    pub fn with_last_good_artifact_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.last_good_artifact_path = Some(path.into());
        self
    }

    pub fn with_conflict_policy(mut self, conflict_policy: CatalogConflictPolicy) -> Self {
        self.conflict_policy = conflict_policy;
        self
    }

    pub fn use_last_good_on_invalid(mut self, use_last_good_on_invalid: bool) -> Self {
        self.use_last_good_on_invalid = use_last_good_on_invalid;
        self
    }

    pub fn with_discovery_config(mut self, discovery_config: CatalogSourceDiscoveryConfig) -> Self {
        self.discovery_config = discovery_config;
        self
    }

    pub fn with_source_read_options(
        mut self,
        source_read_options: CatalogSourceReadOptions,
    ) -> Self {
        self.source_read_options = source_read_options;
        self
    }
}

#[derive(Debug, Clone)]
pub struct CatalogArtifactBuildOutput {
    pub catalog: DxCatalog,
    pub report: CatalogArtifactBuildReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogArtifactBuildReport {
    pub artifact_path: PathBuf,
    pub last_good_artifact_path: Option<PathBuf>,
    pub last_good_read_error: Option<String>,
    pub artifact_written: bool,
    pub artifact_header: CatalogArtifactHeader,
    pub discovery: CatalogSourceDiscoveryReport,
    pub source_read: CatalogSourceReadReport,
    pub build: CatalogBuildReport,
    pub registration_spec_count: u32,
    pub ready_registration_spec_count: u32,
}

#[derive(Debug, Clone)]
pub struct CatalogSourceReadOutput {
    pub inputs: Vec<CatalogGeneratorInput>,
    pub report: CatalogSourceReadReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogSourceReadReport {
    pub input_count: u32,
    pub provider_source_count: u32,
    pub model_catalog_source_count: u32,
    pub local_model_source_count: u32,
    pub manual_source_count: u32,
    pub skipped_model_catalog_candidate_count: u32,
    pub source_error_count: u32,
    pub provider_sources: Vec<ProviderSourceReadReport>,
    pub model_catalog_sources: Vec<ModelCatalogReadReport>,
    pub local_model_sources: Vec<LocalModelCatalogReadReport>,
    pub manual_sources: Vec<ManualCatalogSourceReport>,
    pub skipped_model_catalog_candidates: Vec<SkippedCatalogSourceCandidate>,
    pub source_errors: Vec<CatalogSourceReadError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManualCatalogSourceReport {
    pub source_id: String,
    pub source_kind: CatalogSourceKind,
    pub purpose: CatalogSourcePurpose,
    pub root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SkippedCatalogSourceCandidate {
    pub source_id: String,
    pub source_kind: CatalogSourceKind,
    pub purpose: CatalogSourcePurpose,
    pub root: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogSourceReadError {
    pub source_id: String,
    pub source_kind: CatalogSourceKind,
    pub purpose: CatalogSourcePurpose,
    pub root: PathBuf,
    pub message: String,
}

pub fn build_catalog_artifact_from_sources(
    options: CatalogArtifactBuildOptions,
) -> Result<CatalogArtifactBuildOutput> {
    let discovery = options.discovery_config.discover();
    let source_output = read_discovered_catalog_sources(&discovery, &options.source_read_options);
    let (last_good, last_good_read_error) =
        read_last_good_catalog(options.last_good_artifact_path.as_deref());

    let build = build_catalog_with_last_good(
        source_output.inputs,
        CatalogGeneratorOptions {
            source_revision: options.source_revision,
            generated_unix_ms: options.generated_unix_ms,
            conflict_policy: options.conflict_policy,
            use_last_good_on_invalid: options.use_last_good_on_invalid,
        },
        last_good,
    );

    let final_validation = build.catalog.validate_references();
    if !final_validation.is_valid {
        return Err(DxCatalogError::InvalidCatalog {
            reason: format!(
                "providers={}, models={}, duplicate_providers={}, duplicate_models={}, missing_provider_models={}, missing_route_models={}",
                final_validation.provider_count,
                final_validation.model_count,
                final_validation.duplicate_provider_ids.len(),
                final_validation.duplicate_model_ids.len(),
                final_validation.missing_provider_model_ids.len(),
                final_validation.missing_route_model_ids.len()
            ),
        });
    }

    write_catalog_artifact(&options.artifact_path, &build.catalog)?;
    let artifact_header = MappedCatalogArtifact::open(&options.artifact_path)?.header();
    let registration_specs = build_catalog_provider_registration_specs(&build.catalog);

    let report = CatalogArtifactBuildReport {
        artifact_path: options.artifact_path,
        last_good_artifact_path: options.last_good_artifact_path,
        last_good_read_error,
        artifact_written: true,
        artifact_header,
        discovery,
        source_read: source_output.report,
        build: build.report,
        registration_spec_count: registration_specs.len() as u32,
        ready_registration_spec_count: registration_specs
            .iter()
            .filter(|spec| spec.ready_for_execution)
            .count() as u32,
    };

    Ok(CatalogArtifactBuildOutput {
        catalog: build.catalog,
        report,
    })
}

pub fn read_discovered_catalog_sources(
    discovery: &CatalogSourceDiscoveryReport,
    options: &CatalogSourceReadOptions,
) -> CatalogSourceReadOutput {
    let mut inputs = Vec::new();
    let mut provider_sources = Vec::new();
    let mut model_catalog_sources = Vec::new();
    let mut local_model_sources = Vec::new();
    let mut manual_sources = Vec::new();
    let mut skipped_model_catalog_candidates = Vec::new();
    let mut source_errors = Vec::new();

    for source in discovery.available_sources() {
        match source.purpose {
            CatalogSourcePurpose::ProviderCatalog | CatalogSourcePurpose::AuthProfiles => {
                read_provider_candidate(
                    source,
                    options,
                    &mut inputs,
                    &mut provider_sources,
                    &mut source_errors,
                );
                read_model_catalog_candidates(
                    source,
                    options,
                    &mut inputs,
                    &mut model_catalog_sources,
                    &mut skipped_model_catalog_candidates,
                    &mut source_errors,
                );
            }
            CatalogSourcePurpose::LocalModels => {
                read_local_model_candidate(
                    source,
                    options,
                    &mut inputs,
                    &mut local_model_sources,
                    &mut source_errors,
                );
            }
            CatalogSourcePurpose::FlowLocalRoles
            | CatalogSourcePurpose::MetasearchTool
            | CatalogSourcePurpose::ForgeTool
            | CatalogSourcePurpose::SerializerTool
            | CatalogSourcePurpose::RlmTool => {
                inputs.push(manual_source_input(source, options));
                manual_sources.push(ManualCatalogSourceReport {
                    source_id: stable_source_id(source, "manual"),
                    source_kind: source.kind,
                    purpose: source.purpose,
                    root: source.root.clone(),
                });
            }
        }
    }

    CatalogSourceReadOutput {
        report: CatalogSourceReadReport {
            input_count: inputs.len() as u32,
            provider_source_count: provider_sources.len() as u32,
            model_catalog_source_count: model_catalog_sources.len() as u32,
            local_model_source_count: local_model_sources.len() as u32,
            manual_source_count: manual_sources.len() as u32,
            skipped_model_catalog_candidate_count: skipped_model_catalog_candidates.len() as u32,
            source_error_count: source_errors.len() as u32,
            provider_sources,
            model_catalog_sources,
            local_model_sources,
            manual_sources,
            skipped_model_catalog_candidates,
            source_errors,
        },
        inputs,
    }
}

fn read_provider_candidate(
    source: &CatalogSourceCandidateStatus,
    options: &CatalogSourceReadOptions,
    inputs: &mut Vec<CatalogGeneratorInput>,
    provider_sources: &mut Vec<ProviderSourceReadReport>,
    source_errors: &mut Vec<CatalogSourceReadError>,
) {
    let reader_options = provider_reader_options(source, options);
    match read_provider_source(source, reader_options) {
        Ok(output) => {
            inputs.push(output.input);
            provider_sources.push(output.report);
        }
        Err(error) => source_errors.push(source_read_error(source, error)),
    }
}

fn read_model_catalog_candidates(
    source: &CatalogSourceCandidateStatus,
    options: &CatalogSourceReadOptions,
    inputs: &mut Vec<CatalogGeneratorInput>,
    model_catalog_sources: &mut Vec<ModelCatalogReadReport>,
    skipped_model_catalog_candidates: &mut Vec<SkippedCatalogSourceCandidate>,
    source_errors: &mut Vec<CatalogSourceReadError>,
) {
    if !supports_model_catalog_reader(source.kind) {
        return;
    }

    let files = model_catalog_files_for_source(source);
    if files.is_empty() {
        skipped_model_catalog_candidates.push(skipped_model_catalog_source(
            source,
            "No JSON model catalog file was found beside this provider source.",
        ));
        return;
    }

    for file in files {
        let reader_options = model_catalog_reader_options(source, &file, options);
        match read_model_catalog_file(&file, source.kind, reader_options) {
            Ok(output) => {
                inputs.push(output.input);
                model_catalog_sources.push(output.report);
            }
            Err(error) => source_errors.push(CatalogSourceReadError {
                source_id: stable_file_source_id(source, &file, "model-catalog"),
                source_kind: source.kind,
                purpose: source.purpose,
                root: file,
                message: error.to_string(),
            }),
        }
    }
}

fn read_local_model_candidate(
    source: &CatalogSourceCandidateStatus,
    options: &CatalogSourceReadOptions,
    inputs: &mut Vec<CatalogGeneratorInput>,
    local_model_sources: &mut Vec<LocalModelCatalogReadReport>,
    source_errors: &mut Vec<CatalogSourceReadError>,
) {
    let reader_options = local_model_reader_options(source, options);
    match read_local_model_source(source, reader_options) {
        Ok(output) => {
            inputs.push(output.input);
            local_model_sources.push(output.report);
        }
        Err(error) => source_errors.push(source_read_error(source, error)),
    }
}

fn provider_reader_options(
    source: &CatalogSourceCandidateStatus,
    options: &CatalogSourceReadOptions,
) -> crate::ProviderSourceReaderOptions {
    let mut reader_options = crate::ProviderSourceReaderOptions::new()
        .with_source_id(stable_source_id(source, "provider"))
        .with_auth_source_id(stable_source_id(source, "auth"));

    if let Some(source_revision) = &options.source_revision {
        reader_options = reader_options.with_source_revision(source_revision.clone());
    }
    if let Some(generated_unix_ms) = options.generated_unix_ms {
        reader_options = reader_options.with_generated_unix_ms(generated_unix_ms);
    }

    reader_options
}

fn model_catalog_reader_options(
    source: &CatalogSourceCandidateStatus,
    file: &Path,
    options: &CatalogSourceReadOptions,
) -> ModelCatalogReaderOptions {
    let mut reader_options = ModelCatalogReaderOptions::new()
        .with_source_id(stable_file_source_id(source, file, "model-catalog"))
        .with_max_models(options.model_catalog_max_models);

    if let Some(source_revision) = &options.source_revision {
        reader_options = reader_options.with_source_revision(source_revision.clone());
    }
    if let Some(generated_unix_ms) = options.generated_unix_ms {
        reader_options = reader_options.with_generated_unix_ms(generated_unix_ms);
    }

    reader_options
}

fn local_model_reader_options(
    source: &CatalogSourceCandidateStatus,
    options: &CatalogSourceReadOptions,
) -> LocalModelSourceReaderOptions {
    let mut reader_options = LocalModelSourceReaderOptions::new("local-llama-cpp")
        .with_source_id(stable_source_id(source, "local-models"))
        .with_max_depth(options.local_model_max_depth);

    if let Some(source_revision) = &options.source_revision {
        reader_options = reader_options.with_source_revision(source_revision.clone());
    }
    if let Some(generated_unix_ms) = options.generated_unix_ms {
        reader_options = reader_options.with_generated_unix_ms(generated_unix_ms);
    }

    reader_options
}

fn manual_source_input(
    source: &CatalogSourceCandidateStatus,
    options: &CatalogSourceReadOptions,
) -> CatalogGeneratorInput {
    CatalogGeneratorInput::new(CatalogSourceRecord {
        id: stable_source_id(source, "manual"),
        kind: source.kind,
        revision: options.source_revision.clone(),
        generated_unix_ms: options.generated_unix_ms,
        notes: Some(format!(
            "Discovered {:?} source at {}",
            source.purpose,
            source.root.display()
        )),
    })
}

fn read_last_good_catalog(path: Option<&Path>) -> (Option<DxCatalog>, Option<String>) {
    let Some(path) = path else {
        return (None, None);
    };
    if !path.is_file() {
        return (None, None);
    }

    match read_catalog_artifact(path) {
        Ok(catalog) => (Some(catalog), None),
        Err(error) => (None, Some(error.to_string())),
    }
}

fn supports_model_catalog_reader(source_kind: CatalogSourceKind) -> bool {
    matches!(
        source_kind,
        CatalogSourceKind::ModelsDev
            | CatalogSourceKind::OpenRouter
            | CatalogSourceKind::LiteLlmAliases
    )
}

fn model_catalog_files_for_source(source: &CatalogSourceCandidateStatus) -> Vec<PathBuf> {
    if source.root.is_file() && is_json_file(&source.root) {
        return vec![source.root.clone()];
    }

    if !source.root.is_dir() {
        return Vec::new();
    }

    let mut files = Vec::new();
    let names = model_catalog_file_names(source.kind);
    let src_dir = source.root.join("src");
    for dir in [source.root.as_path(), src_dir.as_path()] {
        for name in &names {
            push_existing_file(&mut files, dir.join(name));
        }
    }

    if let Ok(entries) = fs::read_dir(&source.root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_json_file(&path) {
                push_existing_file(&mut files, path);
            }
        }
    }

    files
}

fn model_catalog_file_names(source_kind: CatalogSourceKind) -> Vec<&'static str> {
    let mut names = vec!["models.json", "model_catalog.json", "catalog.json"];
    match source_kind {
        CatalogSourceKind::ModelsDev => {
            names.extend(["models.dev.json", "models-dev.json", "providers.json"]);
        }
        CatalogSourceKind::OpenRouter => {
            names.extend([
                "openrouter.json",
                "open_router.json",
                "openrouter-models.json",
            ]);
        }
        CatalogSourceKind::LiteLlmAliases => {
            names.extend([
                "litellm.json",
                "lite-llm.json",
                "litellm-models.json",
                "model_list.json",
            ]);
        }
        _ => {}
    }
    names
}

fn push_existing_file(files: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() && !files.iter().any(|existing| same_path(existing, &path)) {
        files.push(path);
    }
}

fn is_json_file(path: &Path) -> bool {
    path.extension()
        .map(|extension| extension.to_string_lossy().eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

fn skipped_model_catalog_source(
    source: &CatalogSourceCandidateStatus,
    reason: impl Into<String>,
) -> SkippedCatalogSourceCandidate {
    SkippedCatalogSourceCandidate {
        source_id: stable_source_id(source, "model-catalog"),
        source_kind: source.kind,
        purpose: source.purpose,
        root: source.root.clone(),
        reason: reason.into(),
    }
}

fn source_read_error(
    source: &CatalogSourceCandidateStatus,
    error: impl std::error::Error,
) -> CatalogSourceReadError {
    CatalogSourceReadError {
        source_id: stable_source_id(source, "reader-error"),
        source_kind: source.kind,
        purpose: source.purpose,
        root: source.root.clone(),
        message: error.to_string(),
    }
}

fn stable_source_id(source: &CatalogSourceCandidateStatus, namespace: &str) -> String {
    format!(
        "{}-{}-{:016x}",
        namespace,
        source.id,
        stable_path_hash(&source.root)
    )
}

fn stable_file_source_id(
    source: &CatalogSourceCandidateStatus,
    file: &Path,
    namespace: &str,
) -> String {
    format!(
        "{}-{}-{:016x}",
        namespace,
        source.id,
        stable_path_hash(file)
    )
}

fn stable_path_hash(path: &Path) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in path_key(path).as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn same_path(left: &Path, right: &Path) -> bool {
    path_key(left) == path_key(right)
}

fn path_key(path: &Path) -> String {
    if cfg!(windows) {
        path.to_string_lossy().to_ascii_lowercase()
    } else {
        path.to_string_lossy().into_owned()
    }
}
