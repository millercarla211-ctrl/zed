use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

use crate::embed::ProviderAuthKind;
use crate::runtime::Modality;

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
pub enum AccessTier {
    LocalUnlimited,
    FreeRemote,
    PremiumRemote,
}

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
pub enum RemoteCapability {
    Chat,
    Vision,
    Audio,
    Image,
    Video,
    Embeddings,
    Tools,
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
pub struct RemoteModelEndpoint {
    pub provider_id: String,
    pub model_id: String,
    pub label: String,
    pub access_tier: AccessTier,
    pub auth_kind: ProviderAuthKind,
    pub capabilities: Vec<RemoteCapability>,
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
pub struct UnifiedUsagePolicy {
    pub prefer_local: bool,
    pub use_free_remote_pool: bool,
    pub use_premium_remote_pool: bool,
    pub auto_switch: bool,
    pub local_fallback_required: bool,
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
pub struct SeamlessRoutePlan {
    pub modality: Modality,
    pub local_model_key: Option<String>,
    pub remote_candidates: Vec<RemoteModelEndpoint>,
    pub policy: UnifiedUsagePolicy,
    pub notes: Vec<String>,
}

pub struct RemoteProviderRouter;

impl RemoteProviderRouter {
    pub fn default_speech_remote_candidates() -> Vec<RemoteModelEndpoint> {
        vec![RemoteModelEndpoint {
            provider_id: "nvidia-nim".to_string(),
            model_id: "nvidia/nemotron-speech-streaming-en-0.6b".to_string(),
            label: "NVIDIA NIM Nemotron Speech Streaming EN 0.6B".to_string(),
            access_tier: AccessTier::PremiumRemote,
            auth_kind: ProviderAuthKind::ApiKey,
            capabilities: vec![RemoteCapability::Audio],
        }]
    }

    pub fn default_policy() -> UnifiedUsagePolicy {
        UnifiedUsagePolicy {
            prefer_local: true,
            use_free_remote_pool: true,
            use_premium_remote_pool: true,
            auto_switch: true,
            local_fallback_required: true,
        }
    }

    pub fn plan(
        modality: Modality,
        local_model_key: Option<String>,
        mut remote_candidates: Vec<RemoteModelEndpoint>,
    ) -> SeamlessRoutePlan {
        remote_candidates.sort_by_key(|endpoint| match endpoint.access_tier {
            AccessTier::FreeRemote => 0,
            AccessTier::PremiumRemote => 1,
            AccessTier::LocalUnlimited => 2,
        });

        SeamlessRoutePlan {
            modality,
            local_model_key,
            remote_candidates,
            policy: Self::default_policy(),
            notes: vec![
                "Local models should remain the first unlimited path whenever they meet the task and device constraints."
                    .to_string(),
                "Remote free offers are the next pool to consume before premium accounts when quality and latency remain acceptable."
                    .to_string(),
                "Premium remote accounts should be preserved for cases where local and free remote paths do not meet the task."
                    .to_string(),
                "Audio-capable remote providers are optional fallbacks only and must not be used silently while local-only mode is required."
                    .to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_candidates_are_sorted_free_before_premium() {
        let premium = RemoteModelEndpoint {
            provider_id: "premium".to_string(),
            model_id: "model-premium".to_string(),
            label: "Premium".to_string(),
            access_tier: AccessTier::PremiumRemote,
            auth_kind: ProviderAuthKind::ApiKey,
            capabilities: vec![RemoteCapability::Chat],
        };
        let free = RemoteModelEndpoint {
            provider_id: "free".to_string(),
            model_id: "model-free".to_string(),
            label: "Free".to_string(),
            access_tier: AccessTier::FreeRemote,
            auth_kind: ProviderAuthKind::OAuth,
            capabilities: vec![RemoteCapability::Chat],
        };

        let plan = RemoteProviderRouter::plan(
            Modality::Chat,
            Some("qwen3-0.6b".to_string()),
            vec![premium, free],
        );
        assert_eq!(plan.remote_candidates[0].provider_id, "free");
    }
}
