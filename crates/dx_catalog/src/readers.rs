use crate::{
    CatalogGeneratorInput, CatalogSourceCandidateStatus, CatalogSourceKind, CatalogSourcePurpose,
    LlamaCppModelInput, Result, RoutingRole, SourceMetadata, llama_cpp_scan_input,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const DEFAULT_LOCAL_PROVIDER_ID: &str = "local-llama-cpp";
const DEFAULT_SOURCE_ID: &str = "local-llama-cpp-model-scan";
const DEFAULT_MAX_DEPTH: u8 = 2;
const MODEL_EXTENSIONS: &[&str] = &["gguf", "ggml"];
const SKIPPED_DIR_NAMES: &[&str] = &[".cache", ".git", "target", "tmp", "trash"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LocalModelSourceReaderOptions {
    pub provider_id: String,
    pub source_id: String,
    pub source_revision: Option<String>,
    pub generated_unix_ms: Option<u64>,
    pub max_depth: u8,
    pub include_hidden_files: bool,
    pub include_projector_models: bool,
}

impl LocalModelSourceReaderOptions {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            source_id: DEFAULT_SOURCE_ID.to_string(),
            source_revision: None,
            generated_unix_ms: None,
            max_depth: DEFAULT_MAX_DEPTH,
            include_hidden_files: false,
            include_projector_models: false,
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

    pub fn with_max_depth(mut self, max_depth: u8) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn include_hidden_files(mut self, include_hidden_files: bool) -> Self {
        self.include_hidden_files = include_hidden_files;
        self
    }

    pub fn include_projector_models(mut self, include_projector_models: bool) -> Self {
        self.include_projector_models = include_projector_models;
        self
    }
}

impl Default for LocalModelSourceReaderOptions {
    fn default() -> Self {
        Self::new(DEFAULT_LOCAL_PROVIDER_ID)
    }
}

#[derive(Debug, Clone)]
pub struct LocalModelCatalogReadOutput {
    pub input: CatalogGeneratorInput,
    pub report: LocalModelCatalogReadReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LocalModelCatalogReadReport {
    pub root: PathBuf,
    pub provider_id: String,
    pub source_id: String,
    pub source_available: bool,
    pub discovered_model_count: u32,
    pub skipped_file_count: u32,
    pub max_depth: u8,
    pub skipped_files: Vec<SkippedLocalModelFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SkippedLocalModelFile {
    pub path: PathBuf,
    pub reason: String,
}

pub fn read_local_model_source(
    source: &CatalogSourceCandidateStatus,
    options: LocalModelSourceReaderOptions,
) -> Result<LocalModelCatalogReadOutput> {
    let source_available = source.available
        && source.kind == CatalogSourceKind::LlamaCppScan
        && source.purpose == CatalogSourcePurpose::LocalModels;

    if !source_available {
        return Ok(local_model_output(
            source.root.clone(),
            options,
            Vec::new(),
            Vec::new(),
            false,
        ));
    }

    read_local_models_from_root(&source.root, options)
}

pub fn read_local_models_from_root(
    root: impl AsRef<Path>,
    options: LocalModelSourceReaderOptions,
) -> Result<LocalModelCatalogReadOutput> {
    let root = root.as_ref().to_path_buf();
    let mut files = Vec::new();
    let mut skipped_files = Vec::new();

    if root.exists() {
        collect_local_model_files(&root, &root, 0, &options, &mut files, &mut skipped_files)?;
    }

    let source_available = root.exists();
    Ok(local_model_output(
        root,
        options,
        files,
        skipped_files,
        source_available,
    ))
}

fn local_model_output(
    root: PathBuf,
    options: LocalModelSourceReaderOptions,
    files: Vec<LocalModelFile>,
    skipped_files: Vec<SkippedLocalModelFile>,
    source_available: bool,
) -> LocalModelCatalogReadOutput {
    let source_id = options.source_id.clone();
    let provider_id = options.provider_id.clone();
    let max_depth = options.max_depth;
    let metadata = source_metadata(&options, &root, files.len());
    let models = files
        .into_iter()
        .map(|file| local_model_input(&root, &provider_id, file))
        .collect::<Vec<_>>();
    let discovered_model_count = models.len() as u32;
    let skipped_file_count = skipped_files.len() as u32;

    LocalModelCatalogReadOutput {
        input: llama_cpp_scan_input(metadata, models),
        report: LocalModelCatalogReadReport {
            root,
            provider_id,
            source_id,
            source_available,
            discovered_model_count,
            skipped_file_count,
            max_depth,
            skipped_files,
        },
    }
}

fn source_metadata(
    options: &LocalModelSourceReaderOptions,
    root: &Path,
    model_count: usize,
) -> SourceMetadata {
    let mut metadata = SourceMetadata::new(options.source_id.clone()).with_notes(format!(
        "Local llama.cpp model scan; root={}; models={model_count}; max_depth={}",
        root.display(),
        options.max_depth
    ));

    if let Some(source_revision) = &options.source_revision {
        metadata = metadata.with_revision(source_revision.clone());
    }
    if let Some(generated_unix_ms) = options.generated_unix_ms {
        metadata = metadata.with_generated_unix_ms(generated_unix_ms);
    }

    metadata
}

fn collect_local_model_files(
    root: &Path,
    current: &Path,
    depth: u8,
    options: &LocalModelSourceReaderOptions,
    files: &mut Vec<LocalModelFile>,
    skipped_files: &mut Vec<SkippedLocalModelFile>,
) -> Result<()> {
    if depth > options.max_depth {
        return Ok(());
    }

    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if entry.file_type()?.is_dir() {
            if should_skip_dir(&file_name) {
                continue;
            }
            collect_local_model_files(root, &path, depth + 1, options, files, skipped_files)?;
            continue;
        }

        if !entry.file_type()?.is_file() {
            continue;
        }

        if !options.include_hidden_files && file_name.starts_with('.') {
            continue;
        }

        if !is_local_model_file(&path) {
            continue;
        }

        if !options.include_projector_models && is_projector_model(&file_name) {
            skipped_files.push(SkippedLocalModelFile {
                path,
                reason: "projector model skipped by default".to_string(),
            });
            continue;
        }

        let size_bytes = entry.metadata()?.len();
        let relative_path = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        files.push(LocalModelFile {
            path,
            relative_path,
            size_bytes,
        });
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalModelFile {
    path: PathBuf,
    relative_path: PathBuf,
    size_bytes: u64,
}

fn local_model_input(root: &Path, provider_id: &str, file: LocalModelFile) -> LlamaCppModelInput {
    let display_name = file
        .path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Local llama.cpp model".to_string());
    let mut input = LlamaCppModelInput::new(
        provider_id.to_string(),
        stable_model_id(provider_id, &file.relative_path),
        display_name.clone(),
    );
    input.path_hint = Some(file.path.to_string_lossy().into_owned());
    input.quantization = infer_quantization(&display_name);
    input.parameter_count_hint = infer_parameter_count(&display_name);
    input.recommended_roles = infer_roles(&display_name);
    input.aliases = model_aliases(&display_name);
    input.notes = Some(format!(
        "Discovered from {}; relative_path={}; size_bytes={}",
        root.display(),
        file.relative_path.display(),
        file.size_bytes
    ));
    input
}

fn stable_model_id(provider_id: &str, relative_path: &Path) -> String {
    format!(
        "{provider_id}.{}",
        slug(relative_path_without_extension(relative_path))
    )
}

fn relative_path_without_extension(relative_path: &Path) -> String {
    let mut path = relative_path.to_path_buf();
    path.set_extension("");
    path.to_string_lossy().into_owned()
}

fn model_aliases(display_name: &str) -> Vec<String> {
    let mut aliases = BTreeSet::new();
    aliases.insert(display_name.to_string());
    aliases.insert(display_name.to_ascii_lowercase());
    if let Some(parameter_count) = infer_parameter_count(display_name) {
        aliases.insert(parameter_count);
    }
    aliases.into_iter().collect()
}

fn infer_roles(display_name: &str) -> Vec<RoutingRole> {
    let name = display_name.to_ascii_lowercase();
    let mut roles = vec![RoutingRole::Helper];

    if name.contains("code") || name.contains("coder") || name.contains("webgen") {
        push_role(&mut roles, RoutingRole::Coding);
    }
    if name.contains("xlam") || name.contains("tool") || name.contains("fc") {
        push_role(&mut roles, RoutingRole::ToolAgent);
    }
    if name.contains("reason") || name.contains("qwen") || name.contains("ministral") {
        push_role(&mut roles, RoutingRole::Reasoning);
    }
    if name.contains("vl") || name.contains("vision") {
        push_role(&mut roles, RoutingRole::Vision);
    }

    roles
}

fn push_role(roles: &mut Vec<RoutingRole>, role: RoutingRole) {
    if !roles.contains(&role) {
        roles.push(role);
    }
}

fn infer_quantization(display_name: &str) -> Option<String> {
    let upper = display_name.to_ascii_uppercase();
    for marker in [
        "Q2_K", "Q3_K_S", "Q3_K_M", "Q3_K_L", "Q4_0", "Q4_1", "Q4_K_S", "Q4_K_M", "Q5_0", "Q5_1",
        "Q5_K_S", "Q5_K_M", "Q6_K", "Q8_0", "IQ2", "IQ3", "IQ4", "F16", "BF16", "INT8",
    ] {
        if upper.contains(marker) {
            return Some(marker.to_string());
        }
    }
    None
}

fn infer_parameter_count(display_name: &str) -> Option<String> {
    let chars = display_name.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        if !chars[index].is_ascii_digit() {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
            index += 1;
        }

        if index < chars.len() && matches!(chars[index], 'b' | 'B' | 'm' | 'M') {
            let mut value = chars[start..=index].iter().collect::<String>();
            value.make_ascii_uppercase();
            return Some(value);
        }
    }
    None
}

fn is_local_model_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            MODEL_EXTENSIONS
                .iter()
                .any(|model_extension| extension.eq_ignore_ascii_case(model_extension))
        })
        .unwrap_or(false)
}

fn is_projector_model(file_name: &str) -> bool {
    let name = file_name.to_ascii_lowercase();
    name.contains("mmproj") || name.contains("projector")
}

fn should_skip_dir(file_name: &str) -> bool {
    SKIPPED_DIR_NAMES
        .iter()
        .any(|skipped| file_name.eq_ignore_ascii_case(skipped))
}

fn slug(value: String) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut previous_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "model".to_string()
    } else {
        slug
    }
}
