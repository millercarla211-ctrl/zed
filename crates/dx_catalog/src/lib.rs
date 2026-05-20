mod artifact;
mod error;
mod generator;
mod status;
mod types;

pub use artifact::{
    CatalogArtifactHeader, CatalogArtifactRef, DX_CATALOG_ARTIFACT_VERSION, DX_CATALOG_MAGIC,
    MappedCatalogArtifact, deserialize_trusted_catalog_payload, read_catalog_artifact,
    serialize_catalog_payload, write_catalog_artifact,
};
pub use error::DxCatalogError;
pub use generator::{
    CatalogBuildOutput, CatalogBuildReport, CatalogConflictPolicy, CatalogGeneratorInput,
    CatalogGeneratorOptions, build_catalog, build_catalog_with_last_good,
};
pub use status::{DxLaunchStatus, LaunchFeatureStatus, current_launch_status};
pub use types::{
    AuthProfileLink, CatalogSourceKind, CatalogSourceRecord, CatalogValidationReport,
    DX_CATALOG_SCHEMA_VERSION, DxCatalog, LocalRuntimeHints, LocalRuntimeKind, ModelCapabilities,
    ModelPricingMicros, ModelRecord, ProviderAuthKind, ProviderKind, ProviderRecord, RoutingRole,
    RoutingRule,
};

pub type Result<T> = std::result::Result<T, DxCatalogError>;
