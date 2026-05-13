use super::{
    activation::FlowActivationProfile, always_on::FlowAlwaysOnProfile,
    modules::OperatingSystemFamily,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioBackend {
    Cpal,
    PlatformNative,
    BrowserMediaDevices,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakePipelinePlan {
    pub model_hint: &'static str,
    pub keep_hot: bool,
    pub frame_ms: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictationPipelinePlan {
    pub stt_model_hint: &'static str,
    pub vad_hint: &'static str,
    pub punctuation: bool,
    pub streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowAudioPipeline {
    pub backend: AudioBackend,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub wake: WakePipelinePlan,
    pub dictation: DictationPipelinePlan,
    pub low_power_mode: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowAudioPlanner;

impl FlowAudioPlanner {
    pub fn build(
        os: &OperatingSystemFamily,
        activation: &FlowActivationProfile,
        always_on: &FlowAlwaysOnProfile,
    ) -> FlowAudioPipeline {
        let backend = match os {
            OperatingSystemFamily::BrowserWasm => AudioBackend::BrowserMediaDevices,
            OperatingSystemFamily::Android
            | OperatingSystemFamily::Ios
            | OperatingSystemFamily::Macos => AudioBackend::PlatformNative,
            _ => AudioBackend::Cpal,
        };

        let low_power_mode = matches!(always_on.tier, super::always_on::FlowDeviceTier::LowEnd);
        let stt_model_hint = if low_power_mode {
            "moonshine-tiny"
        } else {
            "parakeet-tdt-0.6b-v3-int8"
        };

        FlowAudioPipeline {
            backend,
            sample_rate_hz: 16_000,
            channels: 1,
            wake: WakePipelinePlan {
                model_hint: activation
                    .model_sources
                    .first()
                    .map(|model| model.label)
                    .unwrap_or("openwake-primary"),
                keep_hot: activation.keep_microphone_hot,
                frame_ms: 20,
            },
            dictation: DictationPipelinePlan {
                stt_model_hint,
                vad_hint: "webrtc-vad",
                punctuation: true,
                streaming: true,
            },
            low_power_mode,
        }
    }
}
