import browser from "webextension-polyfill";

const OVERLAY_ID = "flow-browser-overlay";

function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function currentSelectionText() {
  const active = document.activeElement;

  if (
    active instanceof HTMLTextAreaElement ||
    (active instanceof HTMLInputElement &&
      ["text", "search", "email", "url"].includes(active.type))
  ) {
    const start = active.selectionStart ?? 0;
    const end = active.selectionEnd ?? 0;
    return active.value.slice(start, end);
  }

  return window.getSelection()?.toString() ?? "";
}

function currentPageText() {
  return document.body?.innerText?.slice(0, 12000) ?? "";
}

function ensureOverlay() {
  let overlay = document.getElementById(OVERLAY_ID);
  if (overlay) {
    return overlay;
  }

  overlay = document.createElement("div");
  overlay.id = OVERLAY_ID;
  overlay.className = "flow-browser-overlay";
  overlay.innerHTML = `
    <div class="flow-browser-overlay-topline">
      <span class="flow-browser-overlay-eyebrow">Local-first browser runtime</span>
      <span class="flow-browser-overlay-pill">Flow</span>
    </div>
    <h2>Flow is active on this page</h2>
    <div class="muted">Use the extension surface to rewrite, summarize, explain, or compose with local models.</div>
    <div class="flow-browser-overlay-actions">
      <span class="flow-browser-overlay-chip">Shortcut: toggle-flow</span>
      <span class="flow-browser-overlay-chip">Selection-aware</span>
      <span class="flow-browser-overlay-chip">Local-first</span>
    </div>
    <div id="flow-browser-overlay-body" style="margin-top:10px;"></div>
    <div class="actions">
      <button id="flow-browser-overlay-close" type="button">Close</button>
    </div>
  `;
  document.documentElement.appendChild(overlay);

  const close = overlay.querySelector<HTMLButtonElement>("#flow-browser-overlay-close");
  close?.addEventListener("click", () => overlay?.remove());
  return overlay;
}

function toggleOverlay() {
  const existing = document.getElementById(OVERLAY_ID);
  if (existing) {
    existing.remove();
    return;
  }

  const overlay = ensureOverlay();
  const body = overlay.querySelector<HTMLElement>("#flow-browser-overlay-body");
  if (body) {
    const selectionText = currentSelectionText().trim();
    body.innerHTML = selectionText
      ? `
        <div class="flow-browser-overlay-preview">
          <strong>Current selection</strong>
          <p>${escapeHtml(selectionText.slice(0, 280))}</p>
        </div>
      `
      : `
        <div class="flow-browser-overlay-preview empty">
          <strong>No selection yet</strong>
          <p>Select text on the page, then use Flow to rewrite, summarize, or compose locally.</p>
        </div>
      `;
  }
}

async function replaceSelection(text: string) {
  const active = document.activeElement;

  if (
    active instanceof HTMLTextAreaElement ||
    (active instanceof HTMLInputElement &&
      ["text", "search", "email", "url"].includes(active.type))
  ) {
    const start = active.selectionStart ?? 0;
    const end = active.selectionEnd ?? 0;
    active.setRangeText(text, start, end, "end");
    active.dispatchEvent(new Event("input", { bubbles: true }));
    return;
  }

  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0) {
    return;
  }

  const range = selection.getRangeAt(0);
  range.deleteContents();
  range.insertNode(document.createTextNode(text));
  selection.removeAllRanges();
}

browser.runtime.onMessage.addListener((message: any) => {
  switch (message?.type) {
    case "flow:get-quick-context":
      return Promise.resolve({
        selectionText: currentSelectionText(),
        pageText: currentPageText(),
        title: document.title,
        url: location.href,
      });
    case "flow:replace-selection":
      return replaceSelection(message.text ?? "").then(() => ({ ok: true }));
    case "flow:toggle-overlay":
      toggleOverlay();
      return Promise.resolve({ ok: true });
    default:
      return false;
  }
});
