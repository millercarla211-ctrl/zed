use crate::{
    DxCatalog, ModelCapabilities, ModelRecord, ProviderAuthKind, ProviderRecord, RoutingRole,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentPickerProjectionOptions {
    pub max_models_per_group: usize,
    pub include_provider_groups: bool,
    pub include_unselectable_models: bool,
    pub recommended_roles: Vec<RoutingRole>,
}

impl AgentPickerProjectionOptions {
    pub fn new() -> Self {
        Self {
            max_models_per_group: 32,
            include_provider_groups: true,
            include_unselectable_models: true,
            recommended_roles: vec![
                RoutingRole::Helper,
                RoutingRole::ToolAgent,
                RoutingRole::Coding,
                RoutingRole::Reasoning,
                RoutingRole::Vision,
                RoutingRole::Audio,
                RoutingRole::Embeddings,
            ],
        }
    }

    pub fn with_max_models_per_group(mut self, max_models_per_group: usize) -> Self {
        self.max_models_per_group = max_models_per_group.max(1);
        self
    }

    pub fn include_provider_groups(mut self, include_provider_groups: bool) -> Self {
        self.include_provider_groups = include_provider_groups;
        self
    }

    pub fn include_unselectable_models(mut self, include_unselectable_models: bool) -> Self {
        self.include_unselectable_models = include_unselectable_models;
        self
    }

    pub fn with_recommended_roles(mut self, recommended_roles: Vec<RoutingRole>) -> Self {
        self.recommended_roles = recommended_roles;
        self
    }
}

impl Default for AgentPickerProjectionOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentRoutePreferences {
    pub role: RoutingRole,
    pub prefer_local: bool,
    pub prefer_free_tier: bool,
    pub require_selectable: bool,
    pub allow_premium: bool,
    pub max_fallbacks: usize,
}

impl AgentRoutePreferences {
    pub fn new(role: RoutingRole) -> Self {
        Self {
            role,
            prefer_local: matches!(role, RoutingRole::Helper | RoutingRole::Coding),
            prefer_free_tier: true,
            require_selectable: true,
            allow_premium: true,
            max_fallbacks: 4,
        }
    }

    pub fn prefer_local(mut self, prefer_local: bool) -> Self {
        self.prefer_local = prefer_local;
        self
    }

    pub fn prefer_free_tier(mut self, prefer_free_tier: bool) -> Self {
        self.prefer_free_tier = prefer_free_tier;
        self
    }

    pub fn require_selectable(mut self, require_selectable: bool) -> Self {
        self.require_selectable = require_selectable;
        self
    }

    pub fn allow_premium(mut self, allow_premium: bool) -> Self {
        self.allow_premium = allow_premium;
        self
    }

    pub fn with_max_fallbacks(mut self, max_fallbacks: usize) -> Self {
        self.max_fallbacks = max_fallbacks;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentPickerProjection {
    pub summary: AgentPickerProjectionSummary,
    pub groups: Vec<AgentPickerGroup>,
    pub route_recommendations: Vec<CatalogRouteSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentPickerProjectionSummary {
    pub provider_count: u32,
    pub model_count: u32,
    pub selectable_model_count: u32,
    pub local_model_count: u32,
    pub free_model_count: u32,
    pub needs_auth_model_count: u32,
    pub route_recommendation_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentPickerGroup {
    pub id: String,
    pub title: String,
    pub models: Vec<AgentPickerModel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentPickerModel {
    pub model_id: String,
    pub provider_id: String,
    pub display_name: String,
    pub provider_name: String,
    pub description: Option<String>,
    pub badges: Vec<String>,
    pub auth_state: AgentPickerAuthState,
    pub cost_label: Option<String>,
    pub recommended_roles: Vec<RoutingRole>,
    pub selectable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPickerAuthState {
    Local,
    NoAuthRequired,
    Configured,
    NeedsAuth,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogRouteSelection {
    pub role: RoutingRole,
    pub primary_model_id: String,
    pub fallback_model_ids: Vec<String>,
    pub reason: String,
    pub auth_state: AgentPickerAuthState,
    pub local: bool,
    pub free_tier: bool,
}

pub fn build_agent_picker_projection(
    catalog: &DxCatalog,
    options: AgentPickerProjectionOptions,
) -> AgentPickerProjection {
    let providers = provider_index(catalog);
    let picker_models = catalog
        .models
        .iter()
        .filter_map(|model| picker_model(model, &providers))
        .filter(|model| options.include_unselectable_models || model.selectable)
        .collect::<Vec<_>>();

    let route_recommendations = options
        .recommended_roles
        .iter()
        .filter_map(|role| {
            select_catalog_route(
                catalog,
                AgentRoutePreferences::new(*role)
                    .require_selectable(!options.include_unselectable_models),
            )
        })
        .collect::<Vec<_>>();

    let mut groups = Vec::new();
    push_group(
        &mut groups,
        "recommended",
        "Recommended",
        route_recommendations
            .iter()
            .filter_map(|route| {
                picker_models
                    .iter()
                    .find(|model| model.model_id == route.primary_model_id)
            })
            .cloned()
            .collect(),
        options.max_models_per_group,
    );
    push_group(
        &mut groups,
        "local",
        "Local",
        picker_models
            .iter()
            .filter(|model| model.auth_state == AgentPickerAuthState::Local)
            .cloned()
            .collect(),
        options.max_models_per_group,
    );
    push_group(
        &mut groups,
        "free",
        "Free",
        picker_models
            .iter()
            .filter(|model| model.badges.iter().any(|badge| badge == "Free"))
            .cloned()
            .collect(),
        options.max_models_per_group,
    );

    if options.include_provider_groups {
        for provider in catalog.providers.iter() {
            push_group(
                &mut groups,
                format!("provider:{}", provider.id),
                provider.display_name.clone(),
                picker_models
                    .iter()
                    .filter(|model| model.provider_id == provider.id)
                    .cloned()
                    .collect(),
                options.max_models_per_group,
            );
        }
    }

    AgentPickerProjection {
        summary: AgentPickerProjectionSummary {
            provider_count: catalog.providers.len() as u32,
            model_count: picker_models.len() as u32,
            selectable_model_count: picker_models
                .iter()
                .filter(|model| model.selectable)
                .count() as u32,
            local_model_count: picker_models
                .iter()
                .filter(|model| model.auth_state == AgentPickerAuthState::Local)
                .count() as u32,
            free_model_count: picker_models
                .iter()
                .filter(|model| model.badges.iter().any(|badge| badge == "Free"))
                .count() as u32,
            needs_auth_model_count: picker_models
                .iter()
                .filter(|model| model.auth_state == AgentPickerAuthState::NeedsAuth)
                .count() as u32,
            route_recommendation_count: route_recommendations.len() as u32,
        },
        groups,
        route_recommendations,
    }
}

pub fn select_catalog_route(
    catalog: &DxCatalog,
    preferences: AgentRoutePreferences,
) -> Option<CatalogRouteSelection> {
    let providers = provider_index(catalog);

    if let Some(route) = catalog
        .routing_rules
        .iter()
        .find(|rule| rule.role == preferences.role)
    {
        if let Some(model) = catalog.model(&route.primary_model_id)
            && let Some(provider) = providers.get(&model.provider_id).copied()
            && model_allowed(model, provider, &preferences)
        {
            let fallback_model_ids = route
                .fallback_model_ids
                .iter()
                .filter_map(|model_id| {
                    let model = catalog.model(model_id)?;
                    let provider = providers.get(&model.provider_id).copied()?;
                    model_allowed(model, provider, &preferences).then(|| model_id.clone())
                })
                .take(preferences.max_fallbacks)
                .collect::<Vec<_>>();
            return Some(route_selection(
                preferences.role,
                model,
                provider,
                fallback_model_ids,
                "Explicit catalog routing rule".to_string(),
            ));
        }
    }

    let mut candidates = catalog
        .models
        .iter()
        .filter_map(|model| {
            let provider = providers.get(&model.provider_id).copied()?;
            model_allowed(model, provider, &preferences).then(|| {
                (
                    route_score(model, provider, &preferences),
                    model.id.clone(),
                    model,
                    provider,
                )
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));

    let (_, _, primary, provider) = candidates.first()?;
    let fallback_model_ids = candidates
        .iter()
        .skip(1)
        .map(|(_, _, model, _)| model.id.clone())
        .take(preferences.max_fallbacks)
        .collect::<Vec<_>>();

    Some(route_selection(
        preferences.role,
        *primary,
        *provider,
        fallback_model_ids,
        "Best scored catalog route".to_string(),
    ))
}

fn provider_index(catalog: &DxCatalog) -> BTreeMap<String, &ProviderRecord> {
    catalog
        .providers
        .iter()
        .map(|provider| (provider.id.clone(), provider))
        .collect()
}

fn picker_model(
    model: &ModelRecord,
    providers: &BTreeMap<String, &ProviderRecord>,
) -> Option<AgentPickerModel> {
    let provider = providers.get(&model.provider_id).copied()?;
    let auth_state = auth_state(provider);
    let mut badges = capability_badges(&model.capabilities);
    if model.local_runtime.is_some() || provider.is_local {
        badges.push("Local".to_string());
    }
    if model.capabilities.free_tier || provider.supports_free_tier {
        badges.push("Free".to_string());
    }
    if model.capabilities.premium_account || provider.supports_premium_account {
        badges.push("Premium".to_string());
    }
    badges.sort();
    badges.dedup();

    Some(AgentPickerModel {
        model_id: model.id.clone(),
        provider_id: provider.id.clone(),
        display_name: model.display_name.clone(),
        provider_name: provider.display_name.clone(),
        description: model.notes.clone().or_else(|| provider.notes.clone()),
        badges,
        auth_state,
        cost_label: cost_label(model),
        recommended_roles: model.recommended_roles.clone(),
        selectable: auth_state != AgentPickerAuthState::NeedsAuth,
    })
}

fn push_group(
    groups: &mut Vec<AgentPickerGroup>,
    id: impl Into<String>,
    title: impl Into<String>,
    models: Vec<AgentPickerModel>,
    max_models: usize,
) {
    let mut seen = BTreeSet::new();
    let mut models = models
        .into_iter()
        .filter(|model| seen.insert(model.model_id.clone()))
        .collect::<Vec<_>>();
    models.sort_by(|left, right| {
        left.provider_name
            .cmp(&right.provider_name)
            .then_with(|| left.display_name.cmp(&right.display_name))
            .then_with(|| left.model_id.cmp(&right.model_id))
    });
    models.truncate(max_models);
    if !models.is_empty() {
        groups.push(AgentPickerGroup {
            id: id.into(),
            title: title.into(),
            models,
        });
    }
}

fn model_allowed(
    model: &ModelRecord,
    provider: &ProviderRecord,
    preferences: &AgentRoutePreferences,
) -> bool {
    let auth_state = auth_state(provider);
    if preferences.require_selectable && auth_state == AgentPickerAuthState::NeedsAuth {
        return false;
    }
    if !preferences.allow_premium
        && (model.capabilities.premium_account || provider.supports_premium_account)
        && !model.capabilities.free_tier
        && !provider.supports_free_tier
        && model.local_runtime.is_none()
        && !provider.is_local
    {
        return false;
    }
    role_supported(model, preferences.role)
}

fn route_score(
    model: &ModelRecord,
    provider: &ProviderRecord,
    preferences: &AgentRoutePreferences,
) -> i32 {
    let mut score = 0;
    if model.recommended_roles.contains(&preferences.role) {
        score += 100;
    }
    score += capability_score(&model.capabilities, preferences.role);
    if preferences.prefer_local && (model.local_runtime.is_some() || provider.is_local) {
        score += 30;
    }
    if preferences.prefer_free_tier && (model.capabilities.free_tier || provider.supports_free_tier)
    {
        score += 20;
    }
    if auth_state(provider) != AgentPickerAuthState::NeedsAuth {
        score += 10;
    }
    if model.is_latest_hint() {
        score += 2;
    }
    score
}

fn route_selection(
    role: RoutingRole,
    model: &ModelRecord,
    provider: &ProviderRecord,
    fallback_model_ids: Vec<String>,
    reason: String,
) -> CatalogRouteSelection {
    CatalogRouteSelection {
        role,
        primary_model_id: model.id.clone(),
        fallback_model_ids,
        reason,
        auth_state: auth_state(provider),
        local: model.local_runtime.is_some() || provider.is_local,
        free_tier: model.capabilities.free_tier || provider.supports_free_tier,
    }
}

fn auth_state(provider: &ProviderRecord) -> AgentPickerAuthState {
    if provider.is_local || matches!(provider.auth, ProviderAuthKind::LocalRuntime) {
        AgentPickerAuthState::Local
    } else if provider
        .auth_profile
        .as_ref()
        .is_some_and(|profile| profile.configured)
    {
        AgentPickerAuthState::Configured
    } else if matches!(provider.auth, ProviderAuthKind::None) {
        AgentPickerAuthState::NoAuthRequired
    } else if matches!(provider.auth, ProviderAuthKind::Custom) {
        AgentPickerAuthState::Unknown
    } else {
        AgentPickerAuthState::NeedsAuth
    }
}

fn role_supported(model: &ModelRecord, role: RoutingRole) -> bool {
    model.recommended_roles.contains(&role) || capability_score(&model.capabilities, role) > 0
}

fn capability_score(capabilities: &ModelCapabilities, role: RoutingRole) -> i32 {
    match role {
        RoutingRole::Helper | RoutingRole::Fallback => capabilities.chat.then_some(20),
        RoutingRole::ToolAgent => capabilities.tools.then_some(40),
        RoutingRole::Coding => capabilities.coding.then_some(40),
        RoutingRole::Reasoning => capabilities.reasoning.then_some(40),
        RoutingRole::Vision => capabilities.vision.then_some(40),
        RoutingRole::Audio => capabilities.audio.then_some(40),
        RoutingRole::Embeddings => capabilities.embeddings.then_some(40),
    }
    .unwrap_or(0)
}

fn capability_badges(capabilities: &ModelCapabilities) -> Vec<String> {
    let mut badges = Vec::new();
    if capabilities.tools {
        badges.push("Tools".to_string());
    }
    if capabilities.coding {
        badges.push("Code".to_string());
    }
    if capabilities.reasoning {
        badges.push("Reasoning".to_string());
    }
    if capabilities.vision {
        badges.push("Vision".to_string());
    }
    if capabilities.audio {
        badges.push("Audio".to_string());
    }
    if capabilities.embeddings {
        badges.push("Embeddings".to_string());
    }
    badges
}

fn cost_label(model: &ModelRecord) -> Option<String> {
    let pricing = model.pricing?;
    match (
        pricing.input_per_million_tokens,
        pricing.output_per_million_tokens,
    ) {
        (Some(0), Some(0)) => Some("Free".to_string()),
        (Some(input), Some(output)) => Some(format!(
            "{} in / {} out per 1M tokens",
            micros_to_dollars(input),
            micros_to_dollars(output)
        )),
        (Some(input), None) => Some(format!("{} input per 1M tokens", micros_to_dollars(input))),
        (None, Some(output)) => Some(format!(
            "{} output per 1M tokens",
            micros_to_dollars(output)
        )),
        (None, None) => None,
    }
}

fn micros_to_dollars(value: u64) -> String {
    let whole = value / 1_000_000;
    let cents = ((value % 1_000_000) * 100) / 1_000_000;
    if cents == 0 {
        format!("${whole}")
    } else {
        format!("${whole}.{cents:02}")
    }
}

trait ModelFreshness {
    fn is_latest_hint(&self) -> bool;
}

impl ModelFreshness for ModelRecord {
    fn is_latest_hint(&self) -> bool {
        let id = self.id.to_ascii_lowercase();
        let name = self.display_name.to_ascii_lowercase();
        id.contains("latest") || name.contains("latest")
    }
}
