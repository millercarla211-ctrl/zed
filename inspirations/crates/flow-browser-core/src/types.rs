use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserHostFlavor {
    ChromiumExtension,
    FirefoxExtension,
    SafariWebExtension,
    StandaloneWebApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserModality {
    Chat,
    Ocr,
    VisionLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserTask {
    RewriteSelection,
    SummarizePage,
    ComposeDraft,
    ExplainPage,
    OcrImage,
    MultimodalAsk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserExecutionBackend {
    TransformersJsOnnx,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserStorageBackend {
    Opfs,
    IndexedDb,
    ExtensionStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserDeviceTarget {
    WebGpu,
    Wasm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserCapabilityProfile {
    pub flavor: BrowserHostFlavor,
    pub webgpu: bool,
    pub wasm_threads: bool,
    pub cross_origin_isolated: bool,
    pub opfs: bool,
    pub indexeddb: bool,
    pub side_panel: bool,
    pub sidebar_action: bool,
    pub offscreen_document: bool,
    pub background_service_worker: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserPackManifest {
    pub pack_key: String,
    pub model_key: String,
    pub display_name: String,
    pub repo_id: String,
    pub modality: BrowserModality,
    pub requires_webgpu: bool,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserExecutionRequest {
    pub task: BrowserTask,
    pub modality: BrowserModality,
    pub local_only: bool,
    pub preferred_model: Option<String>,
    pub capabilities: BrowserCapabilityProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserExecutionPlan {
    pub task: BrowserTask,
    pub modality: BrowserModality,
    pub selected_model: Option<String>,
    pub pack_key: Option<String>,
    pub backend: BrowserExecutionBackend,
    pub storage_backend: BrowserStorageBackend,
    pub device_target: BrowserDeviceTarget,
    pub remote_allowed: bool,
    pub reasons: Vec<String>,
    pub unsupported_reason: Option<String>,
}
