use acp_thread::AgentModelInfo;
use collections::{HashMap, HashSet};
use dx_catalog::{
    AgentPickerAuthState, AgentPickerProjectionOptions, DxCatalog, ModelRecord, ProviderRecord,
    build_agent_picker_projection, read_catalog_artifact,
};
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

const DX_CATALOG_ARTIFACT_ENV: &str = "DX_CATALOG_ARTIFACT";
const DX_CATALOG_PATH_ENV: &str = "DX_CATALOG_PATH";
const DX_CATALOG_ARTIFACT_FILE_NAME: &str = "catalog.dxcat";

#[derive(Clone, Debug, Default)]
pub struct DxCatalogAgentBridge {
    models: HashMap<String, CatalogModelPresentation>,
}

#[derive(Clone, Debug)]
struct CatalogModelPresentation {
    description: Option<String>,
    cost_label: Option<String>,
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

    fn load_uncached() -> Option<Self> {
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
        let picker_models = projection
            .groups
            .into_iter()
            .flat_map(|group| group.models)
            .map(|model| (model.model_id.clone(), model))
            .collect::<HashMap<_, _>>();

        let mut models = HashMap::new();
        for model in &catalog.models {
            let Some(provider) = providers.get(model.provider_id.as_str()).copied() else {
                continue;
            };

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

            for key in model_lookup_keys(model, provider) {
                models.entry(key).or_insert_with(|| presentation.clone());
            }
        }

        Self { models }
    }
}

struct CatalogArtifactCandidate {
    path: PathBuf,
    env_var: Option<&'static str>,
}

fn catalog_artifact_candidates() -> Vec<CatalogArtifactCandidate> {
    let mut candidates = Vec::new();

    for env_var in [DX_CATALOG_ARTIFACT_ENV, DX_CATALOG_PATH_ENV] {
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

fn model_lookup_keys(model: &ModelRecord, provider: &ProviderRecord) -> Vec<String> {
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
