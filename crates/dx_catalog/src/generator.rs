use crate::{
    AuthProfileLink, CatalogSourceRecord, CatalogValidationReport, DX_CATALOG_SCHEMA_VERSION,
    DxCatalog, ModelRecord, ProviderAuthKind, ProviderRecord, RoutingRule,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct CatalogGeneratorInput {
    pub source: CatalogSourceRecord,
    pub providers: Vec<ProviderRecord>,
    pub models: Vec<ModelRecord>,
    pub routing_rules: Vec<RoutingRule>,
    pub auth_profiles: Vec<ProviderAuthProfileUpdate>,
}

impl CatalogGeneratorInput {
    pub fn new(source: CatalogSourceRecord) -> Self {
        Self {
            source,
            providers: Vec::new(),
            models: Vec::new(),
            routing_rules: Vec::new(),
            auth_profiles: Vec::new(),
        }
    }

    pub fn with_providers(mut self, providers: impl IntoIterator<Item = ProviderRecord>) -> Self {
        self.providers.extend(providers);
        self
    }

    pub fn with_models(mut self, models: impl IntoIterator<Item = ModelRecord>) -> Self {
        self.models.extend(models);
        self
    }

    pub fn with_routing_rules(
        mut self,
        routing_rules: impl IntoIterator<Item = RoutingRule>,
    ) -> Self {
        self.routing_rules.extend(routing_rules);
        self
    }

    pub fn with_auth_profiles(
        mut self,
        auth_profiles: impl IntoIterator<Item = ProviderAuthProfileUpdate>,
    ) -> Self {
        self.auth_profiles.extend(auth_profiles);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAuthProfileUpdate {
    pub provider_id: String,
    pub profile_id: String,
    pub configured: bool,
    pub secret_storage_key: Option<String>,
    pub auth_kind: Option<ProviderAuthKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogGeneratorOptions {
    pub source_revision: String,
    pub generated_unix_ms: u64,
    pub conflict_policy: CatalogConflictPolicy,
    pub use_last_good_on_invalid: bool,
}

impl CatalogGeneratorOptions {
    pub fn new(source_revision: impl Into<String>, generated_unix_ms: u64) -> Self {
        Self {
            source_revision: source_revision.into(),
            generated_unix_ms,
            conflict_policy: CatalogConflictPolicy::PreferLatestSource,
            use_last_good_on_invalid: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogConflictPolicy {
    PreferFirstSource,
    PreferLatestSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogBuildOutput {
    pub catalog: DxCatalog,
    pub report: CatalogBuildReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogBuildReport {
    pub generated_catalog_valid: bool,
    pub used_last_good: bool,
    pub source_count: u32,
    pub provider_count: u32,
    pub model_count: u32,
    pub routing_rule_count: u32,
    pub duplicate_source_ids: Vec<String>,
    pub replaced_provider_ids: Vec<String>,
    pub skipped_provider_ids: Vec<String>,
    pub replaced_model_ids: Vec<String>,
    pub skipped_model_ids: Vec<String>,
    pub replaced_routing_roles: Vec<String>,
    pub skipped_routing_roles: Vec<String>,
    pub applied_auth_profile_provider_ids: Vec<String>,
    pub missing_auth_profile_provider_ids: Vec<String>,
    pub validation: CatalogValidationReport,
}

pub fn build_catalog(
    inputs: impl IntoIterator<Item = CatalogGeneratorInput>,
    options: CatalogGeneratorOptions,
) -> CatalogBuildOutput {
    build_catalog_with_last_good(inputs, options, None)
}

pub fn build_catalog_with_last_good(
    inputs: impl IntoIterator<Item = CatalogGeneratorInput>,
    options: CatalogGeneratorOptions,
    last_good: Option<DxCatalog>,
) -> CatalogBuildOutput {
    let mut source_ids = BTreeSet::new();
    let mut duplicate_source_ids = Vec::new();
    let mut sources = Vec::new();
    let mut providers = BTreeMap::new();
    let mut models = BTreeMap::new();
    let mut routing_rules = Vec::new();
    let mut replaced_provider_ids = Vec::new();
    let mut skipped_provider_ids = Vec::new();
    let mut replaced_model_ids = Vec::new();
    let mut skipped_model_ids = Vec::new();
    let mut replaced_routing_roles = Vec::new();
    let mut skipped_routing_roles = Vec::new();
    let mut auth_profiles = Vec::new();
    let mut applied_auth_profile_provider_ids = Vec::new();
    let mut missing_auth_profile_provider_ids = Vec::new();

    for input in inputs {
        if !source_ids.insert(input.source.id.clone()) {
            duplicate_source_ids.push(input.source.id.clone());
        }
        sources.push(input.source);

        for provider in input.providers {
            merge_record(
                &mut providers,
                provider.id.clone(),
                provider,
                options.conflict_policy,
                &mut replaced_provider_ids,
                &mut skipped_provider_ids,
            );
        }

        for model in input.models {
            merge_record(
                &mut models,
                model.id.clone(),
                model,
                options.conflict_policy,
                &mut replaced_model_ids,
                &mut skipped_model_ids,
            );
        }

        for rule in input.routing_rules {
            merge_routing_rule(
                &mut routing_rules,
                rule,
                options.conflict_policy,
                &mut replaced_routing_roles,
                &mut skipped_routing_roles,
            );
        }

        auth_profiles.extend(input.auth_profiles);
    }

    for auth_profile in auth_profiles {
        if let Some(provider) = providers.get_mut(&auth_profile.provider_id) {
            if let Some(auth_kind) = auth_profile.auth_kind {
                provider.auth = auth_kind;
            }
            provider.auth_profile = Some(AuthProfileLink {
                profile_id: auth_profile.profile_id,
                configured: auth_profile.configured,
                secret_storage_key: auth_profile.secret_storage_key,
            });
            applied_auth_profile_provider_ids.push(provider.id.clone());
        } else {
            missing_auth_profile_provider_ids.push(auth_profile.provider_id);
        }
    }

    let catalog = DxCatalog {
        schema_version: DX_CATALOG_SCHEMA_VERSION,
        generated_unix_ms: options.generated_unix_ms,
        source_revision: options.source_revision,
        sources,
        providers: providers.into_values().collect(),
        models: models.into_values().collect(),
        routing_rules,
    };

    let validation = catalog.validate_references();
    let generated_catalog_valid = validation.is_valid;
    let (catalog, use_last_good) = if !generated_catalog_valid && options.use_last_good_on_invalid {
        match last_good {
            Some(last_good) => (last_good, true),
            None => (catalog, false),
        }
    } else {
        (catalog, false)
    };

    let report = CatalogBuildReport {
        generated_catalog_valid,
        used_last_good: use_last_good,
        source_count: catalog.sources.len() as u32,
        provider_count: catalog.providers.len() as u32,
        model_count: catalog.models.len() as u32,
        routing_rule_count: catalog.routing_rules.len() as u32,
        duplicate_source_ids,
        replaced_provider_ids,
        skipped_provider_ids,
        replaced_model_ids,
        skipped_model_ids,
        replaced_routing_roles,
        skipped_routing_roles,
        applied_auth_profile_provider_ids,
        missing_auth_profile_provider_ids,
        validation,
    };

    CatalogBuildOutput { catalog, report }
}

fn merge_record<T>(
    records: &mut BTreeMap<String, T>,
    id: String,
    record: T,
    conflict_policy: CatalogConflictPolicy,
    replaced_ids: &mut Vec<String>,
    skipped_ids: &mut Vec<String>,
) {
    if records.contains_key(&id) {
        match conflict_policy {
            CatalogConflictPolicy::PreferFirstSource => {
                skipped_ids.push(id);
            }
            CatalogConflictPolicy::PreferLatestSource => {
                replaced_ids.push(id.clone());
                records.insert(id, record);
            }
        }
    } else {
        records.insert(id, record);
    }
}

fn merge_routing_rule(
    rules: &mut Vec<RoutingRule>,
    rule: RoutingRule,
    conflict_policy: CatalogConflictPolicy,
    replaced_roles: &mut Vec<String>,
    skipped_roles: &mut Vec<String>,
) {
    if let Some(existing) = rules.iter_mut().find(|existing| existing.role == rule.role) {
        let role = format!("{:?}", rule.role);
        match conflict_policy {
            CatalogConflictPolicy::PreferFirstSource => {
                skipped_roles.push(role);
            }
            CatalogConflictPolicy::PreferLatestSource => {
                replaced_roles.push(role);
                *existing = rule;
            }
        }
    } else {
        rules.push(rule);
    }
}
