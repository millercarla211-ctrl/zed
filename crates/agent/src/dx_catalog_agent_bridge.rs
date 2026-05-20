use acp_thread::AgentModelInfo;
use collections::{HashMap, HashSet};
use dx_catalog::{
    AgentPickerAuthState, AgentPickerProjectionOptions, CatalogArtifactBuildOptions,
    CatalogExecutionAdapterKind, CatalogExecutionPermission, CatalogExecutionPlan,
    CatalogExecutionPlanRequest, CatalogProviderAdapterModelSpec,
    CatalogProviderAdapterRegistrationSpec, DxCatalog, ModelRecord, ProviderRecord, RoutingRole,
    build_agent_picker_projection, build_catalog_artifact_from_sources,
    build_catalog_execution_plan, build_catalog_provider_registration_specs, read_catalog_artifact,
};
use fs::Fs;
use settings::{
    OpenAiCompatibleAvailableModel, OpenAiCompatibleModelCapabilities,
    OpenAiCompatibleSettingsContent, OpenRouterAvailableModel, update_settings_file,
};
use std::{
    env,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const DX_CATALOG_ARTIFACT_ENV: &str = "DX_CATALOG_ARTIFACT";
const DX_CATALOG_PATH_ENV: &str = "DX_CATALOG_PATH";
const DX_CATALOG_OUTPUT_ENV: &str = "DX_CATALOG_OUTPUT";
const DX_CATALOG_LAST_GOOD_ENV: &str = "DX_CATALOG_LAST_GOOD";
const DX_CATALOG_GENERATE_ENV: &str = "DX_CATALOG_GENERATE";
const DX_CATALOG_GENERATE_ON_LOAD_ENV: &str = "DX_CATALOG_GENERATE_ON_LOAD";
const DX_CATALOG_SOURCE_REVISION_ENV: &str = "DX_CATALOG_SOURCE_REVISION";
const DX_CATALOG_REGISTER_PROVIDERS_ENV: &str = "DX_CATALOG_REGISTER_PROVIDERS";
const DX_CATALOG_REGISTER_PROVIDER_SETTINGS_ENV: &str = "DX_CATALOG_REGISTER_PROVIDER_SETTINGS";
const DX_CATALOG_REGISTER_PROVIDERS_DRY_RUN_ENV: &str = "DX_CATALOG_REGISTER_PROVIDERS_DRY_RUN";
const DX_CATALOG_ARTIFACT_FILE_NAME: &str = "catalog.dxcat";
const DEFAULT_CATALOG_MODEL_MAX_TOKENS: u64 = 200_000;
pub(crate) const DX_CATALOG_PROVIDER_SETTINGS_PREVIEW_SCHEMA: &str =
    "zed.dx_catalog.provider_settings.registration_preview.v1";

#[derive(Clone, Debug, Default)]
pub struct DxCatalogAgentBridge {
    models: HashMap<String, CatalogModelPresentation>,
    route_candidates: HashMap<String, Vec<String>>,
    execution_plans: HashMap<String, CatalogExecutionSummary>,
}

#[derive(Clone, Debug)]
struct CatalogModelPresentation {
    description: Option<String>,
    cost_label: Option<String>,
}

#[derive(Clone, Debug)]
struct CatalogExecutionSummary {
    primary_model_id: String,
    provider_name: String,
    adapter_kind: CatalogExecutionAdapterKind,
    permission: CatalogExecutionPermission,
    blockers: Vec<String>,
    next_action: String,
}

impl DxCatalogAgentBridge {
    pub fn load_from_environment() -> Option<Self> {
        static BRIDGE: OnceLock<Option<DxCatalogAgentBridge>> = OnceLock::new();

        BRIDGE.get_or_init(Self::load_uncached).clone()
    }

    pub fn enrich_model_info(&self, model_info: &mut AgentModelInfo) {
        let Some(presentation) = self.models.get(model_info.id.0.as_ref()) else {
            return;
        };

        if model_info.description.is_none() {
            if let Some(description) = &presentation.description {
                model_info.description = Some(description.clone().into());
            }
        }

        if model_info.cost.is_none() {
            if let Some(cost_label) = &presentation.cost_label {
                model_info.cost = Some(cost_label.clone().into());
            }
        }
    }

    pub fn resolve_model_id<'a>(
        &self,
        requested_model_id: &str,
        executable_model_ids: impl IntoIterator<Item = &'a str>,
    ) -> Option<String> {
        let executable_model_ids = executable_model_ids.into_iter().collect::<HashSet<_>>();
        if executable_model_ids.contains(requested_model_id) {
            return Some(requested_model_id.to_string());
        }

        self.route_candidates
            .get(requested_model_id)?
            .iter()
            .find(|candidate| executable_model_ids.contains(candidate.as_str()))
            .cloned()
    }

    pub fn catalog_selection_error(&self, requested_model_id: &str) -> Option<String> {
        let summary = self.execution_plans.get(requested_model_id)?;
        let blocker = if summary.blockers.is_empty() {
            format!(
                "The {} adapter still needs to be registered before this catalog-only model can execute.",
                summary.adapter_kind.label()
            )
        } else {
            summary.blockers.join(" ")
        };

        Some(format!(
            "DX catalog route `{requested_model_id}` points to `{}` on `{}` but no registered executable provider owns it yet. Required adapter: {} with {}. Next action: {} Blocker: {blocker}",
            summary.primary_model_id,
            summary.provider_name,
            summary.adapter_kind.label(),
            summary.permission.label(),
            summary.next_action
        ))
    }

    fn load_uncached() -> Option<Self> {
        materialize_catalog_artifact_if_approved();

        let candidates = catalog_artifact_candidates();
        for candidate in candidates {
            if !candidate.path.is_file() {
                if let Some(env_var) = candidate.env_var {
                    log::warn!(
                        "DX catalog artifact path from {env_var} does not exist or is not a file: {}",
                        candidate.path.display()
                    );
                }
                continue;
            }

            match read_catalog_artifact(&candidate.path) {
                Ok(catalog) => return Some(Self::from_catalog(&catalog)),
                Err(error) => {
                    log::warn!(
                        "failed to read DX catalog artifact for Agent model picker enrichment at {}: {error}",
                        candidate.path.display()
                    );
                }
            }
        }

        None
    }

    fn from_catalog(catalog: &DxCatalog) -> Self {
        let providers = catalog
            .providers
            .iter()
            .map(|provider| (provider.id.as_str(), provider))
            .collect::<HashMap<_, _>>();
        let projection = build_agent_picker_projection(
            catalog,
            AgentPickerProjectionOptions::new()
                .include_provider_groups(false)
                .include_unselectable_models(true),
        );
        let route_recommendations = projection.route_recommendations.clone();
        let picker_models = projection
            .groups
            .into_iter()
            .flat_map(|group| group.models)
            .map(|model| (model.model_id.clone(), model))
            .collect::<HashMap<_, _>>();

        let mut models = HashMap::new();
        let mut route_candidates = HashMap::new();
        let mut model_lookup_keys = HashMap::new();
        let mut execution_plans = HashMap::new();
        for model in &catalog.models {
            let Some(provider) = providers.get(model.provider_id.as_str()).copied() else {
                continue;
            };
            let lookup_keys = model_lookup_keys_for_record(model, provider);
            let execution_summary = build_catalog_execution_plan(
                catalog,
                CatalogExecutionPlanRequest::for_model_id(model.id.clone()),
            )
            .map(CatalogExecutionSummary::from);

            let presentation = picker_models
                .get(&model.id)
                .map(|picker_model| CatalogModelPresentation {
                    description: catalog_model_description(
                        picker_model.description.as_deref(),
                        &picker_model.badges,
                        picker_model.auth_state,
                    )
                    .or_else(|| fallback_model_description(model, provider)),
                    cost_label: picker_model.cost_label.clone(),
                })
                .unwrap_or_else(|| CatalogModelPresentation {
                    description: fallback_model_description(model, provider),
                    cost_label: None,
                });

            for key in &lookup_keys {
                models.entry(key).or_insert_with(|| presentation.clone());
            }
            insert_route_candidates(
                &mut route_candidates,
                lookup_keys.clone(),
                lookup_keys.clone(),
            );
            insert_execution_summaries(
                &mut execution_plans,
                lookup_keys.clone(),
                execution_summary.clone(),
            );
            model_lookup_keys.insert(model.id.clone(), lookup_keys);
        }

        for route in route_recommendations {
            let mut candidates = Vec::new();
            push_catalog_model_candidates(
                &mut candidates,
                &model_lookup_keys,
                route.primary_model_id.as_str(),
            );
            for fallback_model_id in &route.fallback_model_ids {
                push_catalog_model_candidates(
                    &mut candidates,
                    &model_lookup_keys,
                    fallback_model_id.as_str(),
                );
            }

            insert_route_candidates(
                &mut route_candidates,
                catalog_role_route_ids(route.role),
                candidates,
            );
            let execution_summary = build_catalog_execution_plan(
                catalog,
                CatalogExecutionPlanRequest::for_role(route.role),
            )
            .map(CatalogExecutionSummary::from);
            insert_execution_summaries(
                &mut execution_plans,
                catalog_role_route_ids(route.role),
                execution_summary,
            );
        }

        Self {
            models,
            route_candidates,
            execution_plans,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct CatalogProviderSettingsRegistrationReport {
    eligible_provider_count: usize,
    skipped_provider_count: usize,
    openai_compatible_provider_count: usize,
    open_router_model_count: usize,
    model_count: usize,
}

impl CatalogProviderSettingsRegistrationReport {
    fn from_specs(specs: &[CatalogProviderAdapterRegistrationSpec]) -> Self {
        let mut report = Self::default();

        for spec in specs {
            if !can_write_provider_settings(spec) {
                report.skipped_provider_count += 1;
                continue;
            }

            report.eligible_provider_count += 1;
            report.model_count += spec.models.len();
            match spec.adapter_kind {
                CatalogExecutionAdapterKind::OpenRouterHttp => {
                    report.open_router_model_count += spec.models.len();
                }
                CatalogExecutionAdapterKind::OpenAiCompatibleHttp
                | CatalogExecutionAdapterKind::OllamaCompatibleHttp
                | CatalogExecutionAdapterKind::LiteLlmProxy => {
                    report.openai_compatible_provider_count += 1;
                }
                _ => {}
            }
        }

        report
    }
}

pub fn apply_provider_settings_if_approved(fs: Arc<dyn Fs>, cx: &gpui::App) {
    if !provider_settings_registration_approved() {
        return;
    }

    materialize_catalog_artifact_if_approved();

    let Some(catalog) = read_first_available_catalog_artifact("provider settings registration")
    else {
        return;
    };
    let all_specs = build_catalog_provider_registration_specs(&catalog);
    let report = CatalogProviderSettingsRegistrationReport::from_specs(&all_specs);
    let specs = all_specs
        .into_iter()
        .filter(can_write_provider_settings)
        .collect::<Vec<_>>();

    if report.eligible_provider_count == 0 {
        log::warn!(
            "DX catalog provider settings registration was approved, but no catalog providers were eligible for settings registration"
        );
        return;
    }

    if env_flag_enabled(DX_CATALOG_REGISTER_PROVIDERS_DRY_RUN_ENV) {
        log::info!(
            "DX catalog provider settings registration dry run: providers={}, skipped={}, openai_compatible={}, openrouter_models={}, models={}",
            report.eligible_provider_count,
            report.skipped_provider_count,
            report.openai_compatible_provider_count,
            report.open_router_model_count,
            report.model_count,
        );
        return;
    }

    update_settings_file(fs, cx, move |settings, _| {
        for spec in &specs {
            apply_provider_settings_spec(settings, spec);
        }
    });

    log::info!(
        "DX catalog provider settings registration queued: providers={}, skipped={}, openai_compatible={}, openrouter_models={}, models={}",
        report.eligible_provider_count,
        report.skipped_provider_count,
        report.openai_compatible_provider_count,
        report.open_router_model_count,
        report.model_count,
    );
}

pub(crate) fn provider_settings_registration_preview() -> serde_json::Value {
    let approval_enabled = provider_settings_registration_approved();
    let dry_run_enabled = env_flag_enabled(DX_CATALOG_REGISTER_PROVIDERS_DRY_RUN_ENV);
    let generation_enabled = catalog_generation_approved();
    let artifact_candidates = catalog_artifact_candidate_preview();
    let Some(artifact) =
        read_first_available_catalog_artifact_with_path("provider settings preview")
    else {
        return serde_json::json!({
            "schema": DX_CATALOG_PROVIDER_SETTINGS_PREVIEW_SCHEMA,
            "generated_at_ms": current_unix_ms(),
            "artifact_loaded": false,
            "artifact_path": serde_json::Value::Null,
            "artifact_candidates": artifact_candidates,
            "approval_enabled": approval_enabled,
            "dry_run_enabled": dry_run_enabled,
            "generation_enabled": generation_enabled,
            "summary": {
                "catalog_provider_count": 0,
                "catalog_model_count": 0,
                "registration_spec_count": 0,
                "eligible_provider_count": 0,
                "skipped_provider_count": 0,
                "openai_compatible_provider_count": 0,
                "open_router_model_count": 0,
                "model_count": 0,
                "ready_for_execution_provider_count": 0,
                "requires_user_approval_provider_count": 0,
                "needs_auth_provider_count": 0,
            },
            "providers": [],
            "next_action": "Generate or point DX_CATALOG_ARTIFACT/DX_CATALOG_PATH at a validated catalog.dxcat before previewing provider registration.",
        });
    };

    let specs = build_catalog_provider_registration_specs(&artifact.catalog);
    let report = CatalogProviderSettingsRegistrationReport::from_specs(&specs);
    let ready_for_execution_provider_count = specs
        .iter()
        .filter(|spec| can_write_provider_settings(spec) && spec.ready_for_execution)
        .count();
    let requires_user_approval_provider_count = specs
        .iter()
        .filter(|spec| spec.user_approval_required)
        .count();
    let needs_auth_provider_count = specs.iter().filter(|spec| !spec.auth_configured).count();
    let providers = specs
        .iter()
        .map(provider_settings_registration_preview_provider)
        .collect::<Vec<_>>();

    serde_json::json!({
        "schema": DX_CATALOG_PROVIDER_SETTINGS_PREVIEW_SCHEMA,
        "generated_at_ms": current_unix_ms(),
        "artifact_loaded": true,
        "artifact_path": artifact.path.display().to_string(),
        "artifact_candidates": artifact_candidates,
        "approval_enabled": approval_enabled,
        "dry_run_enabled": dry_run_enabled,
        "generation_enabled": generation_enabled,
        "summary": {
            "catalog_provider_count": artifact.catalog.providers.len(),
            "catalog_model_count": artifact.catalog.models.len(),
            "registration_spec_count": specs.len(),
            "eligible_provider_count": report.eligible_provider_count,
            "skipped_provider_count": report.skipped_provider_count,
            "openai_compatible_provider_count": report.openai_compatible_provider_count,
            "open_router_model_count": report.open_router_model_count,
            "model_count": report.model_count,
            "ready_for_execution_provider_count": ready_for_execution_provider_count,
            "requires_user_approval_provider_count": requires_user_approval_provider_count,
            "needs_auth_provider_count": needs_auth_provider_count,
        },
        "providers": providers,
        "next_action": provider_settings_registration_preview_next_action(
            &report,
            approval_enabled,
            dry_run_enabled,
        ),
    })
}

impl From<CatalogExecutionPlan> for CatalogExecutionSummary {
    fn from(plan: CatalogExecutionPlan) -> Self {
        Self {
            primary_model_id: plan.primary_model_id,
            provider_name: plan.provider_name,
            adapter_kind: plan.adapter_kind,
            permission: plan.permission,
            blockers: plan.blockers,
            next_action: plan.next_action,
        }
    }
}

struct CatalogArtifactCandidate {
    path: PathBuf,
    env_var: Option<&'static str>,
}

struct CatalogArtifactLoad {
    path: PathBuf,
    catalog: DxCatalog,
}

fn catalog_artifact_candidates() -> Vec<CatalogArtifactCandidate> {
    let mut candidates = Vec::new();

    for env_var in [
        DX_CATALOG_ARTIFACT_ENV,
        DX_CATALOG_PATH_ENV,
        DX_CATALOG_OUTPUT_ENV,
    ] {
        if let Some(path) = env::var_os(env_var).map(PathBuf::from) {
            candidates.push(CatalogArtifactCandidate {
                path,
                env_var: Some(env_var),
            });
        }
    }

    for path in [
        paths::data_dir()
            .join("dx_catalog")
            .join(DX_CATALOG_ARTIFACT_FILE_NAME),
        paths::data_dir()
            .join("dx")
            .join(DX_CATALOG_ARTIFACT_FILE_NAME),
    ] {
        candidates.push(CatalogArtifactCandidate {
            path,
            env_var: None,
        });
    }

    candidates
}

fn read_first_available_catalog_artifact(log_context: &str) -> Option<DxCatalog> {
    read_first_available_catalog_artifact_with_path(log_context).map(|load| load.catalog)
}

fn read_first_available_catalog_artifact_with_path(
    log_context: &str,
) -> Option<CatalogArtifactLoad> {
    let candidates = catalog_artifact_candidates();
    for candidate in candidates {
        if !candidate.path.is_file() {
            if let Some(env_var) = candidate.env_var {
                log::warn!(
                    "DX catalog artifact path from {env_var} does not exist or is not a file for {log_context}: {}",
                    candidate.path.display()
                );
            }
            continue;
        }

        match read_catalog_artifact(&candidate.path) {
            Ok(catalog) => {
                return Some(CatalogArtifactLoad {
                    path: candidate.path,
                    catalog,
                });
            }
            Err(error) => {
                log::warn!(
                    "failed to read DX catalog artifact for {log_context} at {}: {error}",
                    candidate.path.display()
                );
            }
        }
    }

    None
}

fn catalog_artifact_candidate_preview() -> Vec<serde_json::Value> {
    catalog_artifact_candidates()
        .into_iter()
        .map(|candidate| {
            serde_json::json!({
                "path": candidate.path.display().to_string(),
                "env_var": candidate.env_var,
                "exists": candidate.path.is_file(),
            })
        })
        .collect()
}

fn provider_settings_registration_preview_provider(
    spec: &CatalogProviderAdapterRegistrationSpec,
) -> serde_json::Value {
    let settings_writable = can_write_provider_settings(spec);
    let mut registration_blockers = spec.registration_blockers.clone();
    if !settings_writable && registration_blockers.is_empty() {
        if !matches!(
            spec.adapter_kind,
            CatalogExecutionAdapterKind::OpenAiCompatibleHttp
                | CatalogExecutionAdapterKind::OllamaCompatibleHttp
                | CatalogExecutionAdapterKind::OpenRouterHttp
                | CatalogExecutionAdapterKind::LiteLlmProxy
        ) {
            registration_blockers.push(format!(
                "{} providers are not supported by the native provider-settings writer yet.",
                spec.adapter_kind.label()
            ));
        }
        if spec
            .base_url
            .as_ref()
            .map(|url| url.trim().is_empty())
            .unwrap_or(true)
        {
            registration_blockers.push("Catalog provider has no base URL.".to_string());
        }
        if spec.models.is_empty() {
            registration_blockers.push("Catalog provider has no catalog models.".to_string());
        }
    }

    let models = spec
        .models
        .iter()
        .take(12)
        .map(provider_settings_registration_preview_model)
        .collect::<Vec<_>>();

    serde_json::json!({
        "provider_id": spec.provider_id.as_str(),
        "provider_name": spec.provider_name.as_str(),
        "adapter_kind": spec.adapter_kind,
        "adapter_kind_label": spec.adapter_kind.label(),
        "permission": spec.permission,
        "permission_label": spec.permission.label(),
        "settings_path": spec.settings_path.as_deref(),
        "base_url": spec.base_url.as_deref(),
        "auth_profile_id": spec.auth_profile_id.as_deref(),
        "auth_configured": spec.auth_configured,
        "user_approval_required": spec.user_approval_required,
        "settings_writable": settings_writable,
        "can_register_settings": spec.can_register_settings,
        "ready_for_execution": spec.ready_for_execution,
        "model_count": spec.models.len(),
        "models_preview": models,
        "truncated_model_count": spec.models.len().saturating_sub(12),
        "registration_blockers": registration_blockers,
        "execution_blockers": &spec.execution_blockers,
        "next_action": spec.next_action.as_str(),
    })
}

fn provider_settings_registration_preview_model(
    model: &CatalogProviderAdapterModelSpec,
) -> serde_json::Value {
    serde_json::json!({
        "model_id": model.model_id.as_str(),
        "display_name": model.display_name.as_str(),
        "context_window_tokens": model.context_window_tokens,
        "max_output_tokens": model.max_output_tokens,
        "supports_tools": model.supports_tools,
        "supports_images": model.supports_images,
        "supports_audio": model.supports_audio,
        "supports_video": model.supports_video,
        "supports_streaming": model.supports_streaming,
        "free_tier": model.free_tier,
        "premium_account": model.premium_account,
    })
}

fn provider_settings_registration_preview_next_action(
    report: &CatalogProviderSettingsRegistrationReport,
    approval_enabled: bool,
    dry_run_enabled: bool,
) -> String {
    if report.eligible_provider_count == 0 {
        return "Resolve provider registration blockers before enabling catalog provider settings."
            .to_string();
    }

    if !approval_enabled {
        return format!(
            "Review {} eligible provider(s), then set DX_CATALOG_REGISTER_PROVIDER_SETTINGS=1 to allow native settings registration.",
            report.eligible_provider_count
        );
    }

    if dry_run_enabled {
        return format!(
            "Dry run is enabled. Unset DX_CATALOG_REGISTER_PROVIDERS_DRY_RUN to write {} eligible provider(s) into native settings.",
            report.eligible_provider_count
        );
    }

    format!(
        "Provider settings registration is approved for {} provider(s); restart or reload Agent startup to apply the settings bridge.",
        report.eligible_provider_count
    )
}

fn materialize_catalog_artifact_if_approved() {
    if !catalog_generation_approved() {
        return;
    }

    let artifact_path = catalog_materialization_artifact_path();
    let source_revision = env::var(DX_CATALOG_SOURCE_REVISION_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "agent-approved-materialization".to_string());
    let generated_unix_ms = current_unix_ms();
    let mut options =
        CatalogArtifactBuildOptions::new(artifact_path.clone(), source_revision, generated_unix_ms);

    if let Some(last_good_path) = catalog_last_good_artifact_path(&artifact_path) {
        options = options.with_last_good_artifact_path(last_good_path);
    }

    match build_catalog_artifact_from_sources(options) {
        Ok(output) => {
            log::info!(
                "DX catalog artifact materialized at {} from {} available source(s); providers={}, models={}, ready_registration_specs={}/{}",
                output.report.artifact_path.display(),
                output.report.discovery.available_count,
                output.report.build.provider_count,
                output.report.build.model_count,
                output.report.ready_registration_spec_count,
                output.report.registration_spec_count,
            );
        }
        Err(error) => {
            log::warn!(
                "DX catalog artifact materialization was requested but failed for {}: {error}",
                artifact_path.display()
            );
        }
    }
}

fn catalog_generation_approved() -> bool {
    [DX_CATALOG_GENERATE_ENV, DX_CATALOG_GENERATE_ON_LOAD_ENV]
        .iter()
        .copied()
        .any(env_flag_enabled)
}

fn provider_settings_registration_approved() -> bool {
    [
        DX_CATALOG_REGISTER_PROVIDERS_ENV,
        DX_CATALOG_REGISTER_PROVIDER_SETTINGS_ENV,
    ]
    .iter()
    .copied()
    .any(env_flag_enabled)
}

fn can_write_provider_settings(spec: &CatalogProviderAdapterRegistrationSpec) -> bool {
    spec.can_register_settings
        && matches!(
            spec.adapter_kind,
            CatalogExecutionAdapterKind::OpenAiCompatibleHttp
                | CatalogExecutionAdapterKind::OllamaCompatibleHttp
                | CatalogExecutionAdapterKind::OpenRouterHttp
                | CatalogExecutionAdapterKind::LiteLlmProxy
        )
        && spec
            .base_url
            .as_ref()
            .is_some_and(|url| !url.trim().is_empty())
        && !spec.models.is_empty()
}

fn apply_provider_settings_spec(
    settings: &mut settings::SettingsContent,
    spec: &CatalogProviderAdapterRegistrationSpec,
) {
    match spec.adapter_kind {
        CatalogExecutionAdapterKind::OpenRouterHttp => {
            apply_open_router_provider_settings(settings, spec);
        }
        CatalogExecutionAdapterKind::OpenAiCompatibleHttp
        | CatalogExecutionAdapterKind::OllamaCompatibleHttp
        | CatalogExecutionAdapterKind::LiteLlmProxy => {
            apply_openai_compatible_provider_settings(settings, spec);
        }
        _ => {}
    }
}

fn apply_openai_compatible_provider_settings(
    settings: &mut settings::SettingsContent,
    spec: &CatalogProviderAdapterRegistrationSpec,
) {
    let Some(api_url) = spec.base_url.clone() else {
        return;
    };
    let language_models = settings.language_models.get_or_insert_default();
    let providers = language_models.openai_compatible.get_or_insert_default();
    let provider = providers
        .entry(Arc::from(spec.provider_id.as_str()))
        .or_insert_with(|| OpenAiCompatibleSettingsContent {
            api_url: api_url.clone(),
            available_models: Vec::new(),
        });

    if provider.api_url.trim().is_empty() {
        provider.api_url = api_url;
    }
    for model in &spec.models {
        upsert_openai_compatible_model(
            &mut provider.available_models,
            openai_compatible_model_from_catalog(model),
        );
    }
}

fn apply_open_router_provider_settings(
    settings: &mut settings::SettingsContent,
    spec: &CatalogProviderAdapterRegistrationSpec,
) {
    let language_models = settings.language_models.get_or_insert_default();
    let open_router = language_models.open_router.get_or_insert_default();
    let should_set_api_url = open_router
        .api_url
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty();
    if should_set_api_url {
        if let Some(api_url) = &spec.base_url {
            open_router.api_url = Some(api_url.clone());
        }
    }
    let models = open_router.available_models.get_or_insert_default();
    for model in &spec.models {
        upsert_open_router_model(models, open_router_model_from_catalog(model));
    }
}

fn openai_compatible_model_from_catalog(
    model: &CatalogProviderAdapterModelSpec,
) -> OpenAiCompatibleAvailableModel {
    OpenAiCompatibleAvailableModel {
        name: model.model_id.clone(),
        display_name: Some(model.display_name.clone()),
        max_tokens: model
            .context_window_tokens
            .map(u64::from)
            .unwrap_or(DEFAULT_CATALOG_MODEL_MAX_TOKENS),
        max_output_tokens: model.max_output_tokens.map(u64::from),
        max_completion_tokens: model.max_output_tokens.map(u64::from),
        reasoning_effort: None,
        capabilities: OpenAiCompatibleModelCapabilities {
            tools: model.supports_tools,
            images: model.supports_images,
            parallel_tool_calls: false,
            prompt_cache_key: false,
            chat_completions: true,
            interleaved_reasoning: false,
        },
    }
}

fn open_router_model_from_catalog(
    model: &CatalogProviderAdapterModelSpec,
) -> OpenRouterAvailableModel {
    OpenRouterAvailableModel {
        name: model.model_id.clone(),
        display_name: Some(model.display_name.clone()),
        max_tokens: model
            .context_window_tokens
            .map(u64::from)
            .unwrap_or(DEFAULT_CATALOG_MODEL_MAX_TOKENS),
        max_output_tokens: model.max_output_tokens.map(u64::from),
        max_completion_tokens: model.max_output_tokens.map(u64::from),
        supports_tools: Some(model.supports_tools),
        supports_images: Some(model.supports_images),
        mode: None,
        provider: None,
    }
}

fn upsert_openai_compatible_model(
    models: &mut Vec<OpenAiCompatibleAvailableModel>,
    model: OpenAiCompatibleAvailableModel,
) {
    if let Some(existing) = models
        .iter_mut()
        .find(|existing| existing.name == model.name)
    {
        *existing = model;
    } else {
        models.push(model);
    }
}

fn upsert_open_router_model(
    models: &mut Vec<OpenRouterAvailableModel>,
    model: OpenRouterAvailableModel,
) {
    if let Some(existing) = models
        .iter_mut()
        .find(|existing| existing.name == model.name)
    {
        *existing = model;
    } else {
        models.push(model);
    }
}

fn env_flag_enabled(key: &str) -> bool {
    env::var(key)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn catalog_materialization_artifact_path() -> PathBuf {
    for env_var in [
        DX_CATALOG_ARTIFACT_ENV,
        DX_CATALOG_PATH_ENV,
        DX_CATALOG_OUTPUT_ENV,
    ] {
        if let Some(path) = env::var_os(env_var).map(PathBuf::from) {
            return path;
        }
    }

    paths::data_dir()
        .join("dx_catalog")
        .join(DX_CATALOG_ARTIFACT_FILE_NAME)
}

fn catalog_last_good_artifact_path(artifact_path: &Path) -> Option<PathBuf> {
    env::var_os(DX_CATALOG_LAST_GOOD_ENV)
        .map(PathBuf::from)
        .or_else(|| artifact_path.is_file().then(|| artifact_path.to_path_buf()))
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn insert_route_candidates(
    route_candidates: &mut HashMap<String, Vec<String>>,
    route_ids: Vec<String>,
    candidates: Vec<String>,
) {
    if candidates.is_empty() {
        return;
    }

    for route_id in route_ids {
        route_candidates
            .entry(route_id)
            .or_insert(candidates.clone());
    }
}

fn insert_execution_summaries(
    execution_plans: &mut HashMap<String, CatalogExecutionSummary>,
    route_ids: Vec<String>,
    summary: Option<CatalogExecutionSummary>,
) {
    let Some(summary) = summary else {
        return;
    };

    for route_id in route_ids {
        execution_plans.entry(route_id).or_insert(summary.clone());
    }
}

fn push_catalog_model_candidates(
    candidates: &mut Vec<String>,
    model_lookup_keys: &HashMap<String, Vec<String>>,
    catalog_model_id: &str,
) {
    let mut seen = candidates.iter().cloned().collect::<HashSet<_>>();
    for candidate in model_lookup_keys
        .get(catalog_model_id)
        .into_iter()
        .flat_map(|candidates| candidates.iter())
    {
        if seen.insert(candidate.clone()) {
            candidates.push(candidate.clone());
        }
    }
}

fn catalog_role_route_ids(role: RoutingRole) -> Vec<String> {
    let role = routing_role_key(role);
    vec![
        format!("dx_catalog/route/{role}"),
        format!("dx_catalog/{role}"),
        format!("dx/{role}"),
    ]
}

fn routing_role_key(role: RoutingRole) -> &'static str {
    match role {
        RoutingRole::Helper => "helper",
        RoutingRole::ToolAgent => "tool_agent",
        RoutingRole::Coding => "coding",
        RoutingRole::Reasoning => "reasoning",
        RoutingRole::Vision => "vision",
        RoutingRole::Audio => "audio",
        RoutingRole::Embeddings => "embeddings",
        RoutingRole::Fallback => "fallback",
    }
}

fn model_lookup_keys_for_record(model: &ModelRecord, provider: &ProviderRecord) -> Vec<String> {
    let mut keys = Vec::new();
    let mut seen = HashSet::new();

    push_model_lookup_key(&mut keys, &mut seen, model.id.clone());
    push_model_lookup_key(
        &mut keys,
        &mut seen,
        format!("{}/{}", provider.id, model.id),
    );

    for alias in &model.aliases {
        push_model_lookup_key(&mut keys, &mut seen, alias.clone());
        if !alias.contains('/') {
            push_model_lookup_key(&mut keys, &mut seen, format!("{}/{}", provider.id, alias));
        }
    }

    for alias in &provider.aliases {
        push_model_lookup_key(&mut keys, &mut seen, format!("{}/{}", alias, model.id));
        for model_alias in &model.aliases {
            push_model_lookup_key(&mut keys, &mut seen, format!("{}/{}", alias, model_alias));
        }
    }

    keys
}

fn push_model_lookup_key(keys: &mut Vec<String>, seen: &mut HashSet<String>, key: String) {
    if !key.is_empty() && seen.insert(key.clone()) {
        keys.push(key);
    }
}

fn catalog_model_description(
    description: Option<&str>,
    badges: &[String],
    auth_state: AgentPickerAuthState,
) -> Option<String> {
    let mut parts = Vec::new();

    if !badges.is_empty() {
        parts.push(format!("DX catalog: {}", badges.join(", ")));
    }

    if let Some(description) = description.filter(|description| !description.trim().is_empty()) {
        parts.push(description.trim().to_string());
    }

    if auth_state == AgentPickerAuthState::NeedsAuth {
        parts.push("Provider credentials are not configured yet.".to_string());
    }

    (!parts.is_empty()).then(|| parts.join(" "))
}

fn fallback_model_description(model: &ModelRecord, provider: &ProviderRecord) -> Option<String> {
    let mut parts = Vec::new();

    if model.local_runtime.is_some() || provider.is_local {
        parts.push("DX catalog: Local runtime".to_string());
    } else if model.capabilities.free_tier || provider.supports_free_tier {
        parts.push("DX catalog: Free route".to_string());
    } else if model.capabilities.premium_account || provider.supports_premium_account {
        parts.push("DX catalog: Premium route".to_string());
    }

    if let Some(notes) = model.notes.as_deref().or(provider.notes.as_deref()) {
        if !notes.trim().is_empty() {
            parts.push(notes.trim().to_string());
        }
    }

    (!parts.is_empty()).then(|| parts.join(" "))
}
