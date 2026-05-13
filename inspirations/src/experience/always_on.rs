#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowDeviceTier {
    LowEnd,
    Balanced,
    Creator,
    Workstation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerStrategy {
    Minimal,
    Balanced,
    Performance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidentLane {
    WakeWord,
    Dictation,
    TypingAssist,
    ClipboardAssist,
    Overlay,
    LocalChat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentModelPlan {
    pub model_id: &'static str,
    pub role: &'static str,
    pub quantization: &'static str,
    pub load_strategy: &'static str,
    pub target_context: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentModelBudget {
    pub max_total_ram_mb: u64,
    pub max_hot_model_ram_mb: u64,
    pub unload_after_idle_secs: u64,
    pub warm_prompt_cache_mb: u64,
    pub allow_gpu_residency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlwaysOnFeatureSet {
    pub wake_word_detection: bool,
    pub instant_dictation: bool,
    pub live_grammar: bool,
    pub local_rewrite: bool,
    pub background_summaries: bool,
    pub passive_vad: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryGuard {
    pub minimum_percent_for_full_mode: u8,
    pub pause_background_summaries_below_percent: u8,
    pub force_minimal_mode_on_battery_save: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermalGuard {
    pub backoff_temperature_celsius: u8,
    pub disable_overlay_temperature_celsius: u8,
    pub unload_multimodal_temperature_celsius: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowAlwaysOnProfile {
    pub tier: FlowDeviceTier,
    pub power_strategy: PowerStrategy,
    pub resident_lanes: Vec<ResidentLane>,
    pub resident_models: Vec<ResidentModelPlan>,
    pub budget: ResidentModelBudget,
    pub battery: BatteryGuard,
    pub thermal: ThermalGuard,
    pub features: AlwaysOnFeatureSet,
}

impl FlowAlwaysOnProfile {
    pub fn low_end_24_7() -> Self {
        Self {
            tier: FlowDeviceTier::LowEnd,
            power_strategy: PowerStrategy::Minimal,
            resident_lanes: vec![
                ResidentLane::WakeWord,
                ResidentLane::Dictation,
                ResidentLane::TypingAssist,
                ResidentLane::Overlay,
            ],
            resident_models: vec![
                ResidentModelPlan {
                    model_id: "qwen3-0.6b",
                    role: "instant local rewrite",
                    quantization: "Q4_K_M",
                    load_strategy: "warm-on-focus",
                    target_context: 8192,
                },
                ResidentModelPlan {
                    model_id: "moonshine-tiny",
                    role: "on-demand speech recognition",
                    quantization: "int8",
                    load_strategy: "wake-on-demand",
                    target_context: 0,
                },
                ResidentModelPlan {
                    model_id: "kokoro-onnx-int8",
                    role: "quick voice confirmation",
                    quantization: "int8",
                    load_strategy: "cold-start",
                    target_context: 0,
                },
            ],
            budget: ResidentModelBudget {
                max_total_ram_mb: 2_048,
                max_hot_model_ram_mb: 768,
                unload_after_idle_secs: 24,
                warm_prompt_cache_mb: 96,
                allow_gpu_residency: false,
            },
            battery: BatteryGuard {
                minimum_percent_for_full_mode: 55,
                pause_background_summaries_below_percent: 40,
                force_minimal_mode_on_battery_save: true,
            },
            thermal: ThermalGuard {
                backoff_temperature_celsius: 72,
                disable_overlay_temperature_celsius: 78,
                unload_multimodal_temperature_celsius: 75,
            },
            features: AlwaysOnFeatureSet {
                wake_word_detection: true,
                instant_dictation: true,
                live_grammar: true,
                local_rewrite: true,
                background_summaries: false,
                passive_vad: true,
            },
        }
    }

    pub fn balanced_desktop() -> Self {
        Self {
            tier: FlowDeviceTier::Balanced,
            power_strategy: PowerStrategy::Balanced,
            resident_lanes: vec![
                ResidentLane::WakeWord,
                ResidentLane::Dictation,
                ResidentLane::TypingAssist,
                ResidentLane::ClipboardAssist,
                ResidentLane::Overlay,
                ResidentLane::LocalChat,
            ],
            resident_models: vec![
                ResidentModelPlan {
                    model_id: "smollm3-3b",
                    role: "typing rewrite and command mode",
                    quantization: "Q4_K_M",
                    load_strategy: "warm-resident",
                    target_context: 16_384,
                },
                ResidentModelPlan {
                    model_id: "parakeet-tdt-0.6b-v3-int8",
                    role: "higher-quality on-demand speech recognition",
                    quantization: "int8",
                    load_strategy: "wake-on-demand",
                    target_context: 0,
                },
                ResidentModelPlan {
                    model_id: "gemma-4-e2b",
                    role: "multimodal overlay assist",
                    quantization: "Q4",
                    load_strategy: "warm-on-overlay",
                    target_context: 16_384,
                },
            ],
            budget: ResidentModelBudget {
                max_total_ram_mb: 6_144,
                max_hot_model_ram_mb: 2_048,
                unload_after_idle_secs: 45,
                warm_prompt_cache_mb: 256,
                allow_gpu_residency: true,
            },
            battery: BatteryGuard {
                minimum_percent_for_full_mode: 35,
                pause_background_summaries_below_percent: 25,
                force_minimal_mode_on_battery_save: true,
            },
            thermal: ThermalGuard {
                backoff_temperature_celsius: 76,
                disable_overlay_temperature_celsius: 82,
                unload_multimodal_temperature_celsius: 80,
            },
            features: AlwaysOnFeatureSet {
                wake_word_detection: true,
                instant_dictation: true,
                live_grammar: true,
                local_rewrite: true,
                background_summaries: true,
                passive_vad: true,
            },
        }
    }
}
