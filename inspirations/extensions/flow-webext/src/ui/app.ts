import { requestQuickContext, replaceSelection, toggleOverlay } from "../runtime/browser-api";
import { FlowBrowserEngine } from "../runtime/flow-engine";
import type {
  BrowserPackManifest,
  BrowserPackStatus,
  FlowExecutionPlan,
  FlowInferenceRequest,
  FlowRuntimeReadiness,
  FlowSurface,
  FlowTask,
  FlowUiSettings,
  FlowWorkbenchDraft,
  FlowWorkspaceSection,
  QuickContextPayload,
} from "../runtime/protocol";

type UiState = {
  settings: FlowUiSettings;
  draft: FlowWorkbenchDraft;
  readiness: FlowRuntimeReadiness;
  quickContext: QuickContextPayload | null;
  activeSection: FlowWorkspaceSection;
  status: string;
  output: string;
  running: boolean;
  lastPlan: FlowExecutionPlan | null;
  lastPack: BrowserPackManifest | null;
};

const TASK_OPTIONS: Array<{ task: FlowTask; label: string; detail: string }> = [
  {
    task: "rewrite-selection",
    label: "Rewrite selection",
    detail: "Tighten highlighted text and optionally apply it back into the page.",
  },
  {
    task: "summarize-page",
    label: "Summarize page",
    detail: "Turn the active tab into key points, risks, and next steps.",
  },
  {
    task: "compose-draft",
    label: "Compose draft",
    detail: "Write an email, reply, or post using the current page context.",
  },
  {
    task: "explain-page",
    label: "Explain page",
    detail: "Explain the current page in plain language for the user.",
  },
  {
    task: "ocr-image",
    label: "OCR image",
    detail: "Extract readable text from an image URL or screenshot asset.",
  },
  {
    task: "multimodal-ask",
    label: "Multimodal ask",
    detail: "Ask about an image or document when a WebGPU browser is available.",
  },
];

const SECTION_LABELS: Record<FlowWorkspaceSection, string> = {
  overview: "Overview",
  workspace: "Workbench",
  packs: "Model Packs",
  settings: "Settings",
  delivery: "Delivery",
};

const SURFACE_COPY: Record<
  FlowSurface,
  { title: string; eyebrow: string; intro: string; sections: FlowWorkspaceSection[] }
> = {
  popup: {
    title: "Flow Quick Panel",
    eyebrow: "Fast local actions",
    intro:
      "Handle rewrites, page summaries, and draft replies directly inside the browser with local models.",
    sections: ["overview", "workspace", "packs"],
  },
  sidepanel: {
    title: "Flow Side Panel",
    eyebrow: "Persistent browser workspace",
    intro:
      "Keep the full local workbench open while researching, drafting, and applying edits back to production apps.",
    sections: ["overview", "workspace", "packs", "settings", "delivery"],
  },
  sidebar: {
    title: "Flow Sidebar",
    eyebrow: "Firefox local workspace",
    intro:
      "Use the same local-first Flow runtime in Firefox with pack management, workbench tools, and delivery checks.",
    sections: ["overview", "workspace", "packs", "settings", "delivery"],
  },
  options: {
    title: "Flow Setup Console",
    eyebrow: "Client handoff controls",
    intro:
      "Configure local-only behavior, verify model packs, and review the project state before handing the build to the client.",
    sections: ["overview", "packs", "settings", "delivery"],
  },
};

function normalizeSurface(surface: string): FlowSurface {
  switch (surface) {
    case "options":
    case "sidepanel":
    case "sidebar":
      return surface;
    default:
      return "popup";
  }
}

function defaultSection(surface: FlowSurface): FlowWorkspaceSection {
  switch (surface) {
    case "popup":
      return "overview";
    case "options":
      return "settings";
    default:
      return "workspace";
  }
}

function badgeTone(value: string) {
  if (value === "ready" || value === "on" || value === "enabled") {
    return "good";
  }
  if (value === "partial" || value === "pending" || value === "optional") {
    return "warn";
  }
  if (value === "corrupt" || value === "blocked" || value === "off") {
    return "bad";
  }
  return "muted";
}

function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function clipText(value: string, max = 180) {
  if (!value.trim()) {
    return "";
  }

  return value.length > max ? `${value.slice(0, max - 1)}…` : value;
}

function taskOption(task: FlowTask) {
  return TASK_OPTIONS.find((item) => item.task === task) ?? TASK_OPTIONS[0];
}

function packStatusFor(
  readiness: FlowRuntimeReadiness,
  modelKey: string,
): BrowserPackStatus | null {
  return readiness.packStatuses.find((status) => status.modelKey === modelKey) ?? null;
}

function packActionLabel(status: BrowserPackStatus) {
  switch (status.state) {
    case "partial":
      return "Resume download";
    case "corrupt":
      return "Repair pack";
    case "missing":
      return "Install pack";
    default:
      return "Verify pack";
  }
}

function packSummaryText(status: BrowserPackStatus) {
  switch (status.state) {
    case "ready":
      return "Cached and verified for offline local use.";
    case "partial":
      return "Some files are cached. Resume the download to finish verification.";
    case "corrupt":
      return "At least one cached file failed integrity checks. Repair this pack before use.";
    default:
      return "This pack is not cached yet.";
  }
}

function renderPackCard(
  pack: BrowserPackManifest,
  status: BrowserPackStatus,
  readiness: FlowRuntimeReadiness,
) {
  const isRequired = pack.modelKey === "qwen3-0.6b";
  const needsWebgpu = pack.requiresWebgpu;
  const blocked = needsWebgpu && !readiness.capabilities.webgpu;

  return `
    <article class="feature-card pack-card ${status.state}">
      <div class="card-topline">
        <span class="eyebrow">${pack.modality}</span>
        <span class="badge ${badgeTone(status.state)}">${status.state}</span>
      </div>
      <h3>${escapeHtml(pack.displayName)}</h3>
      <p>${escapeHtml(packSummaryText(status))}</p>
      <div class="meta-list">
        <span><strong>Model</strong> ${escapeHtml(pack.modelKey)}</span>
        <span><strong>Backend</strong> ${escapeHtml(pack.backend)}</span>
        <span><strong>Quant</strong> ${escapeHtml(pack.quantization ?? "default")}</span>
        <span><strong>Files</strong> ${status.filesReady}/${status.filesTotal}</span>
        <span><strong>Storage</strong> ${escapeHtml(status.storageBackend)}</span>
        <span><strong>Role</strong> ${isRequired ? "Required" : "Optional"}</span>
      </div>
      <div class="hint ${blocked ? "bad" : "muted"}">
        ${
          blocked
            ? "This pack is capability-gated because WebGPU is unavailable in the current browser."
            : needsWebgpu
              ? "This pack unlocks multimodal image and document reasoning on WebGPU browsers."
              : "This pack runs on the cross-browser local baseline without a remote dependency."
        }
      </div>
      <div class="actions">
        <button type="button" data-action="download-pack" data-model-key="${escapeHtml(pack.modelKey)}">
          ${escapeHtml(packActionLabel(status))}
        </button>
        <button
          type="button"
          class="secondary"
          data-action="remove-pack"
          data-model-key="${escapeHtml(pack.modelKey)}"
          ${status.filesReady === 0 ? "disabled" : ""}
        >
          Remove cached files
        </button>
      </div>
    </article>
  `;
}

function renderOverview(surface: FlowSurface, state: UiState, engine: FlowBrowserEngine) {
  const textStatus = packStatusFor(state.readiness, state.settings.preferredChatModel);
  const ocrStatus = packStatusFor(state.readiness, state.settings.preferredOcrModel);
  const multimodalStatus = packStatusFor(
    state.readiness,
    state.settings.preferredVisionLanguageModel,
  );

  return `
    <section class="section-stack">
      <div class="hero-card">
        <div class="card-topline">
          <span class="eyebrow">${escapeHtml(SURFACE_COPY[surface].eyebrow)}</span>
          <span class="badge ${badgeTone(state.settings.localOnly ? "on" : "optional")}">
            ${state.settings.localOnly ? "Local only" : "Remote optional"}
          </span>
        </div>
        <h2>${escapeHtml(SURFACE_COPY[surface].title)}</h2>
        <p>${escapeHtml(SURFACE_COPY[surface].intro)}</p>
        <div class="hero-facts">
          <span class="pill">${escapeHtml(state.readiness.capabilities.flavor)}</span>
          <span class="pill">${escapeHtml(state.readiness.storageBackend)}</span>
          <span class="pill">${state.readiness.capabilities.webgpu ? "WebGPU ready" : "WASM fallback"}</span>
          <span class="pill">${textStatus?.state === "ready" ? "Qwen3 ready" : "Qwen3 pending"}</span>
        </div>
      </div>

      <div class="card-grid metrics-grid">
        <article class="metric-card">
          <span class="eyebrow">Text runtime</span>
          <strong>${state.readiness.textReady ? "Ready" : "Install required"}</strong>
          <p>${escapeHtml(textStatus?.displayName ?? "Qwen3 0.6B Browser Pack")}</p>
        </article>
        <article class="metric-card">
          <span class="eyebrow">OCR runtime</span>
          <strong>${state.readiness.ocrReady ? "Ready" : "Optional"}</strong>
          <p>${escapeHtml(ocrStatus?.displayName ?? "TrOCR Small Printed Browser Pack")}</p>
        </article>
        <article class="metric-card">
          <span class="eyebrow">Multimodal</span>
          <strong>${
            state.readiness.capabilities.webgpu
              ? state.readiness.multimodalReady
                ? "Ready"
                : "Install optional"
              : "Capability gated"
          }</strong>
          <p>${escapeHtml(multimodalStatus?.displayName ?? "Qwen3.5 0.8B Browser Pack")}</p>
        </article>
        <article class="metric-card">
          <span class="eyebrow">Client handoff</span>
          <strong>${surface === "options" ? "Control surface" : "Ready for review"}</strong>
          <p>Firebase remains the only intended external wiring gap.</p>
        </article>
      </div>

      <article class="feature-card callout-card">
        <div class="card-topline">
          <span class="eyebrow">Next step</span>
          <span class="badge ${badgeTone("optional")}">Delivery focus</span>
        </div>
        <h3>What Flow still needs on this machine</h3>
        <p>${escapeHtml(state.readiness.recommendedNextStep)}</p>
        <div class="note-list">
          ${state.readiness.notes.map((note) => `<span>${escapeHtml(note)}</span>`).join("")}
        </div>
      </article>

      <div class="section-header">
        <div>
          <h3>Quick actions</h3>
          <p>Run the highest-value local tasks without leaving the page.</p>
        </div>
        <div class="actions">
          <button type="button" class="secondary" data-action="refresh-context">Refresh tab context</button>
          <button type="button" class="secondary" data-action="toggle-overlay">Toggle page overlay</button>
        </div>
      </div>
      <div class="card-grid action-grid">
        ${TASK_OPTIONS.slice(0, 4)
          .map(
            (item) => `
              <button
                type="button"
                class="quick-action"
                data-action="quick-run"
                data-task="${item.task}"
              >
                <strong>${escapeHtml(item.label)}</strong>
                <span>${escapeHtml(item.detail)}</span>
              </button>
            `,
          )
          .join("")}
      </div>

      <article class="feature-card">
        <div class="section-header">
          <div>
            <h3>Active tab context</h3>
            <p>Flow can use the active page, selection, and title to produce better local output.</p>
          </div>
          <button type="button" class="secondary" data-action="open-workspace">
            Open workbench
          </button>
        </div>
        ${
          state.quickContext
            ? `
              <div class="context-grid">
                <div class="context-card">
                  <span class="eyebrow">Title</span>
                  <strong>${escapeHtml(state.quickContext.title || "Untitled page")}</strong>
                  <p>${escapeHtml(clipText(state.quickContext.url, 90))}</p>
                </div>
                <div class="context-card">
                  <span class="eyebrow">Selection</span>
                  <p>${escapeHtml(clipText(state.quickContext.selectionText || "No text selected.", 160))}</p>
                </div>
                <div class="context-card context-wide">
                  <span class="eyebrow">Page excerpt</span>
                  <p>${escapeHtml(clipText(state.quickContext.pageText || "No page text captured.", 260))}</p>
                </div>
              </div>
            `
            : `
              <div class="empty-state">
                <strong>No active tab context cached yet.</strong>
                <p>Use Flow on a supported page, then refresh the tab context to pull the current selection and page text.</p>
              </div>
            `
        }
      </article>
    </section>
  `;
}

function renderWorkspace(state: UiState, engine: FlowBrowserEngine) {
  const plan = engine.planExecution(
    state.draft.task,
    engine.modalityForTask(state.draft.task),
    state.settings.localOnly,
    engine.preferredModelForTask(state.draft.task, state.settings),
  );
  const task = taskOption(state.draft.task);

  return `
    <section class="section-stack">
      <div class="section-header">
        <div>
          <h2>Local workbench</h2>
          <p>Prepare requests, run them locally, and apply results back into the page.</p>
        </div>
        <span class="badge ${badgeTone(plan.unsupportedReason ? "blocked" : "ready")}">
          ${plan.unsupportedReason ? "Blocked" : "Runnable"}
        </span>
      </div>

      <div class="card-grid workspace-grid">
        <article class="feature-card">
          <div class="grid two">
            <label>
              Task
              <select id="flow-task">
                ${TASK_OPTIONS.map(
                  (option) => `
                    <option value="${option.task}" ${option.task === state.draft.task ? "selected" : ""}>
                      ${escapeHtml(option.label)}
                    </option>
                  `,
                ).join("")}
              </select>
            </label>
            <label>
              Target model
              <select id="flow-model">
                ${engine
                  .packCatalog()
                  .filter((pack) => pack.modality === engine.modalityForTask(state.draft.task))
                  .map((pack) => {
                    const selectedModel = engine.preferredModelForTask(
                      state.draft.task,
                      state.settings,
                    );
                    return `
                      <option value="${escapeHtml(pack.modelKey)}" ${
                        pack.modelKey === selectedModel ? "selected" : ""
                      }>
                        ${escapeHtml(pack.displayName)}
                      </option>
                    `;
                  })
                  .join("")}
              </select>
            </label>
          </div>
          <div class="hint muted">${escapeHtml(task.detail)}</div>
          <label>
            Prompt
            <textarea id="flow-prompt">${escapeHtml(state.draft.prompt)}</textarea>
          </label>
          <label>
            Image URL(s)
            <input
              id="flow-images"
              type="text"
              value="${escapeHtml(state.draft.imageSources)}"
              placeholder="https://example.com/image.png, data:image/png;base64,..."
            />
          </label>
          <div class="actions">
            <button type="button" class="secondary" data-action="refresh-context">Refresh tab context</button>
            <button type="button" class="secondary" data-action="clear-context">Clear context</button>
            <button type="button" data-action="run-flow">${state.running ? "Running..." : "Run local flow"}</button>
          </div>
        </article>

        <article class="feature-card">
          <div class="card-topline">
            <span class="eyebrow">Execution plan</span>
            <span class="badge ${badgeTone(plan.unsupportedReason ? "blocked" : "ready")}">
              ${plan.deviceTarget}
            </span>
          </div>
          <h3>${escapeHtml(task.label)}</h3>
          <div class="meta-list">
            <span><strong>Model</strong> ${escapeHtml(plan.selectedModel ?? "none")}</span>
            <span><strong>Pack</strong> ${escapeHtml(plan.packKey ?? "none")}</span>
            <span><strong>Storage</strong> ${escapeHtml(plan.storageBackend)}</span>
            <span><strong>Remote</strong> ${plan.remoteAllowed ? "Allowed later" : "Disabled"}</span>
          </div>
          ${
            plan.unsupportedReason
              ? `<div class="hint bad">${escapeHtml(plan.unsupportedReason)}</div>`
              : `<div class="note-list">${plan.reasons
                  .map((reason) => `<span>${escapeHtml(reason)}</span>`)
                  .join("")}</div>`
          }
          <div class="divider"></div>
          <div class="status-card">
            <span class="eyebrow">Status</span>
            <pre id="flow-status">${escapeHtml(state.status)}</pre>
          </div>
        </article>
      </div>

      <article class="feature-card">
        <div class="section-header">
          <div>
            <h3>Captured context</h3>
            <p>Selection and page data stay local to the extension unless you later wire a remote backend.</p>
          </div>
        </div>
        ${
          state.quickContext
            ? `
              <div class="context-grid">
                <div class="context-card">
                  <span class="eyebrow">Title</span>
                  <strong>${escapeHtml(state.quickContext.title || "Untitled page")}</strong>
                  <p>${escapeHtml(clipText(state.quickContext.url, 90))}</p>
                </div>
                <div class="context-card">
                  <span class="eyebrow">Selection</span>
                  <p>${escapeHtml(clipText(state.quickContext.selectionText || "No selection captured.", 180))}</p>
                </div>
                <div class="context-card context-wide">
                  <span class="eyebrow">Page excerpt</span>
                  <p>${escapeHtml(clipText(state.quickContext.pageText || "No page text captured.", 260))}</p>
                </div>
              </div>
            `
            : `
              <div class="empty-state">
                <strong>No active tab context cached.</strong>
                <p>Refresh the tab context to pull the current selection and page text before running a local task.</p>
              </div>
            `
        }
      </article>

      <article class="feature-card">
        <div class="section-header">
          <div>
            <h3>Output</h3>
            <p>Copy the result, push a rewrite back into the page, or keep refining the prompt.</p>
          </div>
          <div class="actions">
            <button type="button" class="secondary" data-action="copy-output" ${
              !state.output.trim() ? "disabled" : ""
            }>Copy</button>
            <button type="button" class="secondary" data-action="apply-output" ${
              !state.output.trim() ? "disabled" : ""
            }>Apply to page</button>
            <button type="button" class="secondary" data-action="clear-output" ${
              !state.output.trim() ? "disabled" : ""
            }>Clear</button>
          </div>
        </div>
        <div class="output-shell">
          <pre id="flow-output">${escapeHtml(state.output || "No local output yet.")}</pre>
        </div>
        ${
          state.lastPlan && state.lastPack
            ? `
              <div class="meta-list">
                <span><strong>Last model</strong> ${escapeHtml(state.lastPlan.selectedModel ?? "none")}</span>
                <span><strong>Last pack</strong> ${escapeHtml(state.lastPack.displayName)}</span>
                <span><strong>Target</strong> ${escapeHtml(state.lastPlan.deviceTarget)}</span>
              </div>
            `
            : ""
        }
      </article>
    </section>
  `;
}

function renderSettings(state: UiState, engine: FlowBrowserEngine) {
  const chatPacks = engine.packCatalog().filter((pack) => pack.modality === "chat");
  const ocrPacks = engine.packCatalog().filter((pack) => pack.modality === "ocr");
  const vlmPacks = engine
    .packCatalog()
    .filter((pack) => pack.modality === "vision-language");

  return `
    <section class="section-stack">
      <div class="section-header">
        <div>
          <h2>Behavior and delivery settings</h2>
          <p>Keep the browser experience local-first and ready for the client handoff.</p>
        </div>
      </div>

      <article class="feature-card">
        <div class="toggle-list">
          <label class="toggle-row">
            <input id="setting-local-only" type="checkbox" ${
              state.settings.localOnly ? "checked" : ""
            } />
            <span>
              <strong>Local-only mode</strong>
              <small>Prevent silent remote fallback. Firebase can wire remote providers later.</small>
            </span>
          </label>
          <label class="toggle-row">
            <input id="setting-auto-apply" type="checkbox" ${
              state.settings.autoApplyRewrite ? "checked" : ""
            } />
            <span>
              <strong>Auto-apply rewrites</strong>
              <small>When rewrite selection succeeds, push the updated text back into the active page.</small>
            </span>
          </label>
          <label class="toggle-row">
            <input id="setting-capture-context" type="checkbox" ${
              state.settings.captureActiveTabContext ? "checked" : ""
            } />
            <span>
              <strong>Capture active tab context before runs</strong>
              <small>Refresh title, selection, and page text automatically before local execution.</small>
            </span>
          </label>
        </div>
      </article>

      <article class="feature-card">
        <div class="grid two">
          <label>
            Default task
            <select id="setting-default-task">
              ${TASK_OPTIONS.map(
                (option) => `
                  <option value="${option.task}" ${
                    option.task === state.settings.defaultTask ? "selected" : ""
                  }>
                    ${escapeHtml(option.label)}
                  </option>
                `,
              ).join("")}
            </select>
          </label>
          <label>
            Preferred text model
            <select id="setting-chat-model">
              ${chatPacks.map(
                (pack) => `
                  <option value="${escapeHtml(pack.modelKey)}" ${
                    pack.modelKey === state.settings.preferredChatModel ? "selected" : ""
                  }>
                    ${escapeHtml(pack.displayName)}
                  </option>
                `,
              ).join("")}
            </select>
          </label>
          <label>
            Preferred OCR model
            <select id="setting-ocr-model">
              ${ocrPacks.map(
                (pack) => `
                  <option value="${escapeHtml(pack.modelKey)}" ${
                    pack.modelKey === state.settings.preferredOcrModel ? "selected" : ""
                  }>
                    ${escapeHtml(pack.displayName)}
                  </option>
                `,
              ).join("")}
            </select>
          </label>
          <label>
            Preferred multimodal model
            <select id="setting-vlm-model">
              ${vlmPacks.map(
                (pack) => `
                  <option value="${escapeHtml(pack.modelKey)}" ${
                    pack.modelKey === state.settings.preferredVisionLanguageModel
                      ? "selected"
                      : ""
                  }>
                    ${escapeHtml(pack.displayName)}
                  </option>
                `,
              ).join("")}
            </select>
          </label>
        </div>
      </article>

      <article class="feature-card">
        <div class="section-header">
          <div>
            <h3>User-facing controls</h3>
            <p>These flows are ready to demo or hand off to the client.</p>
          </div>
        </div>
        <div class="note-list">
          <span>Keyboard shortcut support is already wired through the extension command surface.</span>
          <span>Context menu actions can open Flow or toggle the in-page overlay.</span>
          <span>Popup, side panel, sidebar, and options all share the same local browser runtime.</span>
          <span>Qwen3 remains the low-end default for cross-browser local text execution.</span>
        </div>
      </article>
    </section>
  `;
}

function renderDelivery(surface: FlowSurface, state: UiState) {
  const textStatus = state.readiness.textReady ? "complete" : "install text pack";
  const ocrStatus = state.readiness.ocrReady ? "complete" : "optional";
  const multimodalStatus = state.readiness.capabilities.webgpu
    ? state.readiness.multimodalReady
      ? "complete"
      : "optional"
    : "capability gated";

  return `
    <section class="section-stack">
      <div class="section-header">
        <div>
          <h2>Client delivery checklist</h2>
          <p>Use this surface as the final review while Firebase is still being wired.</p>
        </div>
        <span class="badge ${badgeTone("ready")}">Flow surface: ${escapeHtml(surface)}</span>
      </div>

      <div class="card-grid metrics-grid">
        <article class="metric-card">
          <span class="eyebrow">Local text</span>
          <strong>${escapeHtml(textStatus)}</strong>
          <p>Qwen3 offline chat, rewrite, compose, and explain flows.</p>
        </article>
        <article class="metric-card">
          <span class="eyebrow">OCR</span>
          <strong>${escapeHtml(ocrStatus)}</strong>
          <p>Screenshot and image text extraction stays local after pack install.</p>
        </article>
        <article class="metric-card">
          <span class="eyebrow">Multimodal</span>
          <strong>${escapeHtml(multimodalStatus)}</strong>
          <p>WebGPU browsers can unlock image and document reasoning locally.</p>
        </article>
        <article class="metric-card">
          <span class="eyebrow">Remote wiring</span>
          <strong>Firebase pending</strong>
          <p>Everything else in this browser surface is already prepared locally.</p>
        </article>
      </div>

      <article class="feature-card">
        <div class="section-header">
          <div>
            <h3>What is already complete</h3>
            <p>The client can review these features without waiting for remote auth.</p>
          </div>
        </div>
        <div class="note-list">
          <span>Cross-browser popup, side panel, sidebar, and options screens are implemented.</span>
          <span>Local pack download, verification, removal, and partial-download recovery are in place.</span>
          <span>Page context capture, quick actions, rewrite apply-back, and clipboard actions are implemented.</span>
          <span>Local-only privacy defaults are enforced unless you deliberately enable remote wiring later.</span>
          <span>DX / Zed native Rust integration can reuse the same local-first model policy outside the browser.</span>
        </div>
      </article>

      <article class="feature-card">
        <div class="card-topline">
          <span class="eyebrow">Final handoff note</span>
          <span class="badge ${badgeTone("optional")}">External dependency</span>
        </div>
        <h3>Remaining external work</h3>
        <p>
          Firebase environment wiring remains the last outside dependency. This extension surface,
          its local model logic, and the user-facing screens are otherwise ready to deliver.
        </p>
      </article>
    </section>
  `;
}

function renderPacks(state: UiState, engine: FlowBrowserEngine) {
  return `
    <section class="section-stack">
      <div class="section-header">
        <div>
          <h2>Browser model packs</h2>
          <p>Install only the packs the client needs. Qwen3 text is the low-end baseline.</p>
        </div>
        <div class="actions">
          <button type="button" class="secondary" data-action="refresh-runtime">Refresh pack status</button>
        </div>
      </div>
      <div class="card-grid pack-grid">
        ${engine
          .packCatalog()
          .map((pack) =>
            renderPackCard(
              pack,
              packStatusFor(state.readiness, pack.modelKey) ?? {
                packKey: pack.packKey,
                modelKey: pack.modelKey,
                displayName: pack.displayName,
                state: "missing",
                filesReady: 0,
                filesTotal: pack.files.length,
                storageBackend: state.readiness.storageBackend,
                lastUpdatedAt: null,
                lastError: null,
              },
              state.readiness,
            ),
          )
          .join("")}
      </div>
    </section>
  `;
}

function renderShell(surface: FlowSurface, state: UiState, engine: FlowBrowserEngine) {
  const copy = SURFACE_COPY[surface];
  const sections = copy.sections;
  const activeSection = sections.includes(state.activeSection)
    ? state.activeSection
    : sections[0];

  let body = "";
  switch (activeSection) {
    case "workspace":
      body = renderWorkspace(state, engine);
      break;
    case "packs":
      body = renderPacks(state, engine);
      break;
    case "settings":
      body = renderSettings(state, engine);
      break;
    case "delivery":
      body = renderDelivery(surface, state);
      break;
    default:
      body = renderOverview(surface, state, engine);
      break;
  }

  return `
    <div class="app-shell ${surface}">
      <header class="app-header">
        <div>
          <span class="eyebrow">${escapeHtml(copy.eyebrow)}</span>
          <div class="title-row">
            <h1>${escapeHtml(copy.title)}</h1>
            <span class="pill">${escapeHtml(surface)}</span>
          </div>
        </div>
        <div class="header-actions">
          <button type="button" class="secondary" data-action="go-settings">Settings</button>
          <button type="button" class="secondary" data-action="go-packs">Model packs</button>
        </div>
      </header>

      <nav class="tab-bar" aria-label="Flow workspace sections">
        ${sections
          .map(
            (section) => `
              <button
                type="button"
                class="tab ${section === activeSection ? "active" : ""}"
                data-action="change-section"
                data-section="${section}"
              >
                ${escapeHtml(SECTION_LABELS[section])}
              </button>
            `,
          )
          .join("")}
      </nav>

      ${body}
    </div>
  `;
}

export async function mountFlowApp(surfaceInput: string) {
  const root = document.getElementById("flow-app");
  if (!root) {
    return;
  }
  const mountRoot = root;

  const surface = normalizeSurface(surfaceInput);
  const engine = new FlowBrowserEngine();
  const [settings, draft, readiness] = await Promise.all([
    engine.settings(),
    engine.workbenchDraft(),
    engine.runtimeReadiness(),
  ]);

  const initialTask = draft.task || settings.defaultTask;
  const state: UiState = {
    settings,
    draft: {
      task: initialTask,
      prompt: draft.prompt || engine.defaultPrompt(initialTask),
      imageSources: draft.imageSources ?? "",
    },
    readiness,
    quickContext: null,
    activeSection: defaultSection(surface),
    status: "Ready. Flow will stay local-first unless you deliberately wire remote providers later.",
    output: "",
    running: false,
    lastPlan: null,
    lastPack: null,
  };

  function render() {
    mountRoot.innerHTML = renderShell(surface, state, engine);
    bind();
  }

  async function refreshRuntimeStatus(statusMessage?: string) {
    if (statusMessage) {
      state.status = statusMessage;
    }
    state.readiness = await engine.runtimeReadiness(state.settings.localOnly);
  }

  async function refreshQuickContext(silent = false) {
    if (!silent) {
      state.status = "Refreshing active tab context...";
      render();
    }

    state.quickContext = await requestQuickContext();
    state.status = state.quickContext
      ? `Captured context from ${state.quickContext.title || "the active tab"}.`
      : "No active tab context was available.";
  }

  async function updateSettings(patch: Partial<FlowUiSettings>) {
    state.settings = await engine.saveSettings(patch);
    await refreshRuntimeStatus("Saved browser settings.");
    render();
  }

  async function updateDraft(patch: Partial<FlowWorkbenchDraft>, rerender = false) {
    state.draft = await engine.saveDraft(patch);
    if (rerender) {
      render();
    }
  }

  async function installPack(modelKey: string) {
    state.status = "Preparing model pack...";
    render();

    try {
      await engine.ensurePack(modelKey, (message) => {
        const statusEl = mountRoot.querySelector<HTMLPreElement>("#flow-status");
        if (statusEl) {
          statusEl.textContent = message;
        }
      });
      await refreshRuntimeStatus("Model pack is ready for offline local use.");
    } catch (error) {
      state.status = `Pack error: ${String(error)}`;
    }

    render();
  }

  async function removePack(modelKey: string) {
    state.status = "Removing cached pack files...";
    render();

    await engine.removePack(modelKey);
    await refreshRuntimeStatus("Cached pack files were removed.");
    render();
  }

  async function copyOutput() {
    if (!state.output.trim()) {
      return;
    }

    if (!navigator.clipboard?.writeText) {
      state.status = "Clipboard access is unavailable in this browser surface.";
      render();
      return;
    }

    await navigator.clipboard.writeText(state.output);
    state.status = "Copied local output to the clipboard.";
    render();
  }

  async function applyOutput() {
    if (!state.output.trim()) {
      return;
    }

    const applied = await replaceSelection(state.output);
    state.status = applied
      ? "Applied the local output back into the active page."
      : "Could not apply the local output to the page selection.";
    render();
  }

  async function runFlow(taskOverride?: FlowTask) {
    const task = taskOverride ?? state.draft.task;
    const previousTask = state.draft.task;
    const wasDefaultPrompt =
      !state.draft.prompt.trim() ||
      state.draft.prompt.trim() === engine.defaultPrompt(previousTask);

    if (task !== previousTask) {
      const nextPrompt = wasDefaultPrompt
        ? engine.defaultPrompt(task)
        : state.draft.prompt;
      state.draft = await engine.saveDraft({
        task,
        prompt: nextPrompt,
        imageSources: state.draft.imageSources,
      });
      state.activeSection = "workspace";
      render();
    }

    if (state.settings.captureActiveTabContext) {
      await refreshQuickContext(true);
    }

    const modality = engine.modalityForTask(task);
    const preferredModel = engine.preferredModelForTask(task, state.settings);
    const prompt = state.draft.prompt.trim() || engine.defaultPrompt(task);
    const imageSources = state.draft.imageSources
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean);

    if ((task === "ocr-image" || task === "multimodal-ask") && imageSources.length === 0) {
      state.status = "Add at least one image URL before running OCR or multimodal tasks.";
      state.activeSection = "workspace";
      render();
      return;
    }

    const request: FlowInferenceRequest = {
      task,
      modality,
      prompt,
      selectionText: state.quickContext?.selectionText,
      pageText: state.quickContext?.pageText,
      imageSources,
      localOnly: state.settings.localOnly,
      preferredModel,
    };

    state.running = true;
    state.output = "";
    state.status = "Planning local execution...";
    state.activeSection = "workspace";
    render();

    try {
      const outputEl = () => mountRoot.querySelector<HTMLPreElement>("#flow-output");
      const result = await engine.run(request, (chunk) => {
        state.output += chunk;
        const el = outputEl();
        if (el) {
          el.textContent = state.output;
        }
      });

      if (!state.output.trim()) {
        state.output = result.output;
      }

      state.lastPlan = result.plan;
      state.lastPack = result.pack;
      state.status = [
        `Model ${result.plan.selectedModel ?? "unknown"}`,
        `Pack ${result.pack.displayName}`,
        `Target ${result.plan.deviceTarget}`,
      ].join(" | ");

      if (
        task === "rewrite-selection" &&
        state.settings.autoApplyRewrite &&
        state.quickContext?.selectionText &&
        state.output.trim()
      ) {
        const applied = await replaceSelection(state.output);
        state.status = applied
          ? `${state.status} | rewrite applied to page`
          : `${state.status} | could not apply rewrite automatically`;
      }

      await refreshRuntimeStatus(state.status);
    } catch (error) {
      state.status = `Error: ${String(error)}`;
    } finally {
      state.running = false;
      render();
    }
  }

  function bind() {
    mountRoot
      .querySelectorAll<HTMLButtonElement>("[data-action='change-section']")
      .forEach((button) => {
        button.addEventListener("click", () => {
          const next = button.dataset.section as FlowWorkspaceSection | undefined;
          if (!next) {
            return;
          }
          state.activeSection = next;
          render();
        });
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='go-settings']")
      ?.addEventListener("click", () => {
        state.activeSection = "settings";
        render();
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='go-packs']")
      ?.addEventListener("click", () => {
        state.activeSection = "packs";
        render();
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='open-workspace']")
      ?.addEventListener("click", () => {
        state.activeSection = "workspace";
        render();
      });

    mountRoot
      .querySelectorAll<HTMLButtonElement>("[data-action='quick-run']")
      .forEach((button) => {
        button.addEventListener("click", () => {
          const task = button.dataset.task as FlowTask | undefined;
          if (task) {
            void runFlow(task);
          }
        });
      });

    mountRoot
      .querySelectorAll<HTMLButtonElement>("[data-action='download-pack']")
      .forEach((button) => {
        button.addEventListener("click", () => {
          const modelKey = button.dataset.modelKey;
          if (modelKey) {
            void installPack(modelKey);
          }
        });
      });

    mountRoot
      .querySelectorAll<HTMLButtonElement>("[data-action='remove-pack']")
      .forEach((button) => {
        button.addEventListener("click", () => {
          const modelKey = button.dataset.modelKey;
          if (modelKey) {
            void removePack(modelKey);
          }
        });
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='refresh-runtime']")
      ?.addEventListener("click", () => {
        void refreshRuntimeStatus("Refreshed browser runtime state.").then(() => render());
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='refresh-context']")
      ?.addEventListener("click", () => {
        void refreshQuickContext().then(() => render());
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='clear-context']")
      ?.addEventListener("click", () => {
        state.quickContext = null;
        state.status = "Cleared the cached tab context.";
        render();
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='toggle-overlay']")
      ?.addEventListener("click", () => {
        void toggleOverlay();
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='run-flow']")
      ?.addEventListener("click", () => {
        void runFlow();
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='copy-output']")
      ?.addEventListener("click", () => {
        void copyOutput();
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='apply-output']")
      ?.addEventListener("click", () => {
        void applyOutput();
      });

    mountRoot
      .querySelector<HTMLButtonElement>("[data-action='clear-output']")
      ?.addEventListener("click", () => {
        state.output = "";
        state.status = "Cleared the local output buffer.";
        render();
      });

    mountRoot
      .querySelector<HTMLSelectElement>("#flow-task")
      ?.addEventListener("change", (event) => {
        const select = event.currentTarget as HTMLSelectElement;
        const nextTask = select.value as FlowTask;
        const wasDefaultPrompt =
          !state.draft.prompt.trim() ||
          state.draft.prompt.trim() === engine.defaultPrompt(state.draft.task);
        const nextPrompt = wasDefaultPrompt ? engine.defaultPrompt(nextTask) : state.draft.prompt;
        void updateDraft(
          {
            task: nextTask,
            prompt: nextPrompt,
            imageSources: state.draft.imageSources,
          },
          true,
        );
      });

    mountRoot
      .querySelector<HTMLSelectElement>("#flow-model")
      ?.addEventListener("change", (event) => {
        const select = event.currentTarget as HTMLSelectElement;
        const patch =
          engine.modalityForTask(state.draft.task) === "ocr"
            ? { preferredOcrModel: select.value }
            : engine.modalityForTask(state.draft.task) === "vision-language"
              ? { preferredVisionLanguageModel: select.value }
              : { preferredChatModel: select.value };
        void updateSettings(patch);
      });

    mountRoot
      .querySelector<HTMLTextAreaElement>("#flow-prompt")
      ?.addEventListener("input", (event) => {
        const textarea = event.currentTarget as HTMLTextAreaElement;
        state.draft.prompt = textarea.value;
        void updateDraft({ prompt: textarea.value });
      });

    mountRoot
      .querySelector<HTMLInputElement>("#flow-images")
      ?.addEventListener("input", (event) => {
        const input = event.currentTarget as HTMLInputElement;
        state.draft.imageSources = input.value;
        void updateDraft({ imageSources: input.value });
      });

    mountRoot
      .querySelector<HTMLInputElement>("#setting-local-only")
      ?.addEventListener("change", (event) => {
        const input = event.currentTarget as HTMLInputElement;
        void updateSettings({ localOnly: input.checked });
      });

    mountRoot
      .querySelector<HTMLInputElement>("#setting-auto-apply")
      ?.addEventListener("change", (event) => {
        const input = event.currentTarget as HTMLInputElement;
        void updateSettings({ autoApplyRewrite: input.checked });
      });

    mountRoot
      .querySelector<HTMLInputElement>("#setting-capture-context")
      ?.addEventListener("change", (event) => {
        const input = event.currentTarget as HTMLInputElement;
        void updateSettings({ captureActiveTabContext: input.checked });
      });

    mountRoot
      .querySelector<HTMLSelectElement>("#setting-default-task")
      ?.addEventListener("change", (event) => {
        const select = event.currentTarget as HTMLSelectElement;
        const nextTask = select.value as FlowTask;
        void updateSettings({ defaultTask: nextTask });
      });

    mountRoot
      .querySelector<HTMLSelectElement>("#setting-chat-model")
      ?.addEventListener("change", (event) => {
        const select = event.currentTarget as HTMLSelectElement;
        void updateSettings({ preferredChatModel: select.value });
      });

    mountRoot
      .querySelector<HTMLSelectElement>("#setting-ocr-model")
      ?.addEventListener("change", (event) => {
        const select = event.currentTarget as HTMLSelectElement;
        void updateSettings({ preferredOcrModel: select.value });
      });

    mountRoot
      .querySelector<HTMLSelectElement>("#setting-vlm-model")
      ?.addEventListener("change", (event) => {
        const select = event.currentTarget as HTMLSelectElement;
        void updateSettings({ preferredVisionLanguageModel: select.value });
      });
  }

  if (state.settings.captureActiveTabContext) {
    state.quickContext = await requestQuickContext();
  }

  render();
}
