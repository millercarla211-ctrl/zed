use crate::{
    AgentRoutePreferences, DxCatalog, LocalRuntimeKind, ModelCapabilities, ModelRecord,
    ProviderAuthKind, ProviderKind, ProviderRecord, RoutingRole, select_catalog_route,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogExecutionPlanRequest {
    pub model_id: Option<String>,
    pub role: Option<RoutingRole>,
    pub prefer_local: bool,
    pub prefer_free_tier: bool,
    pub require_selectable: bool,
    pub allow_premium: bool,
    pub max_fallbacks: usize,
}

impl CatalogExecutionPlanRequest {
    pub fn for_model_id(model_id: impl Into<String>) -> Self {
        Self {
            model_id: Some(model_id.into()),
            role: None,
            prefer_local: false,
            prefer_free_tier: true,
            require_selectable: true,
            allow_premium: true,
            max_fallbacks: 4,
        }
    }

    pub fn for_role(role: RoutingRole) -> Self {
        let preferences = AgentRoutePreferences::new(role);
        Self {
            model_id: None,
            role: Some(role),
            prefer_local: preferences.prefer_local,
            prefer_free_tier: preferences.prefer_free_tier,
            require_selectable: preferences.require_selectable,
            allow_premium: preferences.allow_premium,
            max_fallbacks: preferences.max_fallbacks,
        }
    }

    fn route_preferences(&self) -> Option<AgentRoutePreferences> {
        Some(
            AgentRoutePreferences::new(self.role?)
                .prefer_local(self.prefer_local)
                .prefer_free_tier(self.prefer_free_tier)
                .require_selectable(self.require_selectable)
                .allow_premium(self.allow_premium)
                .with_max_fallbacks(self.max_fallbacks),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogExecutionPlan {
    pub requested_model_id: Option<String>,
    pub role: Option<RoutingRole>,
    pub primary_model_id: String,
    pub fallback_model_ids: Vec<String>,
    pub provider_id: String,
    pub provider_name: String,
    pub model_display_name: String,
    pub adapter_kind: CatalogExecutionAdapterKind,
    pub permission: CatalogExecutionPermission,
    pub auth_configured: bool,
    pub base_url: Option<String>,
    pub local_runtime: Option<LocalRuntimeKind>,
    pub can_stream: bool,
    pub can_use_tools: bool,
    pub capabilities: ModelCapabilities,
    pub ready_for_adapter_registration: bool,
    pub blockers: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogExecutionAdapterKind {
    LocalLlamaCpp,
    OllamaCompatibleHttp,
    OpenAiCompatibleHttp,
    OpenRouterHttp,
    LiteLlmProxy,
    AnthropicHttp,
    GoogleAiHttp,
    BedrockRuntime,
    NativeAccount,
    Custom,
    Unsupported,
}

impl CatalogExecutionAdapterKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::LocalLlamaCpp => "local llama.cpp",
            Self::OllamaCompatibleHttp => "Ollama-compatible HTTP",
            Self::OpenAiCompatibleHttp => "OpenAI-compatible HTTP",
            Self::OpenRouterHttp => "OpenRouter HTTP",
            Self::LiteLlmProxy => "LiteLLM proxy",
            Self::AnthropicHttp => "Anthropic HTTP",
            Self::GoogleAiHttp => "Google AI HTTP",
            Self::BedrockRuntime => "Bedrock runtime",
            Self::NativeAccount => "native account",
            Self::Custom => "custom",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogExecutionPermission {
    None,
    ApiKey,
    OAuth,
    BrowserSession,
    LocalRuntime,
    NativeAccount,
    Custom,
}

impl CatalogExecutionPermission {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "no extra credential",
            Self::ApiKey => "API key",
            Self::OAuth => "OAuth account",
            Self::BrowserSession => "browser session",
            Self::LocalRuntime => "local runtime approval",
            Self::NativeAccount => "native account",
            Self::Custom => "custom setup",
        }
    }
}

pub fn build_catalog_execution_plan(
    catalog: &DxCatalog,
    request: CatalogExecutionPlanRequest,
) -> Option<CatalogExecutionPlan> {
    let (primary_model_id, fallback_model_ids) = if let Some(model_id) = &request.model_id {
        (model_id.clone(), Vec::new())
    } else {
        let route = select_catalog_route(catalog, request.route_preferences()?)?;
        (route.primary_model_id, route.fallback_model_ids)
    };
    let model = catalog.model(&primary_model_id)?;
    let provider = catalog.provider(&model.provider_id)?;
    Some(execution_plan_for_model(
        request,
        model,
        provider,
        fallback_model_ids,
    ))
}

fn execution_plan_for_model(
    request: CatalogExecutionPlanRequest,
    model: &ModelRecord,
    provider: &ProviderRecord,
    fallback_model_ids: Vec<String>,
) -> CatalogExecutionPlan {
    let adapter_kind = adapter_kind(provider);
    let permission = permission(provider);
    let auth_configured = auth_configured(provider, permission);
    let base_url = effective_base_url(provider);
    let mut blockers = execution_blockers(
        provider,
        model,
        adapter_kind,
        permission,
        auth_configured,
        base_url.as_deref(),
    );
    blockers.sort();
    blockers.dedup();
    let ready_for_adapter_registration = blockers.is_empty();
    let next_action = if ready_for_adapter_registration {
        format!(
            "Register a permissioned {} adapter for provider `{}` and model `{}`.",
            adapter_kind.label(),
            provider.id,
            model.id
        )
    } else {
        blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "Resolve catalog execution prerequisites.".to_string())
    };

    CatalogExecutionPlan {
        requested_model_id: request.model_id,
        role: request.role,
        primary_model_id: model.id.clone(),
        fallback_model_ids,
        provider_id: provider.id.clone(),
        provider_name: provider.display_name.clone(),
        model_display_name: model.display_name.clone(),
        adapter_kind,
        permission,
        auth_configured,
        base_url,
        local_runtime: model.local_runtime.as_ref().map(|runtime| runtime.runtime),
        can_stream: provider.supports_streaming || model.capabilities.streaming,
        can_use_tools: provider.supports_tools || model.capabilities.tools,
        capabilities: model.capabilities.clone(),
        ready_for_adapter_registration,
        blockers,
        next_action,
    }
}

fn adapter_kind(provider: &ProviderRecord) -> CatalogExecutionAdapterKind {
    match provider.kind {
        ProviderKind::LocalLlamaCpp => CatalogExecutionAdapterKind::LocalLlamaCpp,
        ProviderKind::OllamaCompatible => CatalogExecutionAdapterKind::OllamaCompatibleHttp,
        ProviderKind::OpenAiCompatible => CatalogExecutionAdapterKind::OpenAiCompatibleHttp,
        ProviderKind::OpenRouter => CatalogExecutionAdapterKind::OpenRouterHttp,
        ProviderKind::Anthropic => CatalogExecutionAdapterKind::AnthropicHttp,
        ProviderKind::GoogleAi => CatalogExecutionAdapterKind::GoogleAiHttp,
        ProviderKind::Bedrock => CatalogExecutionAdapterKind::BedrockRuntime,
        ProviderKind::LiteLlmAlias => CatalogExecutionAdapterKind::LiteLlmProxy,
        ProviderKind::NativeAccount => CatalogExecutionAdapterKind::NativeAccount,
        ProviderKind::Custom => CatalogExecutionAdapterKind::Custom,
        ProviderKind::ModelsDev | ProviderKind::Unknown => CatalogExecutionAdapterKind::Unsupported,
    }
}

fn permission(provider: &ProviderRecord) -> CatalogExecutionPermission {
    match provider.auth {
        ProviderAuthKind::None => CatalogExecutionPermission::None,
        ProviderAuthKind::ApiKey => CatalogExecutionPermission::ApiKey,
        ProviderAuthKind::OAuth => CatalogExecutionPermission::OAuth,
        ProviderAuthKind::BrowserSession => CatalogExecutionPermission::BrowserSession,
        ProviderAuthKind::LocalRuntime => CatalogExecutionPermission::LocalRuntime,
        ProviderAuthKind::NativeAccount => CatalogExecutionPermission::NativeAccount,
        ProviderAuthKind::Custom => CatalogExecutionPermission::Custom,
    }
}

fn auth_configured(provider: &ProviderRecord, permission: CatalogExecutionPermission) -> bool {
    match permission {
        CatalogExecutionPermission::None | CatalogExecutionPermission::LocalRuntime => true,
        CatalogExecutionPermission::ApiKey
        | CatalogExecutionPermission::OAuth
        | CatalogExecutionPermission::BrowserSession
        | CatalogExecutionPermission::NativeAccount
        | CatalogExecutionPermission::Custom => provider
            .auth_profile
            .as_ref()
            .is_some_and(|profile| profile.configured),
    }
}

fn effective_base_url(provider: &ProviderRecord) -> Option<String> {
    provider.base_url.clone().or_else(|| match provider.kind {
        ProviderKind::OpenRouter => Some("https://openrouter.ai/api/v1".to_string()),
        ProviderKind::OllamaCompatible => Some("http://localhost:11434/v1".to_string()),
        _ => None,
    })
}

fn execution_blockers(
    provider: &ProviderRecord,
    model: &ModelRecord,
    adapter_kind: CatalogExecutionAdapterKind,
    permission: CatalogExecutionPermission,
    auth_configured: bool,
    base_url: Option<&str>,
) -> Vec<String> {
    let mut blockers = Vec::new();

    if adapter_kind == CatalogExecutionAdapterKind::Unsupported {
        blockers.push(format!(
            "No catalog execution adapter is defined for provider kind `{:?}`.",
            provider.kind
        ));
    }

    if adapter_requires_base_url(adapter_kind) && base_url.is_none() {
        blockers.push(format!(
            "Provider `{}` needs a base URL before an HTTP adapter can be registered.",
            provider.id
        ));
    }

    if !auth_configured {
        blockers.push(format!(
            "Provider `{}` requires {} configuration before execution.",
            provider.id,
            permission.label()
        ));
    }

    if permission == CatalogExecutionPermission::LocalRuntime || model.local_runtime.is_some() {
        blockers.push(format!(
            "Model `{}` needs explicit local runtime approval before execution.",
            model.id
        ));
    }

    if !provider.is_enabled_by_default {
        blockers.push(format!(
            "Provider `{}` is disabled by default and needs explicit user approval.",
            provider.id
        ));
    }

    if adapter_kind == CatalogExecutionAdapterKind::Custom
        || permission == CatalogExecutionPermission::Custom
    {
        blockers.push(format!(
            "Provider `{}` needs a custom adapter setup contract.",
            provider.id
        ));
    }

    blockers
}

fn adapter_requires_base_url(adapter_kind: CatalogExecutionAdapterKind) -> bool {
    matches!(
        adapter_kind,
        CatalogExecutionAdapterKind::OllamaCompatibleHttp
            | CatalogExecutionAdapterKind::OpenAiCompatibleHttp
            | CatalogExecutionAdapterKind::OpenRouterHttp
            | CatalogExecutionAdapterKind::LiteLlmProxy
            | CatalogExecutionAdapterKind::AnthropicHttp
            | CatalogExecutionAdapterKind::GoogleAiHttp
    )
}
