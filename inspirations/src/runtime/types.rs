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
pub enum DeviceTier {
    Low,
    Balanced,
    Performance,
    Workstation,
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
pub enum ComputeBackend {
    Cpu,
    Cuda,
    Vulkan,
    Metal,
    Rocm,
    DirectMl,
    CoreMl,
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
pub enum Modality {
    Chat,
    Text,
    UiGeneration,
    VisionLanguage,
    SpeechToText,
    TextToSpeech,
    Ocr,
    ImageGeneration,
    VideoGeneration,
    WakeWord,
    Grammar,
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
pub enum ArtifactFormat {
    Gguf,
    Onnx,
    Safetensors,
    Json,
    Binary,
    Unknown,
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
pub enum RuntimeKind {
    LlamaCppEmbedded,
    MistralRsEmbedded,
    CandleEmbedded,
    OnnxRuntimeEmbedded,
    SherpaOnnxEmbedded,
    WhisperCppSubprocess,
    StableDiffusionCppSubprocess,
    HarperCoreEmbedded,
    PythonWorker,
    ConversionOrchestrator,
    Unsupported,
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
pub enum RuntimeLaunch {
    Embedded,
    Subprocess,
    Conversion,
    PublishableArtifact,
    Unsupported,
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
pub enum ConversionLane {
    Gguf,
    Onnx,
    NativeSafetensors,
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
pub enum PublishStatus {
    Planned,
    Validated,
    Published,
    Refused,
    LocalOnly,
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
pub struct GraphicsDevice {
    pub name: String,
    pub vendor: Option<String>,
    pub vram_bytes: Option<u64>,
    pub integrated: bool,
    pub backends: Vec<ComputeBackend>,
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
pub struct DeviceProfile {
    pub os: String,
    pub arch: String,
    pub cpu_model: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub battery_powered: Option<bool>,
    pub thermal_class: Option<String>,
    pub graphics: Vec<GraphicsDevice>,
    pub tier: DeviceTier,
}

#[derive(Debug, Clone, PartialEq, Eq, SerdeSerialize, SerdeDeserialize)]
pub struct ModelManifest {
    pub key: String,
    pub display_name: String,
    pub family: String,
    pub repo_id: String,
    pub modality: Modality,
    pub local_path: Option<String>,
    pub artifact_format: ArtifactFormat,
    pub preferred_runtime: RuntimeKind,
    pub fallback_runtimes: Vec<RuntimeKind>,
    pub conversion_lanes: Vec<ConversionLane>,
    pub minimum_memory_bytes: u64,
    pub quantization: Option<String>,
    pub license: Option<String>,
    pub redistributable: bool,
    pub gated: bool,
    pub local_only: bool,
    pub tags: Vec<String>,
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
pub struct ArtifactFile {
    pub path: String,
    pub bytes: Option<u64>,
    pub sha256: Option<String>,
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
pub struct ArtifactBundle {
    pub model_key: String,
    pub upstream_repo: String,
    pub upstream_revision: Option<String>,
    pub root_dir: String,
    pub artifact_format: ArtifactFormat,
    pub quantization: Option<String>,
    pub license: Option<String>,
    pub runtime: RuntimeKind,
    pub files: Vec<ArtifactFile>,
    pub redistributable: bool,
    pub gated: bool,
    pub local_only: bool,
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
pub struct BenchmarkRecord {
    pub model_key: String,
    pub runtime: RuntimeKind,
    pub modality: Modality,
    pub load_time_ms: u64,
    pub tokens_per_second: Option<u64>,
    pub samples_per_second: Option<u64>,
    pub measured_at_unix_ms: u64,
    pub device_tier: DeviceTier,
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
pub struct ConversionJob {
    pub model_key: String,
    pub source_repo: String,
    pub lane: ConversionLane,
    pub target_format: ArtifactFormat,
    pub command_preview: Vec<String>,
    pub publish_after_validation: bool,
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
pub struct PublishRecord {
    pub model_key: String,
    pub destination_repo: String,
    pub status: PublishStatus,
    pub reason: Option<String>,
    pub checksum: Option<String>,
    pub verified: bool,
    pub local_only: bool,
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
pub struct WakeWordConfigItem {
    pub command_key: String,
    pub phrase: String,
    pub model_path: String,
    pub threshold: u32,
    pub aliases: Vec<String>,
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
pub struct KeyboardShortcut {
    pub modifiers: Vec<String>,
    pub key: String,
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
pub struct ActivationConfig {
    pub wake_words: Vec<WakeWordConfigItem>,
    pub push_to_talk: KeyboardShortcut,
    pub hands_free_toggle: KeyboardShortcut,
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
pub struct ExecutionPlan {
    pub modality: Modality,
    pub requested_model: Option<String>,
    pub selected_model: Option<String>,
    pub selected_runtime: Option<RuntimeKind>,
    pub launch: RuntimeLaunch,
    pub device_tier: DeviceTier,
    pub estimated_memory_bytes: Option<u64>,
    pub reasons: Vec<String>,
    pub artifact: Option<ArtifactBundle>,
    pub conversion_job: Option<ConversionJob>,
    pub publish_record: Option<PublishRecord>,
    pub unsupported_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerRequest {
    pub modality: Modality,
    pub preferred_model: Option<String>,
    pub allow_conversion: bool,
    pub allow_publish: bool,
}

impl BrokerRequest {
    pub fn new(modality: Modality) -> Self {
        Self {
            modality,
            preferred_model: None,
            allow_conversion: true,
            allow_publish: true,
        }
    }

    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.preferred_model = model;
        self
    }
}
