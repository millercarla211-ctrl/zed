use crate::runtime::{BrokerRequest, Modality, RuntimeBroker};

use super::catalog::default_browser_pack_catalog;
use super::types::{
    BrowserCapabilityProfile, BrowserDeviceTarget, BrowserExecutionBackend, BrowserExecutionPlan,
    BrowserExecutionRequest, BrowserExtensionMessage, BrowserHostFlavor,
    BrowserInferenceInvocation, BrowserInferenceRequest, BrowserPackManifest,
    BrowserPackResolution, BrowserStorageBackend, BrowserTask, BrowserTokenStreamPlan,
    BrowserUiSurface, BrowserWorkerKind,
};

pub struct FlowBrowserEngine {
    broker: RuntimeBroker,
    pack_catalog: Vec<BrowserPackManifest>,
}

impl FlowBrowserEngine {
    pub fn detect() -> Self {
        Self {
            broker: RuntimeBroker::detect(),
            pack_catalog: default_browser_pack_catalog(),
        }
    }

    pub fn from_broker(broker: RuntimeBroker) -> Self {
        Self {
            broker,
            pack_catalog: default_browser_pack_catalog(),
        }
    }

    pub fn broker(&self) -> &RuntimeBroker {
        &self.broker
    }

    pub fn pack_catalog(&self) -> &[BrowserPackManifest] {
        &self.pack_catalog
    }

    pub fn detect_browser_capabilities(
        &self,
        flavor: BrowserHostFlavor,
        webgpu: Option<bool>,
        wasm_threads: Option<bool>,
        cross_origin_isolated: Option<bool>,
        opfs: Option<bool>,
        indexeddb: Option<bool>,
    ) -> BrowserCapabilityProfile {
        let mut capabilities = BrowserCapabilityProfile::baseline(flavor);

        if let Some(value) = webgpu {
            capabilities.webgpu = value;
        }
        if let Some(value) = wasm_threads {
            capabilities.wasm_threads = value;
        }
        if let Some(value) = cross_origin_isolated {
            capabilities.cross_origin_isolated = value;
        }
        if let Some(value) = opfs {
            capabilities.opfs = value;
        }
        if let Some(value) = indexeddb {
            capabilities.indexeddb = value;
        }

        if capabilities.wasm_threads && !capabilities.cross_origin_isolated {
            capabilities.wasm_threads = false;
            capabilities.notes.push(
                "WASM threading was disabled because cross-origin isolation is not enabled."
                    .to_string(),
            );
        }

        capabilities
    }

    pub fn plan_browser_execution(&self, request: BrowserExecutionRequest) -> BrowserExecutionPlan {
        self.broker.build_browser_plan(request, &self.pack_catalog)
    }

    pub fn ensure_browser_model_pack(&self, model_key: &str) -> BrowserPackResolution {
        let pack = self
            .pack_catalog
            .iter()
            .find(|pack| pack.model_key == model_key)
            .cloned();

        BrowserPackResolution {
            model_key: model_key.to_string(),
            download_required: pack.is_some(),
            reason: pack
                .as_ref()
                .map(|pack| {
                    format!(
                        "Browser pack '{}' should be downloaded into OPFS or IndexedDB before local inference.",
                        pack.pack_key
                    )
                })
                .or_else(|| Some("No browser pack is registered for this model.".to_string())),
            pack,
        }
    }

    pub fn run_browser_inference(
        &self,
        request: BrowserInferenceRequest,
    ) -> BrowserInferenceInvocation {
        let plan = self.plan_browser_execution(BrowserExecutionRequest {
            task: request.task,
            modality: request.modality,
            local_only: request.local_only,
            preferred_model: request.preferred_model.clone(),
            allow_remote_fallback: !request.local_only,
            capabilities: request.capabilities.clone(),
        });

        let message = match request.task {
            BrowserTask::RewriteSelection => BrowserExtensionMessage::RewriteSelection {
                prompt: request.prompt.clone(),
                selection_text: request.selection_text.clone().unwrap_or_default(),
            },
            BrowserTask::SummarizeSelection | BrowserTask::SummarizePage => {
                BrowserExtensionMessage::SummarizePage {
                    page_text: request
                        .page_text
                        .clone()
                        .or(request.selection_text.clone())
                        .unwrap_or_default(),
                }
            }
            BrowserTask::ComposeDraft => BrowserExtensionMessage::ComposeDraft {
                prompt: request.prompt.clone(),
                context_text: request.page_text.clone().or(request.selection_text.clone()),
            },
            BrowserTask::ExplainPage => BrowserExtensionMessage::ExplainPage {
                page_text: request.page_text.clone().unwrap_or_default(),
            },
            BrowserTask::OcrImage => BrowserExtensionMessage::OcrImage {
                image_sources: request.image_sources.clone(),
                prompt: (!request.prompt.is_empty()).then_some(request.prompt.clone()),
            },
            BrowserTask::MultimodalAsk => BrowserExtensionMessage::MultimodalAsk {
                prompt: request.prompt.clone(),
                image_sources: request.image_sources.clone(),
            },
        };

        BrowserInferenceInvocation {
            stream_supported: matches!(
                plan.backend,
                BrowserExecutionBackend::TransformersJsOnnx | BrowserExecutionBackend::WebLlmWorker
            ) && matches!(
                request.modality,
                Modality::Chat | Modality::Text | Modality::Grammar | Modality::VisionLanguage
            ),
            plan,
            request,
            message,
        }
    }

    pub fn stream_browser_tokens(
        &self,
        _request: &BrowserInferenceRequest,
    ) -> BrowserTokenStreamPlan {
        BrowserTokenStreamPlan {
            channel: "flow.browser.tokens".to_string(),
            chunk_event: "stream_chunk".to_string(),
            done_event: "stream_done".to_string(),
            error_event: "stream_error".to_string(),
        }
    }
}

impl RuntimeBroker {
    pub fn build_browser_plan(
        &self,
        request: BrowserExecutionRequest,
        pack_catalog: &[BrowserPackManifest],
    ) -> BrowserExecutionPlan {
        let storage_backend = preferred_storage_backend(&request.capabilities);
        let ui_surfaces = preferred_ui_surfaces(&request.capabilities);
        let mut reasons = vec![format!(
            "Browser host {:?} is using a capability-tiered local execution path.",
            request.capabilities.flavor
        )];
        let mut enabled_features = Vec::new();
        let mut disabled_features = Vec::new();
        let mut unsupported_reason = None;

        let broker_request =
            BrokerRequest::new(request.modality).with_model(request.preferred_model);
        let broker_plan = self.build_plan(broker_request);

        let mut selected_model = broker_plan.selected_model.clone();
        let mut selected_pack = selected_model
            .as_ref()
            .and_then(|model_key| {
                pack_catalog
                    .iter()
                    .find(|pack| &pack.model_key == model_key)
            })
            .cloned();

        if matches!(
            request.modality,
            Modality::Chat | Modality::Text | Modality::Grammar
        ) {
            selected_model = Some("qwen3-0.6b".to_string());
            selected_pack = pack_catalog
                .iter()
                .find(|pack| pack.model_key == "qwen3-0.6b")
                .cloned();
        } else if matches!(request.modality, Modality::Ocr) {
            selected_model = Some("trocr-small-printed".to_string());
            selected_pack = pack_catalog
                .iter()
                .find(|pack| pack.model_key == "trocr-small-printed")
                .cloned();
        } else if matches!(request.modality, Modality::VisionLanguage) {
            selected_model = Some("qwen3.5-0.8b".to_string());
            selected_pack = pack_catalog
                .iter()
                .find(|pack| pack.model_key == "qwen3.5-0.8b")
                .cloned();
        }

        let device_target = if request.capabilities.webgpu {
            Some(BrowserDeviceTarget::WebGpu)
        } else {
            Some(BrowserDeviceTarget::Wasm)
        };

        let worker_kind = if request.capabilities.background_service_worker {
            Some(BrowserWorkerKind::BackgroundServiceWorker)
        } else if matches!(
            request.capabilities.flavor,
            BrowserHostFlavor::FirefoxExtension | BrowserHostFlavor::SafariWebExtension
        ) {
            Some(BrowserWorkerKind::BackgroundDocument)
        } else {
            Some(BrowserWorkerKind::DedicatedWorker)
        };

        if request.capabilities.webgpu {
            enabled_features.push("webgpu".to_string());
        } else {
            disabled_features.push("webgpu".to_string());
            reasons.push(
                "WebGPU is unavailable, so Flow will use browser CPU/WASM execution.".to_string(),
            );
        }

        if request.capabilities.wasm_threads {
            enabled_features.push("wasm-threads".to_string());
        } else {
            disabled_features.push("wasm-threads".to_string());
        }

        if !request.capabilities.opfs {
            disabled_features.push("opfs".to_string());
            reasons.push(
                "OPFS is unavailable; IndexedDB or extension storage will be used.".to_string(),
            );
        } else {
            enabled_features.push("opfs".to_string());
        }

        if matches!(request.modality, Modality::VisionLanguage) && !request.capabilities.webgpu {
            unsupported_reason = Some(
                "Local multimodal execution is disabled because this browser profile lacks WebGPU."
                    .to_string(),
            );
            disabled_features.push("local-multimodal".to_string());
        } else if matches!(request.modality, Modality::VisionLanguage) {
            enabled_features.push("local-multimodal".to_string());
            reasons.push(
                "Multimodal inference is enabled because WebGPU-capable browser hardware was detected."
                    .to_string(),
            );
        }

        if matches!(request.modality, Modality::Ocr) {
            enabled_features.push("ocr".to_string());
            reasons.push(
                "OCR is mapped to a cross-browser ONNX pack so screenshots and document crops work locally."
                    .to_string(),
            );
        }

        if matches!(
            request.modality,
            Modality::Chat | Modality::Text | Modality::Grammar
        ) {
            enabled_features.push("local-text".to_string());
            reasons.push(
                "Cross-browser local text inference is locked to the Qwen3 0.6B ONNX browser pack."
                    .to_string(),
            );
        }

        if request.local_only {
            reasons.push(
                "Local-only mode is enabled, so Flow must not silently fall back to remote inference."
                    .to_string(),
            );
        }

        let backend = selected_pack
            .as_ref()
            .map(|pack| pack.backend)
            .unwrap_or(BrowserExecutionBackend::Unsupported);

        if unsupported_reason.is_none() && selected_pack.is_none() {
            unsupported_reason =
                Some("No browser-ready pack is registered for this request.".to_string());
        }

        BrowserExecutionPlan {
            task: request.task,
            modality: request.modality,
            selected_model,
            pack_key: selected_pack.as_ref().map(|pack| pack.pack_key.clone()),
            backend,
            storage_backend,
            worker_kind,
            device_target,
            ui_surfaces,
            enabled_features,
            disabled_features,
            reasons,
            local_only: request.local_only,
            remote_allowed: !request.local_only && request.allow_remote_fallback,
            unsupported_reason,
        }
    }
}

fn preferred_storage_backend(capabilities: &BrowserCapabilityProfile) -> BrowserStorageBackend {
    if capabilities.opfs {
        BrowserStorageBackend::Opfs
    } else if capabilities.indexeddb {
        BrowserStorageBackend::IndexedDb
    } else {
        BrowserStorageBackend::ExtensionStorage
    }
}

fn preferred_ui_surfaces(capabilities: &BrowserCapabilityProfile) -> Vec<BrowserUiSurface> {
    let mut surfaces = vec![
        BrowserUiSurface::Popup,
        BrowserUiSurface::OptionsPage,
        BrowserUiSurface::ContentOverlay,
    ];

    if capabilities.side_panel {
        surfaces.push(BrowserUiSurface::SidePanel);
    }

    if capabilities.sidebar_action {
        surfaces.push(BrowserUiSurface::SidebarAction);
    }

    if capabilities.offscreen_document {
        surfaces.push(BrowserUiSurface::OffscreenDocument);
    }

    surfaces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_cross_browser_text_baseline() {
        let engine = FlowBrowserEngine::detect();
        let capabilities = engine.detect_browser_capabilities(
            BrowserHostFlavor::FirefoxExtension,
            None,
            None,
            None,
            None,
            None,
        );

        let plan = engine.plan_browser_execution(BrowserExecutionRequest {
            task: BrowserTask::RewriteSelection,
            modality: Modality::Chat,
            local_only: true,
            preferred_model: None,
            allow_remote_fallback: false,
            capabilities,
        });

        assert_eq!(plan.selected_model.as_deref(), Some("qwen3-0.6b"));
        assert!(plan.pack_key.is_some());
        assert!(plan.local_only);
    }

    #[test]
    fn gates_multimodal_without_webgpu() {
        let engine = FlowBrowserEngine::detect();
        let capabilities = engine.detect_browser_capabilities(
            BrowserHostFlavor::SafariWebExtension,
            Some(false),
            Some(false),
            Some(false),
            Some(true),
            Some(true),
        );

        let plan = engine.plan_browser_execution(BrowserExecutionRequest {
            task: BrowserTask::MultimodalAsk,
            modality: Modality::VisionLanguage,
            local_only: true,
            preferred_model: None,
            allow_remote_fallback: false,
            capabilities,
        });

        assert!(plan.unsupported_reason.is_some());
    }
}
