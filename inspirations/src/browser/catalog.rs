use crate::runtime::Modality;

use super::types::{
    BrowserExecutionBackend, BrowserPackFile, BrowserPackManifest, BrowserPackSupport,
};

fn pack(
    pack_key: &str,
    model_key: &str,
    display_name: &str,
    repo_id: &str,
    modality: Modality,
    backend: BrowserExecutionBackend,
    quantization: Option<&str>,
    tokenizer: Option<&str>,
    processor: Option<&str>,
    browser_support: BrowserPackSupport,
    files: &[(&str, &str, Option<u64>, &str)],
    tags: &[&str],
) -> BrowserPackManifest {
    BrowserPackManifest {
        version: 1,
        pack_key: pack_key.to_string(),
        model_key: model_key.to_string(),
        display_name: display_name.to_string(),
        repo_id: repo_id.to_string(),
        modality,
        backend,
        quantization: quantization.map(str::to_string),
        tokenizer: tokenizer.map(str::to_string),
        processor: processor.map(str::to_string),
        browser_support,
        files: files
            .iter()
            .map(|(path, source_url, bytes, purpose)| BrowserPackFile {
                path: (*path).to_string(),
                source_url: (*source_url).to_string(),
                bytes: *bytes,
                sha256: None,
                required: true,
                purpose: (*purpose).to_string(),
            })
            .collect(),
        tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
    }
}

pub fn default_browser_pack_catalog() -> Vec<BrowserPackManifest> {
    vec![
        pack(
            "qwen3-0.6b-onnx-browserpack",
            "qwen3-0.6b",
            "Qwen3 0.6B Browser Pack",
            "onnx-community/Qwen3-0.6B-DQ-ONNX",
            Modality::Chat,
            BrowserExecutionBackend::TransformersJsOnnx,
            Some("q4f16"),
            Some("tokenizer.json"),
            None,
            BrowserPackSupport {
                chromium: true,
                firefox: true,
                safari: true,
                standalone_web: true,
                requires_webgpu: false,
            },
            &[
                (
                    "config.json",
                    "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/config.json",
                    None,
                    "model-config",
                ),
                (
                    "generation_config.json",
                    "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/generation_config.json",
                    None,
                    "generation-config",
                ),
                (
                    "tokenizer.json",
                    "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/tokenizer.json",
                    None,
                    "tokenizer",
                ),
                (
                    "tokenizer_config.json",
                    "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/tokenizer_config.json",
                    None,
                    "tokenizer-config",
                ),
                (
                    "special_tokens_map.json",
                    "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/special_tokens_map.json",
                    None,
                    "special-tokens",
                ),
                (
                    "onnx/model_q4f16.onnx",
                    "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/onnx/model_q4f16.onnx",
                    None,
                    "decoder-model",
                ),
                (
                    "onnx/model.onnx_data",
                    "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/onnx/model.onnx_data",
                    None,
                    "decoder-weights",
                ),
            ],
            &["text", "cross-browser", "baseline", "local-first"],
        ),
        pack(
            "trocr-small-printed-browserpack",
            "trocr-small-printed",
            "TrOCR Small Printed Browser Pack",
            "Xenova/trocr-small-printed",
            Modality::Ocr,
            BrowserExecutionBackend::TransformersJsOnnx,
            Some("quantized"),
            None,
            Some("preprocessor_config.json"),
            BrowserPackSupport {
                chromium: true,
                firefox: true,
                safari: true,
                standalone_web: true,
                requires_webgpu: false,
            },
            &[
                (
                    "config.json",
                    "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/config.json",
                    None,
                    "model-config",
                ),
                (
                    "preprocessor_config.json",
                    "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/preprocessor_config.json",
                    None,
                    "processor-config",
                ),
                (
                    "tokenizer_config.json",
                    "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/tokenizer_config.json",
                    None,
                    "tokenizer-config",
                ),
                (
                    "vocab.json",
                    "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/vocab.json",
                    None,
                    "tokenizer-vocab",
                ),
                (
                    "onnx/encoder_model_quantized.onnx",
                    "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/onnx/encoder_model_quantized.onnx",
                    None,
                    "ocr-encoder",
                ),
                (
                    "onnx/decoder_model_merged_quantized.onnx",
                    "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/onnx/decoder_model_merged_quantized.onnx",
                    None,
                    "ocr-decoder",
                ),
            ],
            &["ocr", "cross-browser", "document", "image"],
        ),
        pack(
            "qwen3.5-0.8b-onnx-browserpack",
            "qwen3.5-0.8b",
            "Qwen3.5 0.8B Browser Pack",
            "onnx-community/Qwen3.5-0.8B-ONNX",
            Modality::VisionLanguage,
            BrowserExecutionBackend::TransformersJsOnnx,
            Some("q4"),
            Some("tokenizer.json"),
            Some("processor_config.json"),
            BrowserPackSupport {
                chromium: true,
                firefox: true,
                safari: true,
                standalone_web: true,
                requires_webgpu: true,
            },
            &[
                (
                    "config.json",
                    "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/config.json",
                    None,
                    "model-config",
                ),
                (
                    "generation_config.json",
                    "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/generation_config.json",
                    None,
                    "generation-config",
                ),
                (
                    "processor_config.json",
                    "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/processor_config.json",
                    None,
                    "processor-config",
                ),
                (
                    "preprocessor_config.json",
                    "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/preprocessor_config.json",
                    None,
                    "preprocessor-config",
                ),
                (
                    "tokenizer.json",
                    "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/tokenizer.json",
                    None,
                    "tokenizer",
                ),
                (
                    "tokenizer_config.json",
                    "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/tokenizer_config.json",
                    None,
                    "tokenizer-config",
                ),
                (
                    "onnx/embed_tokens_q4.onnx",
                    "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/onnx/embed_tokens_q4.onnx",
                    None,
                    "embed-tokens",
                ),
                (
                    "onnx/vision_encoder_fp16.onnx",
                    "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/onnx/vision_encoder_fp16.onnx",
                    None,
                    "vision-encoder",
                ),
                (
                    "onnx/decoder_model_merged_q4.onnx",
                    "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/onnx/decoder_model_merged_q4.onnx",
                    None,
                    "decoder-model",
                ),
            ],
            &["multimodal", "image", "document", "webgpu"],
        ),
    ]
}
