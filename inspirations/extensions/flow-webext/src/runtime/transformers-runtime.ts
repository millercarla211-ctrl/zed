import * as transformers from "@huggingface/transformers";

import { modelBaseUrl } from "./catalog";
import type { BrowserPackManifest, FlowDeviceTarget, FlowInferenceRequest } from "./protocol";

function transformerDevice(target: FlowDeviceTarget) {
  return target === "webgpu" ? "webgpu" : "wasm";
}

function syntheticStream(text: string, onChunk?: (chunk: string) => void) {
  if (!onChunk || !text) {
    return;
  }

  for (const chunk of text.split(/(\s+)/).filter(Boolean)) {
    onChunk(chunk);
  }
}

function buildTextMessages(request: FlowInferenceRequest) {
  if (request.selectionText) {
    return [
      { role: "system", content: "You are Flow, a concise browser writing assistant." },
      {
        role: "user",
        content: `${request.prompt}\n\nSelection:\n${request.selectionText}`,
      },
    ];
  }

  if (request.pageText) {
    return [
      { role: "system", content: "You are Flow, a browser page assistant." },
      {
        role: "user",
        content: `${request.prompt}\n\nPage context:\n${request.pageText}`,
      },
    ];
  }

  return [
    { role: "system", content: "You are Flow, a local browser assistant." },
    { role: "user", content: request.prompt },
  ];
}

export async function runTextGeneration(
  pack: BrowserPackManifest,
  request: FlowInferenceRequest,
  target: FlowDeviceTarget,
  onChunk?: (chunk: string) => void,
) {
  const model = modelBaseUrl(pack.packKey);
  const generator = await transformers.pipeline("text-generation", model, {
    device: transformerDevice(target),
    dtype: target === "webgpu" ? "fp16" : "q4f16",
  });

  const output = await generator(buildTextMessages(request), {
    max_new_tokens: 256,
    do_sample: false,
  });
  const normalizedOutput = output as any[];

  const text =
    normalizedOutput?.[0]?.generated_text?.at?.(-1)?.content ??
    normalizedOutput?.[0]?.generated_text ??
    "";
  syntheticStream(text, onChunk);
  return text.trim();
}

export async function runOcr(
  pack: BrowserPackManifest,
  imageSources: string[],
  target: FlowDeviceTarget,
) {
  const model = modelBaseUrl(pack.packKey);
  const reader = await transformers.pipeline("image-to-text", model, {
    device: transformerDevice(target),
    dtype: "q8",
  });

  const results = [];
  for (const imageSource of imageSources) {
    const output = await reader(imageSource);
    const normalizedOutput = output as any;
    const text = Array.isArray(output)
      ? output.map((item: { generated_text?: string }) => item.generated_text ?? "").join("\n")
      : normalizedOutput?.generated_text ?? "";
    results.push(text.trim());
  }

  return results.join("\n\n").trim();
}

export async function runMultimodal(
  pack: BrowserPackManifest,
  request: FlowInferenceRequest,
  target: FlowDeviceTarget,
  onChunk?: (chunk: string) => void,
) {
  const modelId = modelBaseUrl(pack.packKey);
  const processor = await transformers.AutoProcessor.from_pretrained(modelId);
  const tokenizer = await transformers.AutoTokenizer.from_pretrained(modelId);
  const model = await transformers.AutoModelForImageTextToText.from_pretrained(modelId, {
    device: transformerDevice(target),
    dtype: target === "webgpu" ? "fp16" : "q4",
  });

  const firstImage = request.imageSources[0];
  if (!firstImage) {
    throw new Error("Multimodal local inference requires at least one image source.");
  }

  const image = await transformers.RawImage.read(firstImage);
  const conversation: any = [
    {
      role: "user",
      content: [
        { type: "image" },
        { type: "text", text: request.prompt },
      ],
    },
  ];

  const text =
    typeof (processor as { apply_chat_template?: Function }).apply_chat_template === "function"
      ? processor.apply_chat_template(conversation, {
          add_generation_prompt: true,
        })
      : request.prompt;

  const inputs = await processor(text, image);
  const outputs = (await model.generate({
    ...inputs,
    max_new_tokens: 256,
  })) as any;
  const promptLength = inputs.input_ids?.dims?.at?.(-1) ?? 0;
  const generated =
    promptLength > 0 ? outputs.slice(null, [promptLength, null]) : outputs;
  const decoded =
    typeof (processor as { batch_decode?: Function }).batch_decode === "function"
      ? processor.batch_decode(generated, {
          skip_special_tokens: true,
        })
      : tokenizer.batch_decode(generated, {
          skip_special_tokens: true,
        });

  const response = decoded?.[0] ?? "";
  syntheticStream(response, onChunk);
  return response.trim();
}
