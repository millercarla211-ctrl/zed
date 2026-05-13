use rkyv::{
    Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize,
};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

use crate::runtime::{ActivationConfig, DeviceTier, RuntimeBroker};

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
pub enum ResidentService {
    WakeWordMonitor,
    GlobalHotkeys,
    MicrophoneCapture,
    VoiceActivityDetection,
    PromptCache,
    GrammarLayer,
    DictationCleanup,
    TypingAssistant,
    LocalLlmWarmPool,
    LocalSttWarmPool,
    LocalTtsWarmPool,
    OcrOnDemand,
    VlmOnDemand,
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
pub enum BatteryPolicy {
    IgnoreBattery,
    PreferEfficiency,
    PauseHeavyServices,
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
pub enum ThermalPolicy {
    IgnoreTemperature,
    BackoffHeavyServices,
    SuspendLargeModels,
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
pub struct ResidentModelPlan {
    pub model_key: String,
    pub keep_loaded: bool,
    pub reason: String,
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
pub struct AlwaysOnRuntimePlan {
    pub enabled: bool,
    pub device_tier: DeviceTier,
    pub activation: ActivationConfig,
    pub resident_services: Vec<ResidentService>,
    pub resident_models: Vec<ResidentModelPlan>,
    pub battery_policy: BatteryPolicy,
    pub thermal_policy: ThermalPolicy,
    pub notes: Vec<String>,
}

pub struct AlwaysOnRuntimeDirector;

impl AlwaysOnRuntimeDirector {
    pub fn plan(broker: &RuntimeBroker) -> AlwaysOnRuntimePlan {
        let tier = broker.device_profile().tier;
        let mut resident_services = vec![
            ResidentService::WakeWordMonitor,
            ResidentService::GlobalHotkeys,
            ResidentService::PromptCache,
            ResidentService::GrammarLayer,
            ResidentService::TypingAssistant,
        ];
        let mut resident_models = Vec::new();
        let mut notes = vec![
            "Keep the always-on footprint small enough for 24/7 use.".to_string(),
            "Load heavyweight inference backends on demand unless the device tier can sustain them."
                .to_string(),
        ];

        match tier {
            DeviceTier::Low => {
                resident_services.extend([
                    ResidentService::MicrophoneCapture,
                    ResidentService::VoiceActivityDetection,
                    ResidentService::DictationCleanup,
                ]);
                resident_models.push(ResidentModelPlan {
                    model_key: "wake-words".to_string(),
                    keep_loaded: true,
                    reason: "Wake-word latency must stay low on low-end devices.".to_string(),
                });
                resident_models.push(ResidentModelPlan {
                    model_key: "harper-grammar".to_string(),
                    keep_loaded: true,
                    reason: "Typing assistance should stay instant.".to_string(),
                });
                notes.push(
                    "On low-end machines keep LLM, STT, TTS, OCR, and VLM models unloaded until needed."
                        .to_string(),
                );
            }
            DeviceTier::Balanced => {
                resident_services.extend([
                    ResidentService::MicrophoneCapture,
                    ResidentService::VoiceActivityDetection,
                    ResidentService::DictationCleanup,
                    ResidentService::LocalSttWarmPool,
                ]);
                resident_models.push(ResidentModelPlan {
                    model_key: "moonshine-tiny".to_string(),
                    keep_loaded: true,
                    reason: "Fast dictation start is worth keeping a compact STT path warm."
                        .to_string(),
                });
                resident_models.push(ResidentModelPlan {
                    model_key: "harper-grammar".to_string(),
                    keep_loaded: true,
                    reason: "Typing assistance should remain instant.".to_string(),
                });
            }
            DeviceTier::Performance | DeviceTier::Workstation => {
                resident_services.extend([
                    ResidentService::MicrophoneCapture,
                    ResidentService::VoiceActivityDetection,
                    ResidentService::DictationCleanup,
                    ResidentService::LocalSttWarmPool,
                    ResidentService::LocalTtsWarmPool,
                    ResidentService::LocalLlmWarmPool,
                ]);
                resident_models.push(ResidentModelPlan {
                    model_key: "qwen3-0.6b".to_string(),
                    keep_loaded: true,
                    reason: "A compact assistant model can stay warm on stronger machines.".to_string(),
                });
                resident_models.push(ResidentModelPlan {
                    model_key: "moonshine-tiny".to_string(),
                    keep_loaded: true,
                    reason: "Keep STT warm for rapid dictation.".to_string(),
                });
                resident_models.push(ResidentModelPlan {
                    model_key: "kokoro-int8".to_string(),
                    keep_loaded: tier == DeviceTier::Workstation,
                    reason: "TTS can stay ready on stronger devices.".to_string(),
                });
                notes.push("High-tier devices can keep a small local assistant model warm.".to_string());
            }
        }

        AlwaysOnRuntimePlan {
            enabled: true,
            device_tier: tier,
            activation: broker.activation().clone(),
            resident_services,
            resident_models,
            battery_policy: match tier {
                DeviceTier::Low | DeviceTier::Balanced => BatteryPolicy::PauseHeavyServices,
                DeviceTier::Performance | DeviceTier::Workstation => BatteryPolicy::PreferEfficiency,
            },
            thermal_policy: match tier {
                DeviceTier::Workstation => ThermalPolicy::BackoffHeavyServices,
                _ => ThermalPolicy::SuspendLargeModels,
            },
            notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_end_plan_keeps_only_small_services_resident() {
        let broker = RuntimeBroker::detect();
        let plan = AlwaysOnRuntimeDirector::plan(&broker);
        assert!(plan.enabled);
        assert!(plan
            .resident_services
            .contains(&ResidentService::WakeWordMonitor));
    }
}
