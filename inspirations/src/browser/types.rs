use serde::{Deserialize, Serialize};

use crate::runtime::Modality;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserHostFlavor {
    ChromiumExtension,
    FirefoxExtension,
    SafariWebExtension,
    StandaloneWebApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserExecutionBackend {
    TransformersJsOnnx,
    OnnxRuntimeWeb,
    WebLlmWorker,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserStorageBackend {
    Opfs,
    IndexedDb,
    ExtensionStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserUiSurface {
    Popup,
    OptionsPage,
    ContentOverlay,
    SidePanel,
    SidebarAction,
    OffscreenDocument,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserWorkerKind {
    DedicatedWorker,
    BackgroundServiceWorker,
    BackgroundDocument,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserDeviceTarget {
    WebGpu,
    Wasm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowserTask {
    RewriteSelection,
    SummarizeSelection,
    SummarizePage,
    ComposeDraft,
    ExplainPage,
    OcrImage,
    MultimodalAsk,
}

impl BrowserTask {
    pub fn label(&self) -> &'static str {
        match self {
            Self::RewriteSelection => "rewrite-selection",
            Self::SummarizeSelection => "summarize-selection",
            Self::SummarizePage => "summarize-page",
            Self::ComposeDraft => "compose-draft",
            Self::ExplainPage => "explain-page",
            Self::OcrImage => "ocr-image",
            Self::MultimodalAsk => "multimodal-ask",
        }
    }
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

impl BrowserCapabilityProfile {
    pub fn baseline(flavor: BrowserHostFlavor) -> Self {
        match flavor {
            BrowserHostFlavor::ChromiumExtension => Self {
                flavor,
                webgpu: true,
                wasm_threads: true,
                cross_origin_isolated: true,
                opfs: true,
                indexeddb: true,
                side_panel: true,
                sidebar_action: false,
                offscreen_document: true,
                background_service_worker: true,
                notes: vec![
                    "Chromium is the richest browser target for Flow local inference.".to_string(),
                    "WebGPU and service-worker-assisted extension flows are expected to be available."
                        .to_string(),
                ],
            },
            BrowserHostFlavor::FirefoxExtension => Self {
                flavor,
                webgpu: false,
                wasm_threads: false,
                cross_origin_isolated: false,
                opfs: true,
                indexeddb: true,
                side_panel: false,
                sidebar_action: true,
                offscreen_document: false,
                background_service_worker: false,
                notes: vec![
                    "Firefox ships the text-local baseline first.".to_string(),
                    "Sidebar-based UI is preferred over Chromium sidePanel.".to_string(),
                ],
            },
            BrowserHostFlavor::SafariWebExtension => Self {
                flavor,
                webgpu: false,
                wasm_threads: false,
                cross_origin_isolated: false,
                opfs: true,
                indexeddb: true,
                side_panel: false,
                sidebar_action: false,
                offscreen_document: false,
                background_service_worker: false,
                notes: vec![
                    "Safari should ship the same WebExtension product with stricter capability gating."
                        .to_string(),
                    "Popup and content overlay are the default UI surfaces for Safari packaging."
                        .to_string(),
                ],
            },
            BrowserHostFlavor::StandaloneWebApp => Self {
                flavor,
                webgpu: true,
                wasm_threads: true,
                cross_origin_isolated: true,
                opfs: true,
                indexeddb: true,
                side_panel: false,
                sidebar_action: false,
                offscreen_document: false,
                background_service_worker: true,
                notes: vec![
                    "Standalone web app mode can reuse the browser runtime without extension APIs."
                        .to_string(),
                ],
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserPackFile {
    pub path: String,
    pub source_url: String,
    pub bytes: Option<u64>,
    pub sha256: Option<String>,
    pub required: bool,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserPackSupport {
    pub chromium: bool,
    pub firefox: bool,
    pub safari: bool,
    pub standalone_web: bool,
    pub requires_webgpu: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserPackManifest {
    pub version: u32,
    pub pack_key: String,
    pub model_key: String,
    pub display_name: String,
    pub repo_id: String,
    pub modality: Modality,
    pub backend: BrowserExecutionBackend,
    pub quantization: Option<String>,
    pub tokenizer: Option<String>,
    pub processor: Option<String>,
    pub browser_support: BrowserPackSupport,
    pub files: Vec<BrowserPackFile>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserExecutionRequest {
    pub task: BrowserTask,
    pub modality: Modality,
    pub local_only: bool,
    pub preferred_model: Option<String>,
    pub allow_remote_fallback: bool,
    pub capabilities: BrowserCapabilityProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserExecutionPlan {
    pub task: BrowserTask,
    pub modality: Modality,
    pub selected_model: Option<String>,
    pub pack_key: Option<String>,
    pub backend: BrowserExecutionBackend,
    pub storage_backend: BrowserStorageBackend,
    pub worker_kind: Option<BrowserWorkerKind>,
    pub device_target: Option<BrowserDeviceTarget>,
    pub ui_surfaces: Vec<BrowserUiSurface>,
    pub enabled_features: Vec<String>,
    pub disabled_features: Vec<String>,
    pub reasons: Vec<String>,
    pub local_only: bool,
    pub remote_allowed: bool,
    pub unsupported_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserPackResolution {
    pub model_key: String,
    pub pack: Option<BrowserPackManifest>,
    pub download_required: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserInferenceRequest {
    pub task: BrowserTask,
    pub modality: Modality,
    pub prompt: String,
    pub selection_text: Option<String>,
    pub page_text: Option<String>,
    pub image_sources: Vec<String>,
    pub messages: Vec<BrowserChatMessage>,
    pub local_only: bool,
    pub preferred_model: Option<String>,
    pub capabilities: BrowserCapabilityProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserInferenceInvocation {
    pub plan: BrowserExecutionPlan,
    pub request: BrowserInferenceRequest,
    pub message: BrowserExtensionMessage,
    pub stream_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTokenStreamPlan {
    pub channel: String,
    pub chunk_event: String,
    pub done_event: String,
    pub error_event: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrowserExtensionMessage {
    DetectCapabilities,
    Capabilities {
        capabilities: BrowserCapabilityProfile,
    },
    EnsurePack {
        pack_key: String,
    },
    RunInference {
        request: BrowserInferenceRequest,
    },
    RewriteSelection {
        prompt: String,
        selection_text: String,
    },
    SummarizePage {
        page_text: String,
    },
    ComposeDraft {
        prompt: String,
        context_text: Option<String>,
    },
    ExplainPage {
        page_text: String,
    },
    OcrImage {
        image_sources: Vec<String>,
        prompt: Option<String>,
    },
    MultimodalAsk {
        prompt: String,
        image_sources: Vec<String>,
    },
    StreamChunk {
        delta: String,
    },
    StreamDone,
    Error {
        message: String,
    },
}
