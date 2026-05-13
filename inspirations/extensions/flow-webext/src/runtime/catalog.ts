import type { BrowserPackManifest } from "./protocol";

export const FLOW_BROWSERPACK_ORIGIN = "https://flow.browserpack.local";

export const browserPackCatalog: BrowserPackManifest[] = [
  {
    version: 1,
    packKey: "qwen3-0.6b-onnx-browserpack",
    modelKey: "qwen3-0.6b",
    displayName: "Qwen3 0.6B Browser Pack",
    repoId: "onnx-community/Qwen3-0.6B-DQ-ONNX",
    modality: "chat",
    backend: "transformersjs-onnx",
    quantization: "q4f16",
    requiresWebgpu: false,
    files: [
      {
        path: "config.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/config.json",
        purpose: "model-config",
        contentType: "application/json",
      },
      {
        path: "generation_config.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/generation_config.json",
        purpose: "generation-config",
        contentType: "application/json",
      },
      {
        path: "tokenizer.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/tokenizer.json",
        purpose: "tokenizer",
        contentType: "application/json",
      },
      {
        path: "tokenizer_config.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/tokenizer_config.json",
        purpose: "tokenizer-config",
        contentType: "application/json",
      },
      {
        path: "special_tokens_map.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/special_tokens_map.json",
        purpose: "special-tokens",
        contentType: "application/json",
      },
      {
        path: "onnx/model_q4f16.onnx",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/onnx/model_q4f16.onnx",
        purpose: "decoder-model",
        contentType: "application/octet-stream",
      },
      {
        path: "onnx/model.onnx_data",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3-0.6B-DQ-ONNX/resolve/main/onnx/model.onnx_data",
        purpose: "decoder-weights",
        contentType: "application/octet-stream",
      },
    ],
  },
  {
    version: 1,
    packKey: "trocr-small-printed-browserpack",
    modelKey: "trocr-small-printed",
    displayName: "TrOCR Small Printed Browser Pack",
    repoId: "Xenova/trocr-small-printed",
    modality: "ocr",
    backend: "transformersjs-onnx",
    quantization: "quantized",
    requiresWebgpu: false,
    files: [
      {
        path: "config.json",
        sourceUrl: "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/config.json",
        purpose: "model-config",
        contentType: "application/json",
      },
      {
        path: "preprocessor_config.json",
        sourceUrl:
          "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/preprocessor_config.json",
        purpose: "processor-config",
        contentType: "application/json",
      },
      {
        path: "tokenizer_config.json",
        sourceUrl:
          "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/tokenizer_config.json",
        purpose: "tokenizer-config",
        contentType: "application/json",
      },
      {
        path: "vocab.json",
        sourceUrl: "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/vocab.json",
        purpose: "tokenizer-vocab",
        contentType: "application/json",
      },
      {
        path: "onnx/encoder_model_quantized.onnx",
        sourceUrl:
          "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/onnx/encoder_model_quantized.onnx",
        purpose: "ocr-encoder",
        contentType: "application/octet-stream",
      },
      {
        path: "onnx/decoder_model_merged_quantized.onnx",
        sourceUrl:
          "https://huggingface.co/Xenova/trocr-small-printed/resolve/main/onnx/decoder_model_merged_quantized.onnx",
        purpose: "ocr-decoder",
        contentType: "application/octet-stream",
      },
    ],
  },
  {
    version: 1,
    packKey: "qwen3.5-0.8b-onnx-browserpack",
    modelKey: "qwen3.5-0.8b",
    displayName: "Qwen3.5 0.8B Browser Pack",
    repoId: "onnx-community/Qwen3.5-0.8B-ONNX",
    modality: "vision-language",
    backend: "transformersjs-onnx",
    quantization: "mixed-q4-fp16",
    requiresWebgpu: true,
    files: [
      {
        path: "config.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/config.json",
        purpose: "model-config",
        contentType: "application/json",
      },
      {
        path: "generation_config.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/generation_config.json",
        purpose: "generation-config",
        contentType: "application/json",
      },
      {
        path: "processor_config.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/processor_config.json",
        purpose: "processor-config",
        contentType: "application/json",
      },
      {
        path: "preprocessor_config.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/preprocessor_config.json",
        purpose: "preprocessor-config",
        contentType: "application/json",
      },
      {
        path: "tokenizer.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/tokenizer.json",
        purpose: "tokenizer",
        contentType: "application/json",
      },
      {
        path: "tokenizer_config.json",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/tokenizer_config.json",
        purpose: "tokenizer-config",
        contentType: "application/json",
      },
      {
        path: "onnx/embed_tokens_q4.onnx",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/onnx/embed_tokens_q4.onnx",
        purpose: "embed-tokens",
        contentType: "application/octet-stream",
      },
      {
        path: "onnx/vision_encoder_fp16.onnx",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/onnx/vision_encoder_fp16.onnx",
        purpose: "vision-encoder",
        contentType: "application/octet-stream",
      },
      {
        path: "onnx/decoder_model_merged_q4.onnx",
        sourceUrl:
          "https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX/resolve/main/onnx/decoder_model_merged_q4.onnx",
        purpose: "decoder-model",
        contentType: "application/octet-stream",
      },
    ],
  },
];

export function getBrowserPackByModelKey(modelKey: string) {
  return browserPackCatalog.find((pack) => pack.modelKey === modelKey) ?? null;
}

export function modelBaseUrl(packKey: string) {
  return `${FLOW_BROWSERPACK_ORIGIN}/${packKey}`;
}
