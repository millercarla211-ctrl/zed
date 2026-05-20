use crate::{
    AuthProfileLink, CatalogGeneratorInput, CatalogSourceKind, CatalogSourceRecord,
    LocalRuntimeHints, LocalRuntimeKind, ModelCapabilities, ModelPricingMicros, ModelRecord,
    ProviderAuthKind, ProviderAuthProfileUpdate, ProviderKind, ProviderRecord, RoutingRole,
    RoutingRule,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMetadata {
    pub id: String,
    pub revision: Option<String>,
    pub generated_unix_ms: Option<u64>,
    pub notes: Option<String>,
}

impl SourceMetadata {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            revision: None,
            generated_unix_ms: None,
            notes: None,
        }
    }

    pub fn with_revision(mut self, revision: impl Into<String>) -> Self {
        self.revision = Some(revision.into());
        self
    }

    pub fn with_generated_unix_ms(mut self, generated_unix_ms: u64) -> Self {
        self.generated_unix_ms = Some(generated_unix_ms);
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    pub(crate) fn into_record(self, kind: CatalogSourceKind) -> CatalogSourceRecord {
        CatalogSourceRecord {
            id: self.id,
            kind,
            revision: self.revision,
            generated_unix_ms: self.generated_unix_ms,
            notes: self.notes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalProviderInput {
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

impl ExternalProviderInput {
    pub fn new(id: impl Into<String>, display_name: impl Into<String>, kind: ProviderKind) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            kind,
            auth: ProviderAuthKind::None,
            auth_profile: None,
            aliases: Vec::new(),
            base_url: None,
            homepage_url: None,
            supports_streaming: true,
            supports_tools: false,
            supports_free_tier: false,
            supports_premium_account: false,
            is_local: false,
            is_enabled_by_default: true,
            notes: None,
        }
    }

    fn into_record(self) -> ProviderRecord {
        ProviderRecord {
            id: self.id,
            display_name: self.display_name,
            kind: self.kind,
            auth: self.auth,
            auth_profile: self.auth_profile,
            aliases: self.aliases,
            base_url: self.base_url,
            homepage_url: self.homepage_url,
            supports_streaming: self.supports_streaming,
            supports_tools: self.supports_tools,
            supports_free_tier: self.supports_free_tier,
            supports_premium_account: self.supports_premium_account,
            is_local: self.is_local,
            is_enabled_by_default: self.is_enabled_by_default,
            notes: self.notes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalModelInput {
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

impl ExternalModelInput {
    pub fn new(
        id: impl Into<String>,
        provider_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            provider_id: provider_id.into(),
            display_name: display_name.into(),
            aliases: Vec::new(),
            capabilities: ModelCapabilities::default(),
            context_window_tokens: None,
            max_output_tokens: None,
            pricing: None,
            local_runtime: None,
            recommended_roles: Vec::new(),
            free_tier_hint: None,
            premium_account_hint: None,
            notes: None,
        }
    }

    fn into_record(self) -> ModelRecord {
        ModelRecord {
            id: self.id,
            provider_id: self.provider_id,
            display_name: self.display_name,
            aliases: self.aliases,
            capabilities: self.capabilities,
            context_window_tokens: self.context_window_tokens,
            max_output_tokens: self.max_output_tokens,
            pricing: self.pricing,
            local_runtime: self.local_runtime,
            recommended_roles: self.recommended_roles,
            free_tier_hint: self.free_tier_hint,
            premium_account_hint: self.premium_account_hint,
            notes: self.notes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowLocalRoleInput {
    pub role: RoutingRole,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub context_window_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub quantization: Option<String>,
    pub parameter_count_hint: Option<String>,
    pub path_hint: Option<String>,
    pub preferred_threads: Option<u16>,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub supports_coding: bool,
    pub supports_reasoning: bool,
    pub notes: Option<String>,
}

impl FlowLocalRoleInput {
    pub fn new(
        role: RoutingRole,
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            role,
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            display_name: display_name.into(),
            aliases: Vec::new(),
            context_window_tokens: None,
            max_output_tokens: None,
            quantization: None,
            parameter_count_hint: None,
            path_hint: None,
            preferred_threads: None,
            supports_tools: true,
            supports_vision: false,
            supports_audio: false,
            supports_coding: matches!(role, RoutingRole::Coding),
            supports_reasoning: matches!(role, RoutingRole::Reasoning),
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaCppModelInput {
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub path_hint: Option<String>,
    pub quantization: Option<String>,
    pub parameter_count_hint: Option<String>,
    pub context_window_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub aliases: Vec<String>,
    pub recommended_roles: Vec<RoutingRole>,
    pub notes: Option<String>,
}

impl LlamaCppModelInput {
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            display_name: display_name.into(),
            path_hint: None,
            quantization: None,
            parameter_count_hint: None,
            context_window_tokens: None,
            max_output_tokens: None,
            aliases: Vec::new(),
            recommended_roles: Vec::new(),
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteLlmAliasInput {
    pub provider_id: String,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub base_url: Option<String>,
    pub auth: ProviderAuthKind,
    pub supports_tools: bool,
    pub supports_free_tier: bool,
    pub notes: Option<String>,
}

impl LiteLlmAliasInput {
    pub fn new(provider_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            display_name: display_name.into(),
            aliases: Vec::new(),
            base_url: None,
            auth: ProviderAuthKind::ApiKey,
            supports_tools: true,
            supports_free_tier: false,
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthProfileInput {
    pub provider_id: String,
    pub profile_id: String,
    pub configured: bool,
    pub secret_storage_key: Option<String>,
    pub auth_kind: Option<ProviderAuthKind>,
}

impl AuthProfileInput {
    pub fn new(provider_id: impl Into<String>, profile_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            profile_id: profile_id.into(),
            configured: false,
            secret_storage_key: None,
            auth_kind: None,
        }
    }
}

pub fn flow_local_roles_input(
    metadata: SourceMetadata,
    roles: impl IntoIterator<Item = FlowLocalRoleInput>,
) -> CatalogGeneratorInput {
    let mut input =
        CatalogGeneratorInput::new(metadata.into_record(CatalogSourceKind::FlowLocalRoles));

    for role in roles {
        input.providers.push(local_llama_provider(
            role.provider_id.clone(),
            Some("Flow local model role provider".to_string()),
        ));
        input.models.push(flow_role_model(&role));
        input.routing_rules.push(RoutingRule {
            role: role.role,
            primary_model_id: role.model_id,
            fallback_model_ids: Vec::new(),
            offline_allowed: true,
            prefer_free_tier: false,
            prefer_local: true,
            notes: Some("Generated from Flow local role mapping".to_string()),
        });
    }

    input
}

pub fn llama_cpp_scan_input(
    metadata: SourceMetadata,
    models: impl IntoIterator<Item = LlamaCppModelInput>,
) -> CatalogGeneratorInput {
    let mut input =
        CatalogGeneratorInput::new(metadata.into_record(CatalogSourceKind::LlamaCppScan));

    for model in models {
        input.providers.push(local_llama_provider(
            model.provider_id.clone(),
            Some("Local llama.cpp scan provider".to_string()),
        ));
        input.models.push(llama_cpp_model(model));
    }

    input
}

pub fn zeroclaw_providers_input(
    metadata: SourceMetadata,
    providers: impl IntoIterator<Item = ExternalProviderInput>,
    models: impl IntoIterator<Item = ExternalModelInput>,
) -> CatalogGeneratorInput {
    external_catalog_input(
        metadata,
        CatalogSourceKind::ZeroclawProviders,
        providers,
        models,
    )
}

pub fn models_dev_input(
    metadata: SourceMetadata,
    providers: impl IntoIterator<Item = ExternalProviderInput>,
    models: impl IntoIterator<Item = ExternalModelInput>,
) -> CatalogGeneratorInput {
    external_catalog_input(metadata, CatalogSourceKind::ModelsDev, providers, models)
}

pub fn openrouter_input(
    metadata: SourceMetadata,
    providers: impl IntoIterator<Item = ExternalProviderInput>,
    models: impl IntoIterator<Item = ExternalModelInput>,
) -> CatalogGeneratorInput {
    external_catalog_input(metadata, CatalogSourceKind::OpenRouter, providers, models)
}

pub fn lite_llm_aliases_input(
    metadata: SourceMetadata,
    aliases: impl IntoIterator<Item = LiteLlmAliasInput>,
) -> CatalogGeneratorInput {
    let providers = aliases.into_iter().map(|alias| {
        let mut provider = ExternalProviderInput::new(
            alias.provider_id,
            alias.display_name,
            ProviderKind::LiteLlmAlias,
        );
        provider.auth = alias.auth;
        provider.aliases = alias.aliases;
        provider.base_url = alias.base_url;
        provider.supports_tools = alias.supports_tools;
        provider.supports_free_tier = alias.supports_free_tier;
        provider.notes = alias.notes;
        provider.into_record()
    });

    CatalogGeneratorInput::new(metadata.into_record(CatalogSourceKind::LiteLlmAliases))
        .with_providers(providers)
}

pub fn lite_llm_catalog_input(
    metadata: SourceMetadata,
    providers: impl IntoIterator<Item = ExternalProviderInput>,
    models: impl IntoIterator<Item = ExternalModelInput>,
) -> CatalogGeneratorInput {
    external_catalog_input(
        metadata,
        CatalogSourceKind::LiteLlmAliases,
        providers,
        models,
    )
}

pub fn auth_profiles_input(
    metadata: SourceMetadata,
    profiles: impl IntoIterator<Item = AuthProfileInput>,
) -> CatalogGeneratorInput {
    let auth_profiles = profiles
        .into_iter()
        .map(|profile| ProviderAuthProfileUpdate {
            provider_id: profile.provider_id,
            profile_id: profile.profile_id,
            configured: profile.configured,
            secret_storage_key: profile.secret_storage_key,
            auth_kind: profile.auth_kind,
        });

    CatalogGeneratorInput::new(metadata.into_record(CatalogSourceKind::UserAuthProfiles))
        .with_auth_profiles(auth_profiles)
}

fn external_catalog_input(
    metadata: SourceMetadata,
    source_kind: CatalogSourceKind,
    providers: impl IntoIterator<Item = ExternalProviderInput>,
    models: impl IntoIterator<Item = ExternalModelInput>,
) -> CatalogGeneratorInput {
    CatalogGeneratorInput::new(metadata.into_record(source_kind))
        .with_providers(
            providers
                .into_iter()
                .map(ExternalProviderInput::into_record),
        )
        .with_models(models.into_iter().map(ExternalModelInput::into_record))
}

fn local_llama_provider(id: String, notes: Option<String>) -> ProviderRecord {
    ProviderRecord {
        id,
        display_name: "Local llama.cpp".to_string(),
        kind: ProviderKind::LocalLlamaCpp,
        auth: ProviderAuthKind::LocalRuntime,
        auth_profile: None,
        aliases: vec!["llama.cpp".to_string(), "local".to_string()],
        base_url: None,
        homepage_url: None,
        supports_streaming: true,
        supports_tools: true,
        supports_free_tier: true,
        supports_premium_account: false,
        is_local: true,
        is_enabled_by_default: true,
        notes,
    }
}

fn flow_role_model(role: &FlowLocalRoleInput) -> ModelRecord {
    let mut capabilities = local_model_capabilities();
    capabilities.tools = role.supports_tools;
    capabilities.vision = role.supports_vision;
    capabilities.audio = role.supports_audio;
    capabilities.coding = role.supports_coding || matches!(role.role, RoutingRole::Coding);
    capabilities.reasoning = role.supports_reasoning || matches!(role.role, RoutingRole::Reasoning);

    ModelRecord {
        id: role.model_id.clone(),
        provider_id: role.provider_id.clone(),
        display_name: role.display_name.clone(),
        aliases: role.aliases.clone(),
        capabilities,
        context_window_tokens: role.context_window_tokens,
        max_output_tokens: role.max_output_tokens,
        pricing: None,
        local_runtime: Some(LocalRuntimeHints {
            runtime: LocalRuntimeKind::LlamaCpp,
            path_hint: role.path_hint.clone(),
            quantization: role.quantization.clone(),
            parameter_count_hint: role.parameter_count_hint.clone(),
            requires_gpu: false,
            preferred_threads: role.preferred_threads,
        }),
        recommended_roles: vec![role.role],
        free_tier_hint: Some("Local runtime".to_string()),
        premium_account_hint: None,
        notes: role.notes.clone(),
    }
}

fn llama_cpp_model(model: LlamaCppModelInput) -> ModelRecord {
    ModelRecord {
        id: model.model_id,
        provider_id: model.provider_id,
        display_name: model.display_name,
        aliases: model.aliases,
        capabilities: local_model_capabilities(),
        context_window_tokens: model.context_window_tokens,
        max_output_tokens: model.max_output_tokens,
        pricing: None,
        local_runtime: Some(LocalRuntimeHints {
            runtime: LocalRuntimeKind::LlamaCpp,
            path_hint: model.path_hint,
            quantization: model.quantization,
            parameter_count_hint: model.parameter_count_hint,
            requires_gpu: false,
            preferred_threads: None,
        }),
        recommended_roles: model.recommended_roles,
        free_tier_hint: Some("Local runtime".to_string()),
        premium_account_hint: None,
        notes: model.notes,
    }
}

fn local_model_capabilities() -> ModelCapabilities {
    ModelCapabilities {
        chat: true,
        tools: true,
        vision: false,
        audio: false,
        video: false,
        embeddings: false,
        coding: true,
        reasoning: false,
        local_runtime: true,
        streaming: true,
        free_tier: true,
        premium_account: false,
    }
}
