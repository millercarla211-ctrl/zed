use crate::{
    AuthProfileInput, CatalogGeneratorInput, CatalogSourceCandidateStatus, CatalogSourceKind,
    CatalogSourcePurpose, ExternalProviderInput, LiteLlmAliasInput, ProviderAuthKind, ProviderKind,
    Result, SourceMetadata, auth_profiles_input, lite_llm_aliases_input, models_dev_input,
    openrouter_input, zeroclaw_providers_input,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const DEFAULT_SOURCE_ID: &str = "flow-provider-source";
const DEFAULT_AUTH_SOURCE_ID: &str = "flow-provider-auth-source";
const DEFAULT_SECRET_STORAGE_PREFIX: &str = "env";
const ENV_FILE_NAMES: &[&str] = &[".env", ".env.local"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderSourceReaderOptions {
    pub source_id: String,
    pub auth_source_id: String,
    pub source_revision: Option<String>,
    pub generated_unix_ms: Option<u64>,
    pub include_integration_dirs: bool,
    pub include_env_auth_profiles: bool,
    pub secret_storage_prefix: String,
}

impl ProviderSourceReaderOptions {
    pub fn new() -> Self {
        Self {
            source_id: DEFAULT_SOURCE_ID.to_string(),
            auth_source_id: DEFAULT_AUTH_SOURCE_ID.to_string(),
            source_revision: None,
            generated_unix_ms: None,
            include_integration_dirs: true,
            include_env_auth_profiles: true,
            secret_storage_prefix: DEFAULT_SECRET_STORAGE_PREFIX.to_string(),
        }
    }

    pub fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = source_id.into();
        self
    }

    pub fn with_auth_source_id(mut self, auth_source_id: impl Into<String>) -> Self {
        self.auth_source_id = auth_source_id.into();
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

    pub fn include_integration_dirs(mut self, include_integration_dirs: bool) -> Self {
        self.include_integration_dirs = include_integration_dirs;
        self
    }

    pub fn include_env_auth_profiles(mut self, include_env_auth_profiles: bool) -> Self {
        self.include_env_auth_profiles = include_env_auth_profiles;
        self
    }

    pub fn with_secret_storage_prefix(mut self, secret_storage_prefix: impl Into<String>) -> Self {
        self.secret_storage_prefix = secret_storage_prefix.into();
        self
    }
}

impl Default for ProviderSourceReaderOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ProviderSourceReadOutput {
    pub input: CatalogGeneratorInput,
    pub report: ProviderSourceReadReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderSourceReadReport {
    pub root: PathBuf,
    pub source_kind: CatalogSourceKind,
    pub source_id: String,
    pub source_available: bool,
    pub provider_count: u32,
    pub auth_profile_count: u32,
    pub integration_count: u32,
    pub env_key_count: u32,
    pub discovered_provider_ids: Vec<String>,
    pub discovered_auth_profile_ids: Vec<String>,
    pub skipped_entries: Vec<SkippedProviderSourceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SkippedProviderSourceEntry {
    pub path: PathBuf,
    pub reason: String,
}

pub fn read_provider_source(
    source: &CatalogSourceCandidateStatus,
    options: ProviderSourceReaderOptions,
) -> Result<ProviderSourceReadOutput> {
    let supported = matches!(
        source.kind,
        CatalogSourceKind::ZeroclawProviders
            | CatalogSourceKind::ModelsDev
            | CatalogSourceKind::OpenRouter
            | CatalogSourceKind::LiteLlmAliases
            | CatalogSourceKind::UserAuthProfiles
    ) && matches!(
        source.purpose,
        CatalogSourcePurpose::ProviderCatalog | CatalogSourcePurpose::AuthProfiles
    );

    if !source.available || !supported {
        return Ok(provider_source_output(
            source.root.clone(),
            source.kind,
            options,
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            0,
            0,
            false,
        ));
    }

    read_provider_source_root(&source.root, source.kind, options)
}

pub fn read_provider_source_root(
    root: impl AsRef<Path>,
    source_kind: CatalogSourceKind,
    options: ProviderSourceReaderOptions,
) -> Result<ProviderSourceReadOutput> {
    let root = root.as_ref().to_path_buf();
    let source_available = root.exists();
    let mut providers = BTreeMap::new();
    let mut skipped_entries = Vec::new();
    let mut integration_count = 0;

    if source_available && options.include_integration_dirs {
        integration_count =
            read_integration_providers(&root, &mut providers, &mut skipped_entries)?;
    }

    let env_keys = if source_available {
        read_provider_env_keys(&root)?
    } else {
        BTreeSet::new()
    };

    for env_key in &env_keys {
        if let Some(provider) = provider_from_env_key(env_key) {
            upsert_provider(&mut providers, provider);
        }
    }

    ensure_source_specific_provider(source_kind, &mut providers);

    let auth_profiles = if source_available && options.include_env_auth_profiles {
        auth_profiles_from_root(&root, &env_keys, &options)?
    } else {
        Vec::new()
    };

    Ok(provider_source_output(
        root,
        source_kind,
        options,
        providers,
        auth_profiles,
        skipped_entries,
        integration_count,
        env_keys.len() as u32,
        source_available,
    ))
}

fn provider_source_output(
    root: PathBuf,
    source_kind: CatalogSourceKind,
    options: ProviderSourceReaderOptions,
    providers: BTreeMap<String, ExternalProviderInput>,
    auth_profiles: Vec<AuthProfileInput>,
    skipped_entries: Vec<SkippedProviderSourceEntry>,
    integration_count: u32,
    env_key_count: u32,
    source_available: bool,
) -> ProviderSourceReadOutput {
    let provider_values = providers.into_values().collect::<Vec<_>>();
    let auth_profile_ids = auth_profiles
        .iter()
        .map(|profile| profile.profile_id.clone())
        .collect::<Vec<_>>();
    let provider_ids = provider_values
        .iter()
        .map(|provider| provider.id.clone())
        .collect::<Vec<_>>();
    let provider_count = provider_values.len() as u32;
    let auth_profile_count = auth_profiles.len() as u32;
    let source_id = if source_kind == CatalogSourceKind::UserAuthProfiles {
        options.auth_source_id.clone()
    } else {
        options.source_id.clone()
    };
    let metadata = source_metadata(&options, source_kind, &root, provider_count);

    let mut input = match source_kind {
        CatalogSourceKind::ModelsDev => models_dev_input(metadata, provider_values, Vec::new()),
        CatalogSourceKind::OpenRouter => openrouter_input(metadata, provider_values, Vec::new()),
        CatalogSourceKind::LiteLlmAliases => {
            let aliases = provider_values
                .into_iter()
                .map(lite_llm_alias_from_provider)
                .collect::<Vec<_>>();
            lite_llm_aliases_input(metadata, aliases)
        }
        CatalogSourceKind::UserAuthProfiles => auth_profiles_input(
            auth_source_metadata(&options, &root, auth_profile_count),
            auth_profiles.clone(),
        ),
        _ => zeroclaw_providers_input(metadata, provider_values, Vec::new()),
    };

    if source_kind != CatalogSourceKind::UserAuthProfiles && !auth_profiles.is_empty() {
        let auth_input = auth_profiles_input(
            auth_source_metadata(&options, &root, auth_profile_count),
            auth_profiles.clone(),
        );
        input.auth_profiles.extend(auth_input.auth_profiles);
    }

    ProviderSourceReadOutput {
        input,
        report: ProviderSourceReadReport {
            root,
            source_kind,
            source_id,
            source_available,
            provider_count,
            auth_profile_count,
            integration_count,
            env_key_count,
            discovered_provider_ids: provider_ids,
            discovered_auth_profile_ids: auth_profile_ids,
            skipped_entries,
        },
    }
}

fn source_metadata(
    options: &ProviderSourceReaderOptions,
    source_kind: CatalogSourceKind,
    root: &Path,
    provider_count: u32,
) -> SourceMetadata {
    let mut metadata = SourceMetadata::new(options.source_id.clone()).with_notes(format!(
        "Provider source scan; kind={source_kind:?}; root={}; providers={provider_count}",
        root.display()
    ));

    if let Some(source_revision) = &options.source_revision {
        metadata = metadata.with_revision(source_revision.clone());
    }
    if let Some(generated_unix_ms) = options.generated_unix_ms {
        metadata = metadata.with_generated_unix_ms(generated_unix_ms);
    }

    metadata
}

fn auth_source_metadata(
    options: &ProviderSourceReaderOptions,
    root: &Path,
    auth_profile_count: u32,
) -> SourceMetadata {
    let mut metadata = SourceMetadata::new(options.auth_source_id.clone()).with_notes(format!(
        "Secret-safe provider auth profile scan; root={}; profiles={auth_profile_count}",
        root.display()
    ));

    if let Some(source_revision) = &options.source_revision {
        metadata = metadata.with_revision(source_revision.clone());
    }
    if let Some(generated_unix_ms) = options.generated_unix_ms {
        metadata = metadata.with_generated_unix_ms(generated_unix_ms);
    }

    metadata
}

fn read_integration_providers(
    root: &Path,
    providers: &mut BTreeMap<String, ExternalProviderInput>,
    skipped_entries: &mut Vec<SkippedProviderSourceEntry>,
) -> Result<u32> {
    let integrations_root = root.join("integrations");
    if !integrations_root.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in fs::read_dir(&integrations_root)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_dir() {
            skipped_entries.push(SkippedProviderSourceEntry {
                path,
                reason: "integration entry is not a directory".to_string(),
            });
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }

        let provider = provider_from_integration_name(&name, &path);
        upsert_provider(providers, provider);
        count += 1;
    }

    Ok(count)
}

fn read_provider_env_keys(root: &Path) -> Result<BTreeSet<String>> {
    let mut keys = BTreeSet::new();
    for file_name in ENV_FILE_NAMES {
        let env_path = root.join(file_name);
        if !env_path.exists() {
            continue;
        }

        let contents = fs::read_to_string(env_path)?;
        for line in contents.lines() {
            if let Some(key) = env_key(line) {
                keys.insert(key.to_string());
            }
        }
    }
    Ok(keys)
}

fn auth_profiles_from_root(
    root: &Path,
    env_keys: &BTreeSet<String>,
    options: &ProviderSourceReaderOptions,
) -> Result<Vec<AuthProfileInput>> {
    let mut profiles = BTreeMap::new();

    for env_key in env_keys {
        if let Some(template) = provider_template_from_env_key(env_key) {
            let mut profile = AuthProfileInput::new(
                template.provider_id,
                format!("{}-env", template.provider_id),
            );
            profile.configured = true;
            profile.secret_storage_key =
                Some(format!("{}:{}", options.secret_storage_prefix, env_key));
            profile.auth_kind = Some(template.auth);
            profiles.insert(profile.profile_id.clone(), profile);
        }
    }

    for provider_id in zeroclaw_provider_sections(root)? {
        let mut profile = AuthProfileInput::new(
            provider_id.clone(),
            format!("{provider_id}-zeroclaw-config"),
        );
        profile.configured = true;
        profile.secret_storage_key = Some(format!("zeroclaw:config.toml:{provider_id}"));
        profile.auth_kind = Some(ProviderAuthKind::ApiKey);
        profiles.insert(profile.profile_id.clone(), profile);
    }

    Ok(profiles.into_values().collect())
}

fn zeroclaw_provider_sections(root: &Path) -> Result<Vec<String>> {
    let config_path = root.join("config.toml");
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(config_path)?;
    let mut providers = BTreeSet::new();
    for line in contents.lines() {
        let line = line.trim();
        if let Some(section) = line
            .strip_prefix("[providers.models.")
            .and_then(|line| line.strip_suffix(']'))
        {
            providers.insert(slug(section));
        }
    }
    Ok(providers.into_iter().collect())
}

fn ensure_source_specific_provider(
    source_kind: CatalogSourceKind,
    providers: &mut BTreeMap<String, ExternalProviderInput>,
) {
    match source_kind {
        CatalogSourceKind::ModelsDev => upsert_provider(
            providers,
            provider(
                "models-dev",
                "models.dev",
                ProviderKind::ModelsDev,
                ProviderAuthKind::None,
            ),
        ),
        CatalogSourceKind::OpenRouter => upsert_provider(
            providers,
            provider(
                "openrouter",
                "OpenRouter",
                ProviderKind::OpenRouter,
                ProviderAuthKind::ApiKey,
            ),
        ),
        CatalogSourceKind::LiteLlmAliases => upsert_provider(
            providers,
            provider(
                "lite-llm",
                "LiteLLM",
                ProviderKind::LiteLlmAlias,
                ProviderAuthKind::ApiKey,
            ),
        ),
        _ => {}
    }
}

fn provider_from_integration_name(name: &str, path: &Path) -> ExternalProviderInput {
    let lower = name.to_ascii_lowercase();
    let (provider_id, display_name, kind, auth) = if lower.contains("gemini") {
        (
            "gemini-cli".to_string(),
            "Gemini CLI".to_string(),
            ProviderKind::GoogleAi,
            ProviderAuthKind::OAuth,
        )
    } else if lower.contains("qwen") {
        (
            "qwen-code".to_string(),
            "Qwen Code".to_string(),
            ProviderKind::OpenAiCompatible,
            ProviderAuthKind::OAuth,
        )
    } else {
        (
            slug(name),
            title_case(name),
            ProviderKind::Custom,
            ProviderAuthKind::Custom,
        )
    };

    let mut provider = provider(provider_id, display_name, kind, auth);
    provider.supports_tools = true;
    provider.supports_free_tier = true;
    provider.supports_premium_account = true;
    provider.notes = Some(format!(
        "Detected from integration directory {}",
        path.display()
    ));
    provider
}

fn provider_from_env_key(env_key: &str) -> Option<ExternalProviderInput> {
    provider_template_from_env_key(env_key).map(|template| {
        let mut provider = provider(
            template.provider_id,
            template.display_name,
            template.kind,
            template.auth,
        );
        provider.supports_tools = true;
        provider.supports_free_tier = true;
        provider.supports_premium_account = true;
        provider.notes = Some(format!("Detected from secret-safe env key {env_key}"));
        provider
    })
}

fn provider_template_from_env_key(env_key: &str) -> Option<ProviderTemplate> {
    let upper = env_key.to_ascii_uppercase();
    let template = match upper.as_str() {
        "ANTHROPIC" | "ANTHROPIC_API_KEY" => ProviderTemplate::new(
            "anthropic",
            "Anthropic",
            ProviderKind::Anthropic,
            ProviderAuthKind::ApiKey,
        ),
        "CEREBRAS" | "CEREBRAS_API_KEY" => openai_compatible("cerebras", "Cerebras"),
        "COHERE" | "COHERE_API_KEY" => openai_compatible("cohere", "Cohere"),
        "DEEPSEEK" | "DEEPSEEK_API_KEY" => openai_compatible("deepseek", "DeepSeek"),
        "FIREWORKS" | "FIREWORKS_API_KEY" => openai_compatible("fireworks", "Fireworks"),
        "GEMINI" | "GEMINI_API_KEY" | "GOOGLE_API_KEY" => ProviderTemplate::new(
            "gemini-cli",
            "Gemini",
            ProviderKind::GoogleAi,
            ProviderAuthKind::ApiKey,
        ),
        "GITHUB_MODELS" | "GITHUB_TOKEN" => openai_compatible("github-models", "GitHub Models"),
        "GROQ" | "GROQ_API_KEY" => openai_compatible("groq", "Groq"),
        "HUGGINGFACE" | "HUGGINGFACE_API_KEY" | "HF_TOKEN" => ProviderTemplate::new(
            "huggingface",
            "Hugging Face",
            ProviderKind::Custom,
            ProviderAuthKind::ApiKey,
        ),
        "META_LLAMA" | "META_LLAMA_API_KEY" => openai_compatible("meta-llama", "Meta Llama"),
        "MISTRAL" | "MISTRAL_API_KEY" => openai_compatible("mistral", "Mistral"),
        "NVIDIA" | "NVIDIA_API_KEY" => openai_compatible("nvidia", "NVIDIA"),
        "OPENAI" | "OPENAI_API_KEY" => openai_compatible("openai", "OpenAI"),
        "OPENROUTER" | "OPENROUTER_API_KEY" => ProviderTemplate::new(
            "openrouter",
            "OpenRouter",
            ProviderKind::OpenRouter,
            ProviderAuthKind::ApiKey,
        ),
        "PERPLEXITY" | "PERPLEXITY_API_KEY" => openai_compatible("perplexity", "Perplexity"),
        "QWEN" | "QWEN_API_KEY" | "DASHSCOPE" | "DASHSCOPE_API_KEY" => {
            openai_compatible("qwen-code", "Qwen Code")
        }
        "REPLICATE" | "REPLICATE_API_TOKEN" => ProviderTemplate::new(
            "replicate",
            "Replicate",
            ProviderKind::Custom,
            ProviderAuthKind::ApiKey,
        ),
        "SAMBANOVA" | "SAMBANOVA_API_KEY" => openai_compatible("sambanova", "SambaNova"),
        "TOGETHER" | "TOGETHER_API_KEY" => openai_compatible("together", "Together AI"),
        "XAI_GROK" | "XAI_API_KEY" => openai_compatible("xai-grok", "xAI Grok"),
        _ => return None,
    };
    Some(template)
}

fn openai_compatible(provider_id: &'static str, display_name: &'static str) -> ProviderTemplate {
    ProviderTemplate::new(
        provider_id,
        display_name,
        ProviderKind::OpenAiCompatible,
        ProviderAuthKind::ApiKey,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProviderTemplate {
    provider_id: &'static str,
    display_name: &'static str,
    kind: ProviderKind,
    auth: ProviderAuthKind,
}

impl ProviderTemplate {
    fn new(
        provider_id: &'static str,
        display_name: &'static str,
        kind: ProviderKind,
        auth: ProviderAuthKind,
    ) -> Self {
        Self {
            provider_id,
            display_name,
            kind,
            auth,
        }
    }
}

fn provider(
    provider_id: impl Into<String>,
    display_name: impl Into<String>,
    kind: ProviderKind,
    auth: ProviderAuthKind,
) -> ExternalProviderInput {
    let mut provider = ExternalProviderInput::new(provider_id, display_name, kind);
    provider.auth = auth;
    provider.supports_streaming = true;
    provider.supports_tools = false;
    provider.supports_free_tier = false;
    provider.supports_premium_account = false;
    provider.is_enabled_by_default = true;
    provider
}

fn lite_llm_alias_from_provider(provider: ExternalProviderInput) -> LiteLlmAliasInput {
    let ExternalProviderInput {
        id,
        display_name,
        auth,
        aliases,
        base_url,
        supports_tools,
        supports_free_tier,
        notes,
        ..
    } = provider;

    let mut alias = LiteLlmAliasInput::new(format!("lite-llm-{id}"), display_name);
    alias.aliases = aliases;
    alias.aliases.push(id);
    alias.base_url = base_url;
    alias.auth = auth;
    alias.supports_tools = supports_tools;
    alias.supports_free_tier = supports_free_tier;
    alias.notes = notes;
    alias
}

fn upsert_provider(
    providers: &mut BTreeMap<String, ExternalProviderInput>,
    provider: ExternalProviderInput,
) {
    providers
        .entry(provider.id.clone())
        .and_modify(|existing| merge_provider(existing, &provider))
        .or_insert(provider);
}

fn merge_provider(existing: &mut ExternalProviderInput, incoming: &ExternalProviderInput) {
    existing.supports_tools |= incoming.supports_tools;
    existing.supports_streaming |= incoming.supports_streaming;
    existing.supports_free_tier |= incoming.supports_free_tier;
    existing.supports_premium_account |= incoming.supports_premium_account;
    existing.is_enabled_by_default |= incoming.is_enabled_by_default;
    if existing.notes.is_none() {
        existing.notes = incoming.notes.clone();
    }
    for alias in &incoming.aliases {
        if !existing.aliases.contains(alias) {
            existing.aliases.push(alias.clone());
        }
    }
}

fn env_key(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (key, _) = line.split_once('=')?;
    let key = key.trim();
    if key
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Some(key)
    } else {
        None
    }
}

fn title_case(value: &str) -> String {
    if value.eq_ignore_ascii_case("qwen-code") {
        "Qwen Code".to_string()
    } else if value.eq_ignore_ascii_case("gemini-cli") {
        "Gemini CLI".to_string()
    } else {
        value
            .split(['-', '_'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn slug(value: &str) -> String {
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
        "provider".to_string()
    } else {
        slug
    }
}
