use super::{
    ParsedModelCatalog,
    helpers::{
        direct_model_id, lite_llm_provider_id, mark_provider_model_capabilities,
        model_capabilities, pricing_from_value, provider_base, provider_kind_from_id,
        recommended_roles, slug, string_at_paths, title_case, truncate, u32_at_paths,
        upsert_provider,
    },
    skip,
};
use crate::{
    CatalogSourceKind, ExternalModelInput, ExternalProviderInput, ProviderAuthKind, ProviderKind,
};
use serde_json::Value;

pub(crate) fn parse_model_catalog_value(
    value: &Value,
    source_kind: CatalogSourceKind,
    max_models: usize,
) -> ParsedModelCatalog {
    match source_kind {
        CatalogSourceKind::ModelsDev => parse_models_dev_catalog(value, max_models),
        CatalogSourceKind::OpenRouter => parse_openrouter_catalog(value, max_models),
        CatalogSourceKind::LiteLlmAliases => parse_lite_llm_catalog(value, max_models),
        _ => ParsedModelCatalog::default(),
    }
}

fn parse_models_dev_catalog(value: &Value, max_models: usize) -> ParsedModelCatalog {
    let mut parsed = ParsedModelCatalog::default();
    let Some(root) = value.as_object() else {
        parsed
            .skipped_entries
            .push(skip("models.dev", "root is not an object"));
        return parsed;
    };

    let provider_map = value
        .get("providers")
        .and_then(Value::as_object)
        .unwrap_or(root);

    for (provider_id, provider_value) in provider_map {
        if parsed.models.len() >= max_models {
            parsed
                .skipped_entries
                .push(skip(provider_id, "model limit reached"));
            break;
        }

        let Some(models_value) = provider_value.get("models") else {
            parsed
                .skipped_entries
                .push(skip(provider_id, "provider has no models field"));
            continue;
        };

        let provider =
            provider_from_catalog(provider_id, provider_value, CatalogSourceKind::ModelsDev);
        upsert_provider(&mut parsed.providers, provider);
        push_models_dev_models(
            provider_id,
            models_value,
            max_models,
            &mut parsed.models,
            &mut parsed.skipped_entries,
        );
    }

    mark_provider_model_capabilities(&mut parsed.providers, &parsed.models);
    parsed
}

fn push_models_dev_models(
    provider_id: &str,
    models_value: &Value,
    max_models: usize,
    models: &mut Vec<ExternalModelInput>,
    skipped_entries: &mut Vec<super::SkippedModelCatalogEntry>,
) {
    if let Some(model_map) = models_value.as_object() {
        for (model_id, model_value) in model_map {
            if models.len() >= max_models {
                skipped_entries.push(skip(provider_id, "model limit reached"));
                break;
            }
            if let Some(model) = model_from_value(
                provider_id,
                model_id,
                model_value,
                CatalogSourceKind::ModelsDev,
            ) {
                models.push(model);
            } else {
                skipped_entries.push(skip(
                    &format!("{provider_id}/{model_id}"),
                    "model entry is not an object",
                ));
            }
        }
        return;
    }

    if let Some(model_list) = models_value.as_array() {
        for model_value in model_list {
            if models.len() >= max_models {
                skipped_entries.push(skip(provider_id, "model limit reached"));
                break;
            }
            let model_id = string_at_paths(model_value, &[&["id"], &["model_id"], &["model"]])
                .unwrap_or_else(|| "model".to_string());
            if let Some(model) = model_from_value(
                provider_id,
                &model_id,
                model_value,
                CatalogSourceKind::ModelsDev,
            ) {
                models.push(model);
            } else {
                skipped_entries.push(skip(
                    &format!("{provider_id}/{model_id}"),
                    "model entry is not an object",
                ));
            }
        }
    } else {
        skipped_entries.push(skip(provider_id, "models field is not an object or array"));
    }
}

fn parse_openrouter_catalog(value: &Value, max_models: usize) -> ParsedModelCatalog {
    let mut parsed = ParsedModelCatalog::default();
    let mut provider = provider_base(
        "openrouter",
        "OpenRouter",
        ProviderKind::OpenRouter,
        ProviderAuthKind::ApiKey,
    );
    provider.base_url = Some("https://openrouter.ai/api/v1".to_string());
    provider.homepage_url = Some("https://openrouter.ai".to_string());
    provider.supports_tools = true;
    provider.supports_premium_account = true;
    provider.notes = Some("Parsed from OpenRouter /api/v1/models metadata".to_string());
    upsert_provider(&mut parsed.providers, provider);

    let Some(model_list) = value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
    else {
        parsed
            .skipped_entries
            .push(skip("openrouter", "root data field is not an array"));
        return parsed;
    };

    for model_value in model_list {
        if parsed.models.len() >= max_models {
            parsed
                .skipped_entries
                .push(skip("openrouter", "model limit reached"));
            break;
        }

        let Some(raw_id) = string_at_paths(model_value, &[&["id"], &["canonical_slug"]]) else {
            parsed
                .skipped_entries
                .push(skip("openrouter", "model has no id"));
            continue;
        };
        let Some(model) = openrouter_model_from_value(&raw_id, model_value) else {
            parsed
                .skipped_entries
                .push(skip(&raw_id, "model entry is not an object"));
            continue;
        };
        parsed.models.push(model);
    }

    mark_provider_model_capabilities(&mut parsed.providers, &parsed.models);
    parsed
}

fn parse_lite_llm_catalog(value: &Value, max_models: usize) -> ParsedModelCatalog {
    let mut parsed = ParsedModelCatalog::default();
    let Some(model_list) = value
        .get("model_list")
        .and_then(Value::as_array)
        .or_else(|| value.get("models").and_then(Value::as_array))
        .or_else(|| value.as_array())
    else {
        parsed
            .skipped_entries
            .push(skip("lite-llm", "root model list is not an array"));
        return parsed;
    };

    for model_value in model_list {
        if parsed.models.len() >= max_models {
            parsed
                .skipped_entries
                .push(skip("lite-llm", "model limit reached"));
            break;
        }

        let model_name = string_at_paths(model_value, &[&["model_name"], &["name"], &["id"]])
            .unwrap_or_else(|| "model".to_string());
        let provider_id = lite_llm_provider_id(model_value, &model_name);
        let mut provider = provider_base(
            &provider_id,
            title_case(&provider_id),
            provider_kind_from_id(&provider_id, CatalogSourceKind::LiteLlmAliases),
            ProviderAuthKind::ApiKey,
        );
        provider.base_url = string_at_paths(
            model_value,
            &[
                &["litellm_params", "api_base"],
                &["litellm_params", "api_base_url"],
                &["api_base"],
                &["base_url"],
            ],
        );
        provider.supports_tools = true;
        provider.supports_premium_account = true;
        provider.notes = Some("Parsed from LiteLLM model_list metadata".to_string());
        upsert_provider(&mut parsed.providers, provider);

        if let Some(model) = lite_llm_model_from_value(&provider_id, &model_name, model_value) {
            parsed.models.push(model);
        } else {
            parsed
                .skipped_entries
                .push(skip(&model_name, "LiteLLM model entry is not an object"));
        }
    }

    mark_provider_model_capabilities(&mut parsed.providers, &parsed.models);
    parsed
}

fn provider_from_catalog(
    provider_id: &str,
    value: &Value,
    source_kind: CatalogSourceKind,
) -> ExternalProviderInput {
    let display_name = string_at_paths(
        value,
        &[
            &["name"],
            &["display_name"],
            &["displayName"],
            &["title"],
            &["label"],
        ],
    )
    .unwrap_or_else(|| title_case(provider_id));
    let mut provider = provider_base(
        provider_id,
        display_name,
        provider_kind_from_id(provider_id, source_kind),
        ProviderAuthKind::ApiKey,
    );
    provider.base_url = string_at_paths(
        value,
        &[&["base_url"], &["baseURL"], &["api"], &["api_url"]],
    );
    provider.homepage_url = string_at_paths(value, &[&["homepage"], &["homepage_url"], &["docs"]]);
    provider.supports_premium_account = true;
    provider.notes = Some(format!("Parsed from {source_kind:?} provider metadata"));
    provider
}

fn openrouter_model_from_value(raw_id: &str, value: &Value) -> Option<ExternalModelInput> {
    let _ = value.as_object()?;
    let model_id = format!("openrouter/{raw_id}");
    let display_name = string_at_paths(value, &[&["name"], &["display_name"], &["displayName"]])
        .unwrap_or_else(|| title_case(raw_id));
    let mut model = ExternalModelInput::new(model_id, "openrouter", display_name);
    model.aliases.push(raw_id.to_string());
    if let Some(canonical_slug) = string_at_paths(value, &[&["canonical_slug"]]) {
        if !model.aliases.contains(&canonical_slug) {
            model.aliases.push(canonical_slug);
        }
    }
    model.context_window_tokens = u32_at_paths(
        value,
        &[&["context_length"], &["top_provider", "context_length"]],
    );
    model.max_output_tokens = u32_at_paths(value, &[&["top_provider", "max_completion_tokens"]]);
    model.pricing = pricing_from_value(value, CatalogSourceKind::OpenRouter);
    model.capabilities = model_capabilities(
        value,
        raw_id,
        &model.display_name,
        CatalogSourceKind::OpenRouter,
        model.pricing,
    );
    model.recommended_roles = recommended_roles(&model.capabilities);
    if model.capabilities.free_tier {
        model.free_tier_hint = Some("OpenRouter free route".to_string());
    } else {
        model.premium_account_hint = Some("OpenRouter account or credits required".to_string());
    }
    model.notes = string_at_paths(value, &[&["description"]])
        .map(|description| format!("OpenRouter description: {}", truncate(&description, 180)));
    Some(model)
}

fn model_from_value(
    provider_id: &str,
    raw_model_id: &str,
    value: &Value,
    source_kind: CatalogSourceKind,
) -> Option<ExternalModelInput> {
    let _ = value.as_object()?;
    let model_id = direct_model_id(provider_id, raw_model_id);
    let display_name = string_at_paths(
        value,
        &[
            &["name"],
            &["display_name"],
            &["displayName"],
            &["title"],
            &["label"],
        ],
    )
    .unwrap_or_else(|| title_case(raw_model_id));
    let mut model = ExternalModelInput::new(model_id, provider_id, display_name);
    model.aliases.push(raw_model_id.to_string());
    model.context_window_tokens = u32_at_paths(
        value,
        &[
            &["limit", "context"],
            &["limits", "context"],
            &["context"],
            &["context_length"],
            &["contextWindow"],
            &["context_window"],
        ],
    );
    model.max_output_tokens = u32_at_paths(
        value,
        &[
            &["limit", "output"],
            &["limits", "output"],
            &["max_output"],
            &["max_output_tokens"],
            &["max_tokens"],
        ],
    );
    model.pricing = pricing_from_value(value, source_kind);
    model.capabilities = model_capabilities(
        value,
        raw_model_id,
        &model.display_name,
        source_kind,
        model.pricing,
    );
    model.recommended_roles = recommended_roles(&model.capabilities);
    if model.capabilities.free_tier {
        model.free_tier_hint = Some("Free tier or zero-price catalog entry".to_string());
    } else {
        model.premium_account_hint = Some("Provider account may be required".to_string());
    }
    model.notes = string_at_paths(value, &[&["description"], &["notes"]])
        .map(|description| format!("Catalog note: {}", truncate(&description, 180)));
    Some(model)
}

fn lite_llm_model_from_value(
    provider_id: &str,
    model_name: &str,
    value: &Value,
) -> Option<ExternalModelInput> {
    let _ = value.as_object()?;
    let target_model = string_at_paths(
        value,
        &[
            &["litellm_params", "model"],
            &["model"],
            &["id"],
            &["model_name"],
        ],
    )
    .unwrap_or_else(|| model_name.to_string());
    let model_id = format!("lite-llm/{}", slug(model_name));
    let display_name = string_at_paths(
        value,
        &[
            &["display_name"],
            &["model_info", "display_name"],
            &["name"],
        ],
    )
    .unwrap_or_else(|| title_case(model_name));
    let mut model = ExternalModelInput::new(model_id, provider_id, display_name);
    model.aliases.push(model_name.to_string());
    if target_model != model_name {
        model.aliases.push(target_model.clone());
    }
    model.context_window_tokens = u32_at_paths(
        value,
        &[
            &["model_info", "context_window"],
            &["model_info", "max_input_tokens"],
            &["context_window"],
            &["context_length"],
        ],
    );
    model.max_output_tokens = u32_at_paths(
        value,
        &[
            &["model_info", "max_output_tokens"],
            &["max_output_tokens"],
            &["max_tokens"],
        ],
    );
    model.pricing = pricing_from_value(value, CatalogSourceKind::LiteLlmAliases);
    model.capabilities = model_capabilities(
        value,
        &target_model,
        &model.display_name,
        CatalogSourceKind::LiteLlmAliases,
        model.pricing,
    );
    model.recommended_roles = recommended_roles(&model.capabilities);
    model.notes = Some(format!("LiteLLM route target: {target_model}"));
    Some(model)
}
