use super::always_on::FlowDeviceTier;

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceBenchmarkSnapshot {
    pub ram_gb: f32,
    pub vram_gb: Option<f32>,
    pub average_prompt_latency_ms: u32,
    pub average_decode_tokens_per_sec: f32,
    pub battery_percent: Option<u8>,
    pub thermal_celsius: Option<u8>,
    pub cpu_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierAdjustment {
    Keep,
    Promote(FlowDeviceTier),
    Demote(FlowDeviceTier),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionRecommendation {
    pub current: FlowDeviceTier,
    pub adjustment: TierAdjustment,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowRuntimeTierPolicy {
    pub product_name: &'static str,
}

impl FlowRuntimeTierPolicy {
    pub fn new() -> Self {
        Self {
            product_name: "flow-runtime-tier-policy",
        }
    }

    pub fn baseline_tier(
        &self,
        ram_gb: f32,
        vram_gb: Option<f32>,
        cpu_only: bool,
    ) -> FlowDeviceTier {
        if ram_gb < 8.0 {
            return FlowDeviceTier::LowEnd;
        }

        if cpu_only && ram_gb < 16.0 {
            return FlowDeviceTier::Balanced;
        }

        if ram_gb >= 32.0 || vram_gb.unwrap_or(0.0) >= 10.0 {
            return FlowDeviceTier::Workstation;
        }

        if ram_gb >= 16.0 || vram_gb.unwrap_or(0.0) >= 6.0 {
            return FlowDeviceTier::Creator;
        }

        FlowDeviceTier::Balanced
    }

    pub fn evaluate(
        &self,
        current: FlowDeviceTier,
        benchmark: &DeviceBenchmarkSnapshot,
    ) -> PromotionRecommendation {
        if benchmark.ram_gb < 8.0
            || benchmark.average_prompt_latency_ms > 1_400
            || benchmark.average_decode_tokens_per_sec < 10.0
            || benchmark.thermal_celsius.unwrap_or(0) >= 82
            || benchmark.battery_percent.unwrap_or(100) <= 15
        {
            return PromotionRecommendation {
                current,
                adjustment: TierAdjustment::Demote(FlowDeviceTier::LowEnd),
                reason: "The device is currently constrained, so Flow should fall back to the low-end resident profile."
                    .to_string(),
            };
        }

        if benchmark.ram_gb >= 32.0
            && benchmark.average_prompt_latency_ms <= 700
            && benchmark.average_decode_tokens_per_sec >= 30.0
            && benchmark.vram_gb.unwrap_or(0.0) >= 10.0
        {
            return PromotionRecommendation {
                current,
                adjustment: TierAdjustment::Promote(FlowDeviceTier::Workstation),
                reason: "The device meets the high-end thresholds for workstation-grade local multimodal work."
                    .to_string(),
            };
        }

        if benchmark.ram_gb >= 16.0
            && benchmark.average_prompt_latency_ms <= 1_000
            && benchmark.average_decode_tokens_per_sec >= 18.0
        {
            return PromotionRecommendation {
                current,
                adjustment: TierAdjustment::Promote(FlowDeviceTier::Creator),
                reason:
                    "The device can handle creator-tier local models and richer overlay features."
                        .to_string(),
            };
        }

        if benchmark.ram_gb >= 8.0
            && benchmark.average_prompt_latency_ms <= 1_300
            && benchmark.average_decode_tokens_per_sec >= 12.0
        {
            return PromotionRecommendation {
                current,
                adjustment: TierAdjustment::Promote(FlowDeviceTier::Balanced),
                reason: "The device can sustain the balanced Flow profile for daily local use."
                    .to_string(),
            };
        }

        PromotionRecommendation {
            current,
            adjustment: TierAdjustment::Keep,
            reason: "The current tier remains the best fit for the measured device behavior."
                .to_string(),
        }
    }
}
