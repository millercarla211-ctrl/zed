use wasm_bindgen::prelude::*;

use crate::types::{
    BrowserCapabilityProfile, BrowserDeviceTarget, BrowserExecutionBackend, BrowserExecutionPlan,
    BrowserExecutionRequest, BrowserHostFlavor, BrowserModality, BrowserPackManifest,
    BrowserStorageBackend,
};

fn default_capabilities(flavor: BrowserHostFlavor) -> BrowserCapabilityProfile {
    match flavor {
        BrowserHostFlavor::ChromiumExtension => BrowserCapabilityProfile {
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
                "Chromium gets the richest Flow browser path.".to_string(),
                "This profile assumes WebGPU-ready local inference.".to_string(),
            ],
        },
        BrowserHostFlavor::FirefoxExtension => BrowserCapabilityProfile {
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
            notes: vec!["Firefox defaults to the local text baseline first.".to_string()],
        },
        BrowserHostFlavor::SafariWebExtension => BrowserCapabilityProfile {
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
                "Safari uses the same pack protocol with stricter capability gating.".to_string(),
            ],
        },
        BrowserHostFlavor::StandaloneWebApp => BrowserCapabilityProfile {
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
            notes: vec!["Standalone web mode reuses the extension runtime model.".to_string()],
        },
    }
}

fn default_pack_catalog() -> Vec<BrowserPackManifest> {
    vec![
        BrowserPackManifest {
            pack_key: "qwen3-0.6b-onnx-browserpack".to_string(),
            model_key: "qwen3-0.6b".to_string(),
            display_name: "Qwen3 0.6B Browser Pack".to_string(),
            repo_id: "onnx-community/Qwen3-0.6B-DQ-ONNX".to_string(),
            modality: BrowserModality::Chat,
            requires_webgpu: false,
            files: vec![
                "config.json".to_string(),
                "tokenizer.json".to_string(),
                "onnx/model_q4f16.onnx".to_string(),
            ],
        },
        BrowserPackManifest {
            pack_key: "trocr-small-printed-browserpack".to_string(),
            model_key: "trocr-small-printed".to_string(),
            display_name: "TrOCR Small Printed Browser Pack".to_string(),
            repo_id: "Xenova/trocr-small-printed".to_string(),
            modality: BrowserModality::Ocr,
            requires_webgpu: false,
            files: vec![
                "config.json".to_string(),
                "preprocessor_config.json".to_string(),
                "onnx/encoder_model_quantized.onnx".to_string(),
                "onnx/decoder_model_merged_quantized.onnx".to_string(),
            ],
        },
        BrowserPackManifest {
            pack_key: "qwen3.5-0.8b-onnx-browserpack".to_string(),
            model_key: "qwen3.5-0.8b".to_string(),
            display_name: "Qwen3.5 0.8B Browser Pack".to_string(),
            repo_id: "onnx-community/Qwen3.5-0.8B-ONNX".to_string(),
            modality: BrowserModality::VisionLanguage,
            requires_webgpu: true,
            files: vec![
                "config.json".to_string(),
                "processor_config.json".to_string(),
                "onnx/vision_encoder_fp16.onnx".to_string(),
                "onnx/decoder_model_merged_q4.onnx".to_string(),
            ],
        },
    ]
}

fn plan(request: BrowserExecutionRequest) -> BrowserExecutionPlan {
    let storage_backend = if request.capabilities.opfs {
        BrowserStorageBackend::Opfs
    } else if request.capabilities.indexeddb {
        BrowserStorageBackend::IndexedDb
    } else {
        BrowserStorageBackend::ExtensionStorage
    };

    match request.modality {
        BrowserModality::Chat => BrowserExecutionPlan {
            task: request.task,
            modality: request.modality,
            selected_model: Some("qwen3-0.6b".to_string()),
            pack_key: Some("qwen3-0.6b-onnx-browserpack".to_string()),
            backend: BrowserExecutionBackend::TransformersJsOnnx,
            storage_backend,
            device_target: if request.capabilities.webgpu {
                BrowserDeviceTarget::WebGpu
            } else {
                BrowserDeviceTarget::Wasm
            },
            remote_allowed: !request.local_only,
            reasons: vec!["Cross-browser local text is pinned to Qwen3 0.6B ONNX.".to_string()],
            unsupported_reason: None,
        },
        BrowserModality::Ocr => BrowserExecutionPlan {
            task: request.task,
            modality: request.modality,
            selected_model: Some("trocr-small-printed".to_string()),
            pack_key: Some("trocr-small-printed-browserpack".to_string()),
            backend: BrowserExecutionBackend::TransformersJsOnnx,
            storage_backend,
            device_target: if request.capabilities.webgpu {
                BrowserDeviceTarget::WebGpu
            } else {
                BrowserDeviceTarget::Wasm
            },
            remote_allowed: !request.local_only,
            reasons: vec!["OCR uses a cross-browser ONNX browser pack.".to_string()],
            unsupported_reason: None,
        },
        BrowserModality::VisionLanguage => BrowserExecutionPlan {
            task: request.task,
            modality: request.modality,
            selected_model: Some("qwen3.5-0.8b".to_string()),
            pack_key: Some("qwen3.5-0.8b-onnx-browserpack".to_string()),
            backend: BrowserExecutionBackend::TransformersJsOnnx,
            storage_backend,
            device_target: if request.capabilities.webgpu {
                BrowserDeviceTarget::WebGpu
            } else {
                BrowserDeviceTarget::Wasm
            },
            remote_allowed: !request.local_only,
            reasons: vec![
                "Multimodal local inference is gated on WebGPU-capable browser hardware."
                    .to_string(),
            ],
            unsupported_reason: (!request.capabilities.webgpu).then_some(
                "This browser profile does not expose WebGPU, so local multimodal is disabled."
                    .to_string(),
            ),
        },
    }
}

#[wasm_bindgen]
pub fn detect_browser_capabilities(flavor: JsValue) -> Result<JsValue, JsValue> {
    let flavor: BrowserHostFlavor =
        serde_wasm_bindgen::from_value(flavor).map_err(|err| JsValue::from(err.to_string()))?;
    serde_wasm_bindgen::to_value(&default_capabilities(flavor))
        .map_err(|err| JsValue::from(err.to_string()))
}

#[wasm_bindgen]
pub fn default_browser_pack_catalog_json() -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&default_pack_catalog())
        .map_err(|err| JsValue::from(err.to_string()))
}

#[wasm_bindgen]
pub fn plan_browser_execution(request: JsValue) -> Result<JsValue, JsValue> {
    let request: BrowserExecutionRequest =
        serde_wasm_bindgen::from_value(request).map_err(|err| JsValue::from(err.to_string()))?;
    serde_wasm_bindgen::to_value(&plan(request)).map_err(|err| JsValue::from(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BrowserTask;

    #[test]
    fn plans_text_baseline() {
        let plan = plan(BrowserExecutionRequest {
            task: BrowserTask::RewriteSelection,
            modality: BrowserModality::Chat,
            local_only: true,
            preferred_model: None,
            capabilities: default_capabilities(BrowserHostFlavor::FirefoxExtension),
        });

        assert_eq!(plan.selected_model.as_deref(), Some("qwen3-0.6b"));
        assert!(plan.unsupported_reason.is_none());
    }
}
