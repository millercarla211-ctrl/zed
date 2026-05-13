use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum CatalogSource {
    LocalRegistry,
    ModelsDev,
    LiteLlm,
    NativeProviderScan,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct ProviderCatalogPlan {
    pub sources: Vec<CatalogSource>,
    pub normalize_model_ids: bool,
    pub notes: Vec<String>,
}

pub struct ProviderCatalogBridge;

impl ProviderCatalogBridge {
    pub fn default_plan() -> ProviderCatalogPlan {
        ProviderCatalogPlan {
            sources: vec![
                CatalogSource::LocalRegistry,
                CatalogSource::ModelsDev,
                CatalogSource::LiteLlm,
                CatalogSource::NativeProviderScan,
            ],
            normalize_model_ids: true,
            notes: vec![
                "Use models.dev as a broad metadata catalog for model capabilities, pricing, and provider IDs."
                    .to_string(),
                "Use LiteLLM-style normalization when a unified remote model namespace improves coverage."
                    .to_string(),
                "Keep a local registry so Flow can stay useful even when remote catalogs are unavailable."
                    .to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_catalog_plan_uses_models_dev_and_litellm() {
        let plan = ProviderCatalogBridge::default_plan();
        assert!(plan.sources.contains(&CatalogSource::ModelsDev));
        assert!(plan.sources.contains(&CatalogSource::LiteLlm));
    }
}
