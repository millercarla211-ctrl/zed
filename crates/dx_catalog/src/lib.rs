mod adapters;
mod agent_picker;
mod artifact;
mod error;
mod execution;
mod generator;
mod materialize;
mod model_catalog_readers;
mod provider_readers;
mod readers;
mod receipt_cache;
mod sources;
mod status;
mod types;

pub use adapters::{
    AuthProfileInput, ExternalModelInput, ExternalProviderInput, FlowLocalRoleInput,
    LiteLlmAliasInput, LlamaCppModelInput, SourceMetadata, auth_profiles_input,
    flow_local_roles_input, lite_llm_aliases_input, lite_llm_catalog_input, llama_cpp_scan_input,
    models_dev_input, openrouter_input, zeroclaw_providers_input,
};
pub use agent_picker::{
    AgentPickerAuthState, AgentPickerGroup, AgentPickerModel, AgentPickerProjection,
    AgentPickerProjectionOptions, AgentPickerProjectionSummary, AgentRoutePreferences,
    CatalogRouteSelection, build_agent_picker_projection, select_catalog_route,
};
pub use artifact::{
    CatalogArtifactHeader, CatalogArtifactRef, DX_CATALOG_ARTIFACT_VERSION, DX_CATALOG_MAGIC,
    MappedCatalogArtifact, deserialize_trusted_catalog_payload, read_catalog_artifact,
    serialize_catalog_payload, write_catalog_artifact,
};
pub use error::DxCatalogError;
pub use execution::{
    CatalogExecutionAdapterKind, CatalogExecutionPermission, CatalogExecutionPlan,
    CatalogExecutionPlanRequest, CatalogProviderAdapterModelSpec,
    CatalogProviderAdapterRegistrationSpec, build_catalog_execution_plan,
    build_catalog_provider_registration_specs,
};
pub use generator::{
    CatalogBuildOutput, CatalogBuildReport, CatalogConflictPolicy, CatalogGeneratorInput,
    CatalogGeneratorOptions, ProviderAuthProfileUpdate, build_catalog,
    build_catalog_with_last_good,
};
pub use materialize::{
    CatalogArtifactBuildOptions, CatalogArtifactBuildOutput, CatalogArtifactBuildReport,
    CatalogSourceReadError, CatalogSourceReadOptions, CatalogSourceReadOutput,
    CatalogSourceReadReport, ManualCatalogSourceReport, SkippedCatalogSourceCandidate,
    build_catalog_artifact_from_sources, read_discovered_catalog_sources,
};
pub use model_catalog_readers::{
    ModelCatalogReadOutput, ModelCatalogReadReport, ModelCatalogReaderOptions,
    SkippedModelCatalogEntry, read_model_catalog_file, read_model_catalog_json,
};
pub use provider_readers::{
    ProviderSourceReadOutput, ProviderSourceReadReport, ProviderSourceReaderOptions,
    SkippedProviderSourceEntry, read_provider_source, read_provider_source_root,
};
pub use readers::{
    LocalModelCatalogReadOutput, LocalModelCatalogReadReport, LocalModelSourceReaderOptions,
    SkippedLocalModelFile, read_local_model_source, read_local_models_from_root,
};
pub use receipt_cache::{
    DX_RECEIPT_CACHE_ARTIFACT_VERSION, DX_RECEIPT_CACHE_MAGIC, DX_RECEIPT_CACHE_SCHEMA_VERSION,
    DxReceiptCacheEntry, DxReceiptCacheEntryKind, DxReceiptCacheFreshness, DxReceiptCacheManifest,
    DxReceiptCacheRoot, MappedReceiptCacheArtifact, ReceiptCacheArtifactHeader,
    ReceiptCacheArtifactRef, deserialize_trusted_receipt_cache_payload,
    read_receipt_cache_artifact, serialize_receipt_cache_payload, write_receipt_cache_artifact,
};
pub use sources::{
    CatalogSourceCandidate, CatalogSourceCandidateStatus, CatalogSourceDiscoveryConfig,
    CatalogSourceDiscoveryReport, CatalogSourcePurpose,
};
pub use status::{DxLaunchStatus, LaunchFeatureStatus, current_launch_status};
pub use types::{
    AuthProfileLink, CatalogSourceKind, CatalogSourceRecord, CatalogValidationReport,
    DX_CATALOG_SCHEMA_VERSION, DxCatalog, LocalRuntimeHints, LocalRuntimeKind, ModelCapabilities,
    ModelPricingMicros, ModelRecord, ProviderAuthKind, ProviderKind, ProviderRecord, RoutingRole,
    RoutingRule,
};

pub type Result<T> = std::result::Result<T, DxCatalogError>;
