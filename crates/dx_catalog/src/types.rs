use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const DX_CATALOG_SCHEMA_VERSION: u16 = 1;

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct DxCatalog {
    pub schema_version: u16,
    pub generated_unix_ms: u64,
    pub source_revision: String,
    pub sources: Vec<CatalogSourceRecord>,
    pub providers: Vec<ProviderRecord>,
    pub models: Vec<ModelRecord>,
    pub routing_rules: Vec<RoutingRule>,
}

impl DxCatalog {
    pub fn empty(source_revision: impl Into<String>) -> Self {
        Self {
            schema_version: DX_CATALOG_SCHEMA_VERSION,
            generated_unix_ms: 0,
            source_revision: source_revision.into(),
            sources: Vec::new(),
            providers: Vec::new(),
            models: Vec::new(),
            routing_rules: Vec::new(),
        }
    }

    pub fn provider(&self, id: &str) -> Option<&ProviderRecord> {
        self.providers.iter().find(|provider| provider.id == id)
    }

    pub fn model(&self, id: &str) -> Option<&ModelRecord> {
        self.models.iter().find(|model| model.id == id)
    }

    pub fn validate_references(&self) -> CatalogValidationReport {
        let mut provider_ids = BTreeSet::new();
        let mut model_ids = BTreeSet::new();
        let mut duplicate_provider_ids = Vec::new();
        let mut duplicate_model_ids = Vec::new();
        let mut missing_provider_model_ids = Vec::new();
        let mut missing_route_model_ids = Vec::new();

        for provider in &self.providers {
            if !provider_ids.insert(provider.id.as_str()) {
                duplicate_provider_ids.push(provider.id.clone());
            }
        }

        for model in &self.models {
            if !model_ids.insert(model.id.as_str()) {
                duplicate_model_ids.push(model.id.clone());
            }

            if !provider_ids.contains(model.provider_id.as_str()) {
                missing_provider_model_ids.push(model.id.clone());
            }
        }

        for rule in &self.routing_rules {
            if !model_ids.contains(rule.primary_model_id.as_str()) {
                missing_route_model_ids.push(rule.primary_model_id.clone());
            }

            for model_id in &rule.fallback_model_ids {
                if !model_ids.contains(model_id.as_str()) {
                    missing_route_model_ids.push(model_id.clone());
                }
            }
        }

        let is_valid = duplicate_provider_ids.is_empty()
            && duplicate_model_ids.is_empty()
            && missing_provider_model_ids.is_empty()
            && missing_route_model_ids.is_empty()
            && self.schema_version == DX_CATALOG_SCHEMA_VERSION;

        CatalogValidationReport {
            is_valid,
            schema_version: self.schema_version,
            provider_count: self.providers.len() as u32,
            model_count: self.models.len() as u32,
            routing_rule_count: self.routing_rules.len() as u32,
            duplicate_provider_ids,
            duplicate_model_ids,
            missing_provider_model_ids,
            missing_route_model_ids,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct CatalogSourceRecord {
    pub id: String,
    pub kind: CatalogSourceKind,
    pub revision: Option<String>,
    pub generated_unix_ms: Option<u64>,
    pub notes: Option<String>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSourceKind {
    FlowLocalRoles,
    ZeroclawProviders,
    ModelsDev,
    LiteLlmAliases,
    OpenRouter,
    OllamaCompatible,
    LlamaCppScan,
    UserAuthProfiles,
    Manual,
    Unknown,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct ProviderRecord {
    pub id: String,
    pub display_name: String,
    pub kind: ProviderKind,
    pub auth: ProviderAuthKind,
    pub auth_profile: Option<AuthProfileLink>,
    pub aliases: Vec<String>,
    pub base_url: Option<String>,
    pub homepage_url: Option<String>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_free_tier: bool,
    pub supports_premium_account: bool,
    pub is_local: bool,
    pub is_enabled_by_default: bool,
    pub notes: Option<String>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    LocalLlamaCpp,
    OllamaCompatible,
    OpenAiCompatible,
    OpenRouter,
    Anthropic,
    GoogleAi,
    Bedrock,
    ModelsDev,
    LiteLlmAlias,
    NativeAccount,
    Custom,
    Unknown,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAuthKind {
    None,
    ApiKey,
    OAuth,
    BrowserSession,
    LocalRuntime,
    NativeAccount,
    Custom,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct AuthProfileLink {
    pub profile_id: String,
    pub configured: bool,
    pub secret_storage_key: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct ModelRecord {
    pub id: String,
    pub provider_id: String,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub capabilities: ModelCapabilities,
    pub context_window_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub pricing: Option<ModelPricingMicros>,
    pub local_runtime: Option<LocalRuntimeHints>,
    pub recommended_roles: Vec<RoutingRole>,
    pub free_tier_hint: Option<String>,
    pub premium_account_hint: Option<String>,
    pub notes: Option<String>,
}

#[derive(
    Debug,
    Default,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct ModelCapabilities {
    pub chat: bool,
    pub tools: bool,
    pub vision: bool,
    pub audio: bool,
    pub video: bool,
    pub embeddings: bool,
    pub coding: bool,
    pub reasoning: bool,
    pub local_runtime: bool,
    pub streaming: bool,
    pub free_tier: bool,
    pub premium_account: bool,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct ModelPricingMicros {
    pub input_per_million_tokens: Option<u64>,
    pub output_per_million_tokens: Option<u64>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct LocalRuntimeHints {
    pub runtime: LocalRuntimeKind,
    pub path_hint: Option<String>,
    pub quantization: Option<String>,
    pub parameter_count_hint: Option<String>,
    pub requires_gpu: bool,
    pub preferred_threads: Option<u16>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LocalRuntimeKind {
    LlamaCpp,
    Onnx,
    Candle,
    Vllm,
    OllamaCompatible,
    Custom,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RoutingRole {
    Helper,
    ToolAgent,
    Coding,
    Reasoning,
    Vision,
    Audio,
    Embeddings,
    Fallback,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct RoutingRule {
    pub role: RoutingRole,
    pub primary_model_id: String,
    pub fallback_model_ids: Vec<String>,
    pub offline_allowed: bool,
    pub prefer_free_tier: bool,
    pub prefer_local: bool,
    pub notes: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct CatalogValidationReport {
    pub is_valid: bool,
    pub schema_version: u16,
    pub provider_count: u32,
    pub model_count: u32,
    pub routing_rule_count: u32,
    pub duplicate_provider_ids: Vec<String>,
    pub duplicate_model_ids: Vec<String>,
    pub missing_provider_model_ids: Vec<String>,
    pub missing_route_model_ids: Vec<String>,
}
