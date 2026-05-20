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
use language_model::{LanguageModelProviderId, LanguageModelRegistry};
use language_models::AllLanguageModelSettings;
use settings::{
    OpenAiCompatibleAvailableModel, OpenAiCompatibleModelCapabilities,
    OpenAiCompatibleSettingsContent, OpenRouterAvailableModel, Settings as _, update_settings_file,
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
    "zed.dx_catalog.provider_settings.registration_preview.v2";
pub(crate) const DX_CATALOG_PROVIDER_SETTINGS_REGISTRATION_SCHEMA: &str =
    "zed.dx_catalog.provider_settings.registration_result.v1";

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

        let mut models = HashMap::default();
        let mut route_candidates = HashMap::default();
        let mut model_lookup_keys = HashMap::default();
        let mut execution_plans = HashMap::default();
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
                models
                    .entry(key.clone())
                    .or_insert_with(|| presentation.clone());
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

#[derive(Debug, Default)]
struct CatalogProviderSettingsLiveValidationReport {
    settings_registered_provider_count: usize,
    registry_registered_provider_count: usize,
    authenticated_provider_count: usize,
    settings_model_match_count: usize,
    registry_model_match_count: usize,
    executable_now_provider_count: usize,
    blocked_provider_count: usize,
}

impl CatalogProviderSettingsLiveValidationReport {
    fn record(&mut self, validation: &CatalogProviderSettingsLiveValidation) {
        if validation.settings_registered {
            self.settings_registered_provider_count += 1;
        }
        if validation.registry_provider_registered {
            self.registry_registered_provider_count += 1;
        }
        if validation.provider_authenticated {
            self.authenticated_provider_count += 1;
        }
        self.settings_model_match_count += validation.settings_model_match_count;
        self.registry_model_match_count += validation.registry_model_match_count;
        if validation.executable_now {
            self.executable_now_provider_count += 1;
        } else {
            self.blocked_provider_count += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct CatalogProviderSettingsLiveValidation {
    settings_registered: bool,
    settings_api_url: Option<String>,
    settings_api_url_matches_catalog: bool,
    settings_model_match_count: usize,
    registry_provider_registered: bool,
    provider_authenticated: bool,
    registry_model_match_count: usize,
    credential_ready: bool,
    runtime_ready: bool,
    executable_now: bool,
    remaining_execution_blockers: Vec<String>,
    blockers: Vec<String>,
    status: &'static str,
    next_action: String,
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

pub(crate) fn provider_settings_registration_preview(cx: &gpui::App) -> serde_json::Value {
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
                "settings_registered_provider_count": 0,
                "registry_registered_provider_count": 0,
                "authenticated_provider_count": 0,
                "settings_model_match_count": 0,
                "registry_model_match_count": 0,
                "executable_now_provider_count": 0,
                "blocked_live_validation_provider_count": 0,
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
    let mut live_report = CatalogProviderSettingsLiveValidationReport::default();
    let providers = specs
        .iter()
        .map(|spec| {
            let live_validation = provider_settings_live_validation(spec, cx);
            live_report.record(&live_validation);
            provider_settings_registration_preview_provider(spec, &live_validation)
        })
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
            "settings_registered_provider_count": live_report.settings_registered_provider_count,
            "registry_registered_provider_count": live_report.registry_registered_provider_count,
            "authenticated_provider_count": live_report.authenticated_provider_count,
            "settings_model_match_count": live_report.settings_model_match_count,
            "registry_model_match_count": live_report.registry_model_match_count,
            "executable_now_provider_count": live_report.executable_now_provider_count,
            "blocked_live_validation_provider_count": live_report.blocked_provider_count,
        },
        "providers": providers,
        "next_action": provider_settings_registration_preview_next_action(
            &report,
            &live_report,
            approval_enabled,
            dry_run_enabled,
        ),
    })
}

pub(crate) fn register_provider_settings_from_catalog(
    fs: Arc<dyn Fs>,
    provider_ids: &[String],
    dry_run: bool,
    cx: &gpui::App,
) -> serde_json::Value {
    let requested_provider_ids = provider_ids
        .iter()
        .map(|provider_id| provider_id.trim().to_ascii_lowercase())
        .filter(|provider_id| !provider_id.is_empty())
        .collect::<Vec<_>>();
    let requested_filter = (!requested_provider_ids.is_empty()).then(|| {
        requested_provider_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>()
    });
    let artifact_candidates = catalog_artifact_candidate_preview();
    let Some(artifact) =
        read_first_available_catalog_artifact_with_path("provider settings registration tool")
    else {
        return serde_json::json!({
            "schema": DX_CATALOG_PROVIDER_SETTINGS_REGISTRATION_SCHEMA,
            "generated_at_ms": current_unix_ms(),
            "artifact_loaded": false,
            "artifact_path": serde_json::Value::Null,
            "artifact_candidates": artifact_candidates,
            "dry_run": dry_run,
            "settings_write_queued": false,
            "requested_provider_ids": requested_provider_ids,
            "requested_provider_ids_not_found": [],
            "summary": {
                "catalog_provider_count": 0,
                "catalog_model_count": 0,
                "registration_spec_count": 0,
                "eligible_provider_count": 0,
                "skipped_provider_count": 0,
                "matched_provider_count": 0,
                "selected_provider_count": 0,
                "selected_model_count": 0,
            },
            "providers": [],
            "next_action": "Generate or point DX_CATALOG_ARTIFACT/DX_CATALOG_PATH at a validated catalog.dxcat before registering provider settings.",
        });
    };

    let specs = build_catalog_provider_registration_specs(&artifact.catalog);
    let report = CatalogProviderSettingsRegistrationReport::from_specs(&specs);
    let requested_provider_ids_not_found = requested_provider_ids
        .iter()
        .filter(|requested_provider_id| {
            !specs
                .iter()
                .any(|spec| spec.provider_id.eq_ignore_ascii_case(requested_provider_id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let requested_not_found_count = requested_provider_ids_not_found.len();
    let matches_requested_filter = |spec: &CatalogProviderAdapterRegistrationSpec| {
        requested_filter.as_ref().map_or(true, |filter| {
            filter.contains(&spec.provider_id.to_ascii_lowercase())
        })
    };
    let matched_specs = specs
        .iter()
        .filter(|spec| matches_requested_filter(spec))
        .cloned()
        .collect::<Vec<_>>();
    let selected_specs = matched_specs
        .iter()
        .filter(|spec| can_write_provider_settings(spec))
        .cloned()
        .collect::<Vec<_>>();
    let selected_report = CatalogProviderSettingsRegistrationReport::from_specs(&selected_specs);
    let settings_write_queued = !dry_run && !selected_specs.is_empty();

    if settings_write_queued {
        let specs_to_apply = selected_specs.clone();
        update_settings_file(fs, cx, move |settings, _| {
            for spec in &specs_to_apply {
                apply_provider_settings_spec(settings, spec);
            }
        });
    }

    let providers = matched_specs
        .iter()
        .map(|spec| {
            let live_validation = provider_settings_live_validation(spec, cx);
            let mut provider =
                provider_settings_registration_preview_provider(spec, &live_validation);
            let selected_for_registration = can_write_provider_settings(spec);
            let registration_action = if !selected_for_registration {
                "skipped"
            } else if dry_run {
                "would_register"
            } else {
                "registered"
            };

            if let Some(object) = provider.as_object_mut() {
                object.insert(
                    "selected_for_registration".to_string(),
                    selected_for_registration.into(),
                );
                object.insert(
                    "registration_action".to_string(),
                    registration_action.into(),
                );
            }

            provider
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "schema": DX_CATALOG_PROVIDER_SETTINGS_REGISTRATION_SCHEMA,
        "generated_at_ms": current_unix_ms(),
        "artifact_loaded": true,
        "artifact_path": artifact.path.display().to_string(),
        "artifact_candidates": artifact_candidates,
        "dry_run": dry_run,
        "settings_write_queued": settings_write_queued,
        "requested_provider_ids": requested_provider_ids,
        "requested_provider_ids_not_found": requested_provider_ids_not_found,
        "summary": {
            "catalog_provider_count": artifact.catalog.providers.len(),
            "catalog_model_count": artifact.catalog.models.len(),
            "registration_spec_count": specs.len(),
            "eligible_provider_count": report.eligible_provider_count,
            "skipped_provider_count": report.skipped_provider_count,
            "matched_provider_count": matched_specs.len(),
            "selected_provider_count": selected_specs.len(),
            "selected_openai_compatible_provider_count": selected_report.openai_compatible_provider_count,
            "selected_open_router_model_count": selected_report.open_router_model_count,
            "selected_model_count": selected_report.model_count,
        },
        "providers": providers,
        "next_action": provider_settings_registration_result_next_action(
            selected_specs.len(),
            requested_not_found_count,
            dry_run,
            settings_write_queued,
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

fn provider_settings_live_validation(
    spec: &CatalogProviderAdapterRegistrationSpec,
    cx: &gpui::App,
) -> CatalogProviderSettingsLiveValidation {
    let settings = AllLanguageModelSettings::get_global(cx);
    let registry = LanguageModelRegistry::read_global(cx);
    let registry_provider_id = registry_provider_id_for_catalog_spec(spec);
    let registry_provider = registry.provider(&registry_provider_id);
    let registry_provider_registered = registry_provider.is_some();
    let provider_authenticated = registry_provider
        .as_ref()
        .is_some_and(|provider| provider.is_authenticated(cx));
    let registry_model_match_count = registry_provider
        .as_ref()
        .map(|provider| {
            let catalog_model_ids = spec
                .models
                .iter()
                .map(|model| model.model_id.as_str())
                .collect::<HashSet<_>>();
            provider
                .provided_models(cx)
                .into_iter()
                .filter(|model| catalog_model_ids.contains(model.id().0.as_ref()))
                .count()
        })
        .unwrap_or_default();
    let (
        settings_registered,
        settings_api_url,
        settings_api_url_matches_catalog,
        settings_model_match_count,
    ) = provider_settings_native_state(spec, settings);
    let credential_ready = provider_settings_credentials_ready(spec, provider_authenticated);
    let remaining_execution_blockers =
        remaining_execution_blockers_after_live_validation(spec, credential_ready);
    let runtime_ready = settings_registered && registry_provider_registered;
    let executable_now = can_write_provider_settings(spec)
        && runtime_ready
        && credential_ready
        && settings_api_url_matches_catalog
        && settings_model_match_count > 0
        && registry_model_match_count > 0
        && remaining_execution_blockers.is_empty();
    let mut blockers = remaining_execution_blockers.clone();

    if !can_write_provider_settings(spec) {
        blockers
            .push("Provider is not eligible for the native provider-settings writer.".to_string());
    }
    if can_write_provider_settings(spec) && !settings_registered {
        blockers.push(
            "Provider settings are not registered in native language-model settings.".to_string(),
        );
    }
    if can_write_provider_settings(spec) && settings_registered && settings_model_match_count == 0 {
        blockers.push(
            "Native language-model settings do not include matching catalog models.".to_string(),
        );
    }
    if settings_registered && !settings_api_url_matches_catalog {
        blockers.push("Native provider API URL does not match the catalog base URL.".to_string());
    }
    if settings_registered && !registry_provider_registered {
        blockers.push(
            "Native language-model provider is not registered in the runtime registry yet."
                .to_string(),
        );
    }
    if registry_provider_registered && !credential_ready {
        blockers.push(
            "Provider credentials are not configured or loaded in the runtime registry."
                .to_string(),
        );
    }
    if registry_provider_registered && credential_ready && registry_model_match_count == 0 {
        blockers.push("Runtime provider exposes no matching catalog models yet.".to_string());
    }
    blockers.sort();
    blockers.dedup();

    let status = if executable_now {
        "executable_now"
    } else if can_write_provider_settings(spec) && !settings_registered {
        "ready_for_settings_registration"
    } else if settings_registered && !registry_provider_registered {
        "registered_needs_runtime_provider"
    } else if registry_provider_registered && !credential_ready {
        "registered_needs_credentials"
    } else if registry_provider_registered && registry_model_match_count == 0 {
        "registered_needs_model_refresh"
    } else {
        "blocked"
    };

    let next_action = provider_settings_live_validation_next_action(status, spec, &blockers);

    CatalogProviderSettingsLiveValidation {
        settings_registered,
        settings_api_url,
        settings_api_url_matches_catalog,
        settings_model_match_count,
        registry_provider_registered,
        provider_authenticated,
        registry_model_match_count,
        credential_ready,
        runtime_ready,
        executable_now,
        remaining_execution_blockers,
        blockers,
        status,
        next_action,
    }
}

fn registry_provider_id_for_catalog_spec(
    spec: &CatalogProviderAdapterRegistrationSpec,
) -> LanguageModelProviderId {
    match spec.adapter_kind {
        CatalogExecutionAdapterKind::OpenRouterHttp => LanguageModelProviderId::new("openrouter"),
        _ => LanguageModelProviderId::from(spec.provider_id.clone()),
    }
}

fn provider_settings_native_state(
    spec: &CatalogProviderAdapterRegistrationSpec,
    settings: &AllLanguageModelSettings,
) -> (bool, Option<String>, bool, usize) {
    let catalog_model_ids = spec
        .models
        .iter()
        .map(|model| model.model_id.as_str())
        .collect::<HashSet<_>>();

    match spec.adapter_kind {
        CatalogExecutionAdapterKind::OpenRouterHttp => {
            let api_url = settings.open_router.api_url.clone();
            let model_match_count = settings
                .open_router
                .available_models
                .iter()
                .filter(|model| catalog_model_ids.contains(model.name.as_str()))
                .count();
            (
                !api_url.trim().is_empty(),
                Some(api_url.clone()),
                catalog_base_url_matches_settings(spec.base_url.as_deref(), Some(api_url.as_str())),
                model_match_count,
            )
        }
        CatalogExecutionAdapterKind::OpenAiCompatibleHttp
        | CatalogExecutionAdapterKind::OllamaCompatibleHttp
        | CatalogExecutionAdapterKind::LiteLlmProxy => {
            let Some(provider_settings) = settings.openai_compatible.get(spec.provider_id.as_str())
            else {
                return (false, None, false, 0);
            };
            let model_match_count = provider_settings
                .available_models
                .iter()
                .filter(|model| catalog_model_ids.contains(model.name.as_str()))
                .count();
            (
                true,
                Some(provider_settings.api_url.clone()),
                catalog_base_url_matches_settings(
                    spec.base_url.as_deref(),
                    Some(provider_settings.api_url.as_str()),
                ),
                model_match_count,
            )
        }
        _ => (false, None, false, 0),
    }
}

fn catalog_base_url_matches_settings(
    catalog_url: Option<&str>,
    settings_url: Option<&str>,
) -> bool {
    let Some(catalog_url) = catalog_url else {
        return true;
    };
    let Some(settings_url) = settings_url else {
        return false;
    };
    normalized_url(catalog_url) == normalized_url(settings_url)
}

fn normalized_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_ascii_lowercase()
}

fn provider_settings_credentials_ready(
    spec: &CatalogProviderAdapterRegistrationSpec,
    provider_authenticated: bool,
) -> bool {
    match spec.permission {
        CatalogExecutionPermission::None | CatalogExecutionPermission::LocalRuntime => true,
        CatalogExecutionPermission::ApiKey
        | CatalogExecutionPermission::OAuth
        | CatalogExecutionPermission::BrowserSession
        | CatalogExecutionPermission::NativeAccount => {
            spec.auth_configured || provider_authenticated
        }
        CatalogExecutionPermission::Custom => spec.auth_configured,
    }
}

fn remaining_execution_blockers_after_live_validation(
    spec: &CatalogProviderAdapterRegistrationSpec,
    credential_ready: bool,
) -> Vec<String> {
    spec.execution_blockers
        .iter()
        .filter(|blocker| {
            !(credential_ready
                && blocker.contains("requires")
                && blocker.contains("configuration before execution"))
        })
        .cloned()
        .collect()
}

fn provider_settings_live_validation_next_action(
    status: &str,
    spec: &CatalogProviderAdapterRegistrationSpec,
    blockers: &[String],
) -> String {
    match status {
        "executable_now" => format!(
            "Provider `{}` is registered, authenticated, exposes matching models, and is ready for catalog route exposure.",
            spec.provider_id
        ),
        "ready_for_settings_registration" => format!(
            "Enable `DX_CATALOG_REGISTER_PROVIDER_SETTINGS=1` to write `{}` into native language-model settings.",
            spec.provider_id
        ),
        "registered_needs_runtime_provider" => format!(
            "Restart or reload language-model providers so `{}` is registered in the runtime registry.",
            spec.provider_id
        ),
        "registered_needs_credentials" => format!(
            "Configure credentials for `{}` in the provider settings UI or supported environment variable.",
            spec.provider_id
        ),
        "registered_needs_model_refresh" => format!(
            "Refresh provider models for `{}` after settings and credentials are loaded.",
            spec.provider_id
        ),
        _ => blockers.first().cloned().unwrap_or_else(|| {
            format!(
                "Resolve live validation blockers before exposing `{}` as executable.",
                spec.provider_id
            )
        }),
    }
}

fn provider_settings_registration_preview_provider(
    spec: &CatalogProviderAdapterRegistrationSpec,
    live_validation: &CatalogProviderSettingsLiveValidation,
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
        "live_validation": provider_settings_live_validation_json(live_validation),
        "next_action": spec.next_action.as_str(),
    })
}

fn provider_settings_live_validation_json(
    validation: &CatalogProviderSettingsLiveValidation,
) -> serde_json::Value {
    serde_json::json!({
        "status": validation.status,
        "settings_registered": validation.settings_registered,
        "settings_api_url": validation.settings_api_url.as_deref(),
        "settings_api_url_matches_catalog": validation.settings_api_url_matches_catalog,
        "settings_model_match_count": validation.settings_model_match_count,
        "registry_provider_registered": validation.registry_provider_registered,
        "provider_authenticated": validation.provider_authenticated,
        "registry_model_match_count": validation.registry_model_match_count,
        "credential_ready": validation.credential_ready,
        "runtime_ready": validation.runtime_ready,
        "executable_now": validation.executable_now,
        "remaining_execution_blockers": &validation.remaining_execution_blockers,
        "blockers": &validation.blockers,
        "next_action": validation.next_action.as_str(),
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
    live_report: &CatalogProviderSettingsLiveValidationReport,
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

    if live_report.executable_now_provider_count > 0 {
        return format!(
            "{} provider(s) are live-valid and executable now; keep resolving blockers for {} remaining provider(s) before broad catalog route exposure.",
            live_report.executable_now_provider_count, live_report.blocked_provider_count
        );
    }

    format!(
        "Provider settings registration is approved for {} provider(s); restart or reload Agent startup to apply the settings bridge.",
        report.eligible_provider_count
    )
}

fn provider_settings_registration_result_next_action(
    selected_provider_count: usize,
    requested_not_found_count: usize,
    dry_run: bool,
    settings_write_queued: bool,
) -> String {
    if selected_provider_count == 0 {
        if requested_not_found_count > 0 {
            return "No requested provider IDs matched writable catalog provider settings specs. Inspect the preview tool for exact provider IDs.".to_string();
        }

        return "No matched catalog providers are writable by the native provider-settings bridge yet."
            .to_string();
    }

    if dry_run {
        return format!(
            "Dry run complete for {selected_provider_count} provider(s). Run again with dry_run=false after reviewing the receipt to queue native settings updates."
        );
    }

    if settings_write_queued {
        return format!(
            "Queued native settings updates for {selected_provider_count} provider(s). Restart or reload language-model providers, then inspect live validation again."
        );
    }

    "Review the registration receipt and rerun with dry_run=false when ready.".to_string()
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
    let mut seen = HashSet::default();

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
