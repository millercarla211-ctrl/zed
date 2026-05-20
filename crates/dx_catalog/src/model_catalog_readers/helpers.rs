use crate::{
    CatalogSourceKind, ExternalModelInput, ExternalProviderInput, ModelCapabilities,
    ModelPricingMicros, ProviderAuthKind, ProviderKind, RoutingRole,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn provider_base(
    provider_id: impl Into<String>,
    display_name: impl Into<String>,
    kind: ProviderKind,
    auth: ProviderAuthKind,
) -> ExternalProviderInput {
    let mut provider = ExternalProviderInput::new(provider_id, display_name, kind);
    provider.auth = auth;
    provider.supports_streaming = true;
    provider.is_enabled_by_default = true;
    provider
}

pub(super) fn provider_kind_from_id(
    provider_id: &str,
    source_kind: CatalogSourceKind,
) -> ProviderKind {
    if source_kind == CatalogSourceKind::OpenRouter {
        return ProviderKind::OpenRouter;
    }
    let lower = provider_id.to_ascii_lowercase();
    if lower.contains("anthropic") {
        ProviderKind::Anthropic
    } else if lower.contains("google") || lower.contains("gemini") {
        ProviderKind::GoogleAi
    } else if lower.contains("bedrock") || lower.contains("amazon") || lower.contains("aws") {
        ProviderKind::Bedrock
    } else if lower.contains("openrouter") {
        ProviderKind::OpenRouter
    } else if contains_any(
        &lower,
        &[
            "openai",
            "xai",
            "grok",
            "groq",
            "deepseek",
            "mistral",
            "cohere",
            "cerebras",
            "together",
            "perplexity",
            "sambanova",
            "fireworks",
            "qwen",
            "alibaba",
            "meta",
            "llama",
        ],
    ) {
        ProviderKind::OpenAiCompatible
    } else if source_kind == CatalogSourceKind::ModelsDev {
        ProviderKind::ModelsDev
    } else {
        ProviderKind::OpenAiCompatible
    }
}

pub(super) fn model_capabilities(
    value: &Value,
    raw_id: &str,
    display_name: &str,
    source_kind: CatalogSourceKind,
    pricing: Option<ModelPricingMicros>,
) -> ModelCapabilities {
    let haystack = format!(
        "{} {} {}",
        raw_id.to_ascii_lowercase(),
        display_name.to_ascii_lowercase(),
        string_at_paths(value, &[&["description"], &["notes"]])
            .unwrap_or_default()
            .to_ascii_lowercase()
    );
    let parameters = string_set_at_paths(value, &[&["supported_parameters"], &["parameters"]]);
    let modalities = string_set_at_paths(
        value,
        &[
            &["input_modalities"],
            &["output_modalities"],
            &["architecture", "input_modalities"],
            &["architecture", "output_modalities"],
            &["modalities"],
        ],
    );
    let free = has_free_hint(raw_id, display_name, pricing);

    ModelCapabilities {
        chat: !haystack.contains("embedding"),
        tools: bool_at_paths(
            value,
            &[
                &["tool_call"],
                &["tool_calls"],
                &["tools"],
                &["supports_tools"],
                &["capabilities", "tools"],
            ],
        )
        .unwrap_or_else(|| {
            parameters.contains("tools")
                || parameters.contains("tool_choice")
                || parameters.contains("functions")
        }),
        vision: modalities.contains("image")
            || modalities.contains("images")
            || bool_at_paths(value, &[&["vision"], &["capabilities", "vision"]]).unwrap_or(false),
        audio: modalities.contains("audio")
            || bool_at_paths(value, &[&["audio"], &["capabilities", "audio"]]).unwrap_or(false),
        video: modalities.contains("video")
            || bool_at_paths(value, &[&["video"], &["capabilities", "video"]]).unwrap_or(false),
        embeddings: haystack.contains("embedding")
            || modalities.contains("embedding")
            || modalities.contains("embeddings"),
        coding: contains_any(&haystack, &["code", "coder", "coding", "devstral"]),
        reasoning: bool_at_paths(value, &[&["reasoning"], &["capabilities", "reasoning"]])
            .unwrap_or_else(|| {
                parameters.contains("reasoning")
                    || contains_any(
                        &haystack,
                        &["reasoning", "thinking", "o1", "o3", "o4", "r1"],
                    )
            }),
        local_runtime: false,
        streaming: source_kind != CatalogSourceKind::ModelsDev
            || bool_at_paths(value, &[&["streaming"], &["capabilities", "streaming"]])
                .unwrap_or(true),
        free_tier: free,
        premium_account: !free,
    }
}

pub(super) fn pricing_from_value(
    value: &Value,
    source_kind: CatalogSourceKind,
) -> Option<ModelPricingMicros> {
    let input = number_at_paths(
        value,
        &[
            &["cost", "input"],
            &["pricing", "input"],
            &["pricing", "prompt"],
            &["input_cost"],
            &["input_per_million"],
        ],
    );
    let output = number_at_paths(
        value,
        &[
            &["cost", "output"],
            &["pricing", "output"],
            &["pricing", "completion"],
            &["output_cost"],
            &["output_per_million"],
        ],
    );
    if input.is_none() && output.is_none() {
        return None;
    }

    let convert: fn(f64) -> u64 = if source_kind == CatalogSourceKind::OpenRouter {
        price_per_token_to_micros_per_million
    } else {
        price_per_million_to_micros
    };

    Some(ModelPricingMicros {
        input_per_million_tokens: input.map(convert),
        output_per_million_tokens: output.map(convert),
    })
}

pub(super) fn recommended_roles(capabilities: &ModelCapabilities) -> Vec<RoutingRole> {
    let mut roles = Vec::new();
    if capabilities.tools {
        push_role(&mut roles, RoutingRole::ToolAgent);
    }
    if capabilities.coding {
        push_role(&mut roles, RoutingRole::Coding);
    }
    if capabilities.reasoning {
        push_role(&mut roles, RoutingRole::Reasoning);
    }
    if capabilities.vision {
        push_role(&mut roles, RoutingRole::Vision);
    }
    if capabilities.audio {
        push_role(&mut roles, RoutingRole::Audio);
    }
    if capabilities.embeddings {
        push_role(&mut roles, RoutingRole::Embeddings);
    }
    if roles.is_empty() {
        roles.push(RoutingRole::Helper);
    }
    roles
}

pub(super) fn mark_provider_model_capabilities(
    providers: &mut BTreeMap<String, ExternalProviderInput>,
    models: &[ExternalModelInput],
) {
    for model in models {
        if let Some(provider) = providers.get_mut(&model.provider_id) {
            provider.supports_tools |= model.capabilities.tools;
            provider.supports_free_tier |= model.capabilities.free_tier;
            provider.supports_premium_account |= model.capabilities.premium_account;
        }
    }
}

pub(super) fn upsert_provider(
    providers: &mut BTreeMap<String, ExternalProviderInput>,
    provider: ExternalProviderInput,
) {
    providers
        .entry(provider.id.clone())
        .and_modify(|existing| {
            existing.supports_tools |= provider.supports_tools;
            existing.supports_streaming |= provider.supports_streaming;
            existing.supports_free_tier |= provider.supports_free_tier;
            existing.supports_premium_account |= provider.supports_premium_account;
            if existing.base_url.is_none() {
                existing.base_url = provider.base_url.clone();
            }
            if existing.homepage_url.is_none() {
                existing.homepage_url = provider.homepage_url.clone();
            }
            if existing.notes.is_none() {
                existing.notes = provider.notes.clone();
            }
        })
        .or_insert(provider);
}

pub(super) fn lite_llm_provider_id(value: &Value, model_name: &str) -> String {
    string_at_paths(
        value,
        &[
            &["litellm_params", "custom_llm_provider"],
            &["custom_llm_provider"],
            &["provider"],
            &["model_info", "provider"],
        ],
    )
    .map(|provider| slug(&provider))
    .unwrap_or_else(|| {
        let target_model = string_at_paths(value, &[&["litellm_params", "model"], &["model"]])
            .unwrap_or_else(|| model_name.to_string());
        let provider_prefix = target_model.split_once('/').map(|(provider, _)| provider);
        provider_prefix
            .map(slug)
            .unwrap_or_else(|| "lite-llm".to_string())
    })
}

pub(super) fn direct_model_id(provider_id: &str, raw_model_id: &str) -> String {
    if raw_model_id.contains('/') {
        raw_model_id.to_string()
    } else {
        format!("{provider_id}/{raw_model_id}")
    }
}

pub(super) fn string_at_paths(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| value_at_path(value, path))
        .and_then(string_from_value)
}

pub(super) fn u32_at_paths(value: &Value, paths: &[&[&str]]) -> Option<u32> {
    number_at_paths(value, paths)
        .and_then(|value| (value >= 0.0 && value <= u32::MAX as f64).then(|| value.round() as u32))
}

pub(super) fn title_case(value: impl AsRef<str>) -> String {
    let value = value.as_ref();
    value
        .split(['-', '_', '/', '.'])
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

pub(super) fn slug(value: &str) -> String {
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

pub(super) fn truncate(value: &str, max_chars: usize) -> String {
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        truncated.push_str("...");
    }
    truncated
}

fn price_per_million_to_micros(value: f64) -> u64 {
    (value.max(0.0) * 1_000_000.0).round() as u64
}

fn price_per_token_to_micros_per_million(value: f64) -> u64 {
    (value.max(0.0) * 1_000_000_000_000.0).round() as u64
}

fn has_free_hint(raw_id: &str, display_name: &str, pricing: Option<ModelPricingMicros>) -> bool {
    let name = format!(
        "{} {}",
        raw_id.to_ascii_lowercase(),
        display_name.to_ascii_lowercase()
    );
    if name.contains(":free") || name.contains(" free") || name.ends_with("-free") {
        return true;
    }
    pricing.is_some_and(|pricing| {
        matches!(pricing.input_per_million_tokens, Some(0))
            && matches!(pricing.output_per_million_tokens, Some(0))
    })
}

fn push_role(roles: &mut Vec<RoutingRole>, role: RoutingRole) {
    if !roles.contains(&role) {
        roles.push(role);
    }
}

fn string_from_value(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn bool_at_paths(value: &Value, paths: &[&[&str]]) -> Option<bool> {
    paths
        .iter()
        .find_map(|path| value_at_path(value, path))
        .and_then(bool_from_value)
}

fn bool_from_value(value: &Value) -> Option<bool> {
    value
        .as_bool()
        .or_else(|| match value.as_str()?.to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" => Some(true),
            "false" | "no" | "0" => Some(false),
            _ => None,
        })
}

fn number_at_paths(value: &Value, paths: &[&[&str]]) -> Option<f64> {
    paths
        .iter()
        .find_map(|path| value_at_path(value, path))
        .and_then(number_from_value)
}

fn number_from_value(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.replace(',', "").parse::<f64>().ok())
}

fn string_set_at_paths(value: &Value, paths: &[&[&str]]) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    for path in paths {
        if let Some(value) = value_at_path(value, path) {
            collect_string_set(value, &mut values);
        }
    }
    values
}

fn collect_string_set(value: &Value, values: &mut BTreeSet<String>) {
    if let Some(value) = value.as_str() {
        values.insert(value.to_ascii_lowercase());
    } else if let Some(array) = value.as_array() {
        for item in array {
            collect_string_set(item, values);
        }
    }
}

fn value_at_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}
