import { FlowBrowserStorage } from "./browser-storage";
import { installBrowserPackFetch } from "./browserpack-fetch";
import { browserPackCatalog, getBrowserPackByModelKey } from "./catalog";
import { detectCapabilities } from "./capabilities";
import { runMultimodal, runOcr, runTextGeneration } from "./transformers-runtime";
import type {
  BrowserPackManifest,
  BrowserPackStatus,
  FlowExecutionPlan,
  FlowInferenceRequest,
  FlowModality,
  FlowRuntimeReadiness,
  FlowTask,
  FlowUiSettings,
  FlowWorkbenchDraft,
} from "./protocol";

function storageBackend(capabilities = detectCapabilities()) {
  if (capabilities.opfs) {
    return "opfs";
  }
  if (capabilities.indexeddb) {
    return "indexeddb";
  }
  return "extension-storage";
}

export class FlowBrowserEngine {
  readonly storage = new FlowBrowserStorage();

  constructor() {
    installBrowserPackFetch(this.storage);
  }

  detectCapabilities() {
    return detectCapabilities();
  }

  packCatalog() {
    return browserPackCatalog;
  }

  defaultPrompt(task: FlowTask): string {
    switch (task) {
      case "rewrite-selection":
        return "Rewrite the selected text so it is clearer, tighter, and more professional.";
      case "summarize-page":
        return "Summarize this page into the key points, risks, and next actions.";
      case "compose-draft":
        return "Compose a polished draft reply using the page context and selected text.";
      case "explain-page":
        return "Explain the important ideas on this page in plain language.";
      case "ocr-image":
        return "Extract the visible text from the provided image as clean plain text.";
      case "multimodal-ask":
        return "Answer the question using the provided image and the current page context.";
      default:
        return "Help with the current browser context.";
    }
  }

  modalityForTask(task: FlowTask): FlowModality {
    switch (task) {
      case "ocr-image":
        return "ocr";
      case "multimodal-ask":
        return "vision-language";
      default:
        return "chat";
    }
  }

  preferredModelForTask(task: FlowTask, settings: FlowUiSettings): string {
    switch (this.modalityForTask(task)) {
      case "ocr":
        return settings.preferredOcrModel;
      case "vision-language":
        return settings.preferredVisionLanguageModel;
      default:
        return settings.preferredChatModel;
    }
  }

  async settings(): Promise<FlowUiSettings> {
    return this.storage.getSettings();
  }

  async workbenchDraft(): Promise<FlowWorkbenchDraft> {
    return this.storage.getWorkbenchDraft();
  }

  async runtimeReadiness(localOnly?: boolean): Promise<FlowRuntimeReadiness> {
    const capabilities = this.detectCapabilities();
    const packStatuses = await this.packStatuses();
    const resolvedLocalOnly = localOnly ?? (await this.storage.getLocalOnly());
    const textReady = packStatuses.some(
      (status) => status.modelKey === "qwen3-0.6b" && status.state === "ready",
    );
    const ocrReady = packStatuses.some(
      (status) => status.modelKey === "trocr-small-printed" && status.state === "ready",
    );
    const multimodalReady = packStatuses.some(
      (status) => status.modelKey === "qwen3.5-0.8b" && status.state === "ready",
    );

    let recommendedNextStep = "Flow browser runtime is ready for local work.";
    if (!textReady) {
      recommendedNextStep = "Install the Qwen3 text pack to unlock offline local writing tasks.";
    } else if (!ocrReady) {
      recommendedNextStep = "Install the OCR pack if the client expects screenshot and image text extraction.";
    } else if (capabilities.webgpu && !multimodalReady) {
      recommendedNextStep =
        "Install the multimodal pack to enable image and document reasoning on WebGPU browsers.";
    }

    const notes = [
      resolvedLocalOnly
        ? "Local-only mode is active. Remote providers stay disabled until Firebase wiring is added."
        : "Remote fallback is allowed later, but this build is prepared to run fully local first.",
      textReady
        ? "Qwen3 local text is ready for offline use after pack verification."
        : "Text generation is blocked until the Qwen3 browser pack is cached.",
    ];

    if (!capabilities.webgpu) {
      notes.push("This browser does not expose WebGPU, so multimodal local inference stays gated.");
    }
    if (!capabilities.opfs) {
      notes.push("OPFS is unavailable here, so cached model files fall back to IndexedDB or extension storage.");
    }

    return {
      capabilities,
      storageBackend: storageBackend(capabilities),
      packStatuses,
      localOnly: resolvedLocalOnly,
      textReady,
      ocrReady,
      multimodalReady,
      recommendedNextStep,
      notes,
    };
  }

  planExecution(
    task: FlowTask,
    modality: FlowModality,
    localOnly: boolean,
    preferredModel?: string,
  ): FlowExecutionPlan {
    const capabilities = this.detectCapabilities();

    let selectedModel = preferredModel ?? null;
    if (!selectedModel) {
      if (modality === "chat") {
        selectedModel = "qwen3-0.6b";
      } else if (modality === "ocr") {
        selectedModel = "trocr-small-printed";
      } else if (modality === "vision-language") {
        selectedModel = "qwen3.5-0.8b";
      }
    }

    const pack = selectedModel ? getBrowserPackByModelKey(selectedModel) : null;
    const unsupportedReason =
      modality === "vision-language" && !capabilities.webgpu
        ? "Local multimodal inference is disabled because WebGPU is unavailable."
        : !pack
          ? "No browser pack is registered for this request."
          : null;

    return {
      task,
      modality,
      selectedModel,
      packKey: pack?.packKey ?? null,
      storageBackend: storageBackend(capabilities),
      deviceTarget: capabilities.webgpu ? "webgpu" : "wasm",
      localOnly,
      remoteAllowed: !localOnly,
      reasons: [
        `Flow browser flavor: ${capabilities.flavor}`,
        localOnly
          ? "Local-only mode is enabled; remote inference must stay disabled."
          : "Remote fallback is allowed if the caller chooses to use it later.",
        modality === "chat"
          ? "Cross-browser local text is locked to the Qwen3 0.6B browser pack."
          : modality === "ocr"
            ? "OCR uses the TrOCR browser pack for local screenshot and image text extraction."
            : "Multimodal local inference is gated on WebGPU-capable browsers.",
      ],
      unsupportedReason,
    };
  }

  async ensurePack(modelKey: string, onProgress?: (status: string) => void) {
    const pack = getBrowserPackByModelKey(modelKey);
    if (!pack) {
      throw new Error(`No browser pack is registered for ${modelKey}.`);
    }

    await this.storage.ensurePackDownloaded(pack, onProgress);
    return pack;
  }

  async packStatus(modelKey: string): Promise<BrowserPackStatus | null> {
    const pack = getBrowserPackByModelKey(modelKey);
    if (!pack) {
      return null;
    }

    return this.storage.getPackStatus(pack);
  }

  async packStatuses(): Promise<BrowserPackStatus[]> {
    return this.storage.listPackStatuses(browserPackCatalog);
  }

  async removePack(modelKey: string): Promise<boolean> {
    const pack = getBrowserPackByModelKey(modelKey);
    if (!pack) {
      return false;
    }

    await this.storage.removePack(pack.packKey);
    return true;
  }

  async run(
    request: FlowInferenceRequest,
    onChunk?: (chunk: string) => void,
  ): Promise<{ plan: FlowExecutionPlan; output: string; pack: BrowserPackManifest }> {
    const plan = this.planExecution(
      request.task,
      request.modality,
      request.localOnly,
      request.preferredModel,
    );

    if (plan.unsupportedReason) {
      throw new Error(plan.unsupportedReason);
    }

    if (!plan.selectedModel) {
      throw new Error("The browser execution plan did not select a model.");
    }

    const pack = await this.ensurePack(plan.selectedModel);
    let output = "";

    if (request.modality === "chat") {
      output = await runTextGeneration(pack, request, plan.deviceTarget, onChunk);
    } else if (request.modality === "ocr") {
      output = await runOcr(pack, request.imageSources, plan.deviceTarget);
    } else {
      output = await runMultimodal(pack, request, plan.deviceTarget, onChunk);
    }

    return { plan, output, pack };
  }

  async saveDraft(patch: Partial<FlowWorkbenchDraft>): Promise<FlowWorkbenchDraft> {
    return this.storage.updateWorkbenchDraft(patch);
  }

  async saveSettings(patch: Partial<FlowUiSettings>): Promise<FlowUiSettings> {
    return this.storage.updateSettings(patch);
  }
}
