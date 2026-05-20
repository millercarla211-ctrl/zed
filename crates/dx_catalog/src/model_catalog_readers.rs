mod formats;
mod helpers;

use crate::{
    CatalogGeneratorInput, CatalogSourceKind, ExternalModelInput, ExternalProviderInput, Result,
    SourceMetadata, lite_llm_catalog_input, models_dev_input, openrouter_input,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const DEFAULT_SOURCE_ID: &str = "dx-model-catalog-source";
const DEFAULT_MAX_MODELS: usize = 50_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelCatalogReaderOptions {
    pub source_id: String,
    pub source_revision: Option<String>,
    pub generated_unix_ms: Option<u64>,
    pub max_models: usize,
}

impl ModelCatalogReaderOptions {
    pub fn new() -> Self {
        Self {
            source_id: DEFAULT_SOURCE_ID.to_string(),
            source_revision: None,
            generated_unix_ms: None,
            max_models: DEFAULT_MAX_MODELS,
        }
    }

    pub fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = source_id.into();
        self
    }

    pub fn with_source_revision(mut self, source_revision: impl Into<String>) -> Self {
        self.source_revision = Some(source_revision.into());
        self
    }

    pub fn with_generated_unix_ms(mut self, generated_unix_ms: u64) -> Self {
        self.generated_unix_ms = Some(generated_unix_ms);
        self
    }

    pub fn with_max_models(mut self, max_models: usize) -> Self {
        self.max_models = max_models;
        self
    }
}

impl Default for ModelCatalogReaderOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ModelCatalogReadOutput {
    pub input: CatalogGeneratorInput,
    pub report: ModelCatalogReadReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelCatalogReadReport {
    pub path: Option<PathBuf>,
    pub source_kind: CatalogSourceKind,
    pub source_id: String,
    pub source_available: bool,
    pub provider_count: u32,
    pub model_count: u32,
    pub skipped_entries: Vec<SkippedModelCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SkippedModelCatalogEntry {
    pub location: String,
    pub reason: String,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedModelCatalog {
    pub providers: BTreeMap<String, ExternalProviderInput>,
    pub models: Vec<ExternalModelInput>,
    pub skipped_entries: Vec<SkippedModelCatalogEntry>,
}

pub fn read_model_catalog_file(
    path: impl AsRef<Path>,
    source_kind: CatalogSourceKind,
    options: ModelCatalogReaderOptions,
) -> Result<ModelCatalogReadOutput> {
    let path = path.as_ref().to_path_buf();
    let source_available = path.exists();
    if !source_available || !supports_source_kind(source_kind) {
        return Ok(model_catalog_output(
            Some(path),
            source_kind,
            options,
            source_available,
            ParsedModelCatalog::default(),
        ));
    }

    let contents = fs::read_to_string(&path)?;
    read_model_catalog_json_with_path(&contents, source_kind, options, Some(path), true)
}

pub fn read_model_catalog_json(
    contents: &str,
    source_kind: CatalogSourceKind,
    options: ModelCatalogReaderOptions,
) -> Result<ModelCatalogReadOutput> {
    read_model_catalog_json_with_path(contents, source_kind, options, None, true)
}

fn read_model_catalog_json_with_path(
    contents: &str,
    source_kind: CatalogSourceKind,
    options: ModelCatalogReaderOptions,
    path: Option<PathBuf>,
    source_available: bool,
) -> Result<ModelCatalogReadOutput> {
    if !supports_source_kind(source_kind) {
        let mut parsed = ParsedModelCatalog::default();
        parsed
            .skipped_entries
            .push(skip("source", "unsupported model catalog source kind"));
        return Ok(model_catalog_output(
            path,
            source_kind,
            options,
            source_available,
            parsed,
        ));
    }

    let value: Value = serde_json::from_str(contents)?;
    let parsed = formats::parse_model_catalog_value(&value, source_kind, options.max_models);
    Ok(model_catalog_output(
        path,
        source_kind,
        options,
        source_available,
        parsed,
    ))
}

fn supports_source_kind(source_kind: CatalogSourceKind) -> bool {
    matches!(
        source_kind,
        CatalogSourceKind::ModelsDev
            | CatalogSourceKind::OpenRouter
            | CatalogSourceKind::LiteLlmAliases
    )
}

fn model_catalog_output(
    path: Option<PathBuf>,
    source_kind: CatalogSourceKind,
    options: ModelCatalogReaderOptions,
    source_available: bool,
    parsed: ParsedModelCatalog,
) -> ModelCatalogReadOutput {
    let provider_count = parsed.providers.len() as u32;
    let model_count = parsed.models.len() as u32;
    let metadata = source_metadata(&options, source_kind, path.as_deref(), model_count);
    let provider_values = parsed.providers.into_values().collect::<Vec<_>>();

    let input = match source_kind {
        CatalogSourceKind::ModelsDev => models_dev_input(metadata, provider_values, parsed.models),
        CatalogSourceKind::OpenRouter => openrouter_input(metadata, provider_values, parsed.models),
        CatalogSourceKind::LiteLlmAliases => {
            lite_llm_catalog_input(metadata, provider_values, parsed.models)
        }
        _ => CatalogGeneratorInput::new(metadata.into_record(source_kind)),
    };

    ModelCatalogReadOutput {
        input,
        report: ModelCatalogReadReport {
            path,
            source_kind,
            source_id: options.source_id,
            source_available,
            provider_count,
            model_count,
            skipped_entries: parsed.skipped_entries,
        },
    }
}

fn source_metadata(
    options: &ModelCatalogReaderOptions,
    source_kind: CatalogSourceKind,
    path: Option<&Path>,
    model_count: u32,
) -> SourceMetadata {
    let path_note = path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<inline-json>".to_string());
    let mut metadata = SourceMetadata::new(options.source_id.clone()).with_notes(format!(
        "Model catalog JSON scan; kind={source_kind:?}; path={path_note}; models={model_count}",
    ));

    if let Some(source_revision) = &options.source_revision {
        metadata = metadata.with_revision(source_revision.clone());
    }
    if let Some(generated_unix_ms) = options.generated_unix_ms {
        metadata = metadata.with_generated_unix_ms(generated_unix_ms);
    }

    metadata
}

pub(crate) fn skip(
    location: impl Into<String>,
    reason: impl Into<String>,
) -> SkippedModelCatalogEntry {
    SkippedModelCatalogEntry {
        location: location.into(),
        reason: reason.into(),
    }
}
