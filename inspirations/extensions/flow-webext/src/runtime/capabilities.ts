import type { BrowserCapabilityProfile, BrowserFlavor } from "./protocol";

declare const __FLOW_BROWSER_FLAVOR__: string;

function currentFlavor(): BrowserFlavor {
  switch (__FLOW_BROWSER_FLAVOR__) {
    case "firefox":
      return "firefox";
    case "safari":
      return "safari";
    default:
      return "chromium";
  }
}

export function detectCapabilities(): BrowserCapabilityProfile {
  const flavor = currentFlavor();
  const chromeApi = (globalThis as Record<string, unknown>).chrome as
    | Record<string, unknown>
    | undefined;
  const browserApi = (globalThis as Record<string, unknown>).browser as
    | Record<string, unknown>
    | undefined;

  const webgpu = typeof navigator !== "undefined" && "gpu" in navigator;
  const crossOriginIsolated =
    typeof globalThis.crossOriginIsolated === "boolean"
      ? globalThis.crossOriginIsolated
      : false;
  const wasmThreads =
    crossOriginIsolated && typeof SharedArrayBuffer !== "undefined";
  const opfs =
    typeof navigator !== "undefined" &&
    !!navigator.storage &&
    typeof navigator.storage.getDirectory === "function";
  const indexeddb = typeof indexedDB !== "undefined";

  const sidePanel = !!chromeApi?.sidePanel;
  const sidebarAction = !!browserApi?.sidebarAction;
  const offscreenDocument = !!chromeApi?.offscreen;
  const backgroundServiceWorker = flavor === "chromium";

  const notes = [
    flavor === "chromium"
      ? "Chromium uses the richest local browser runtime path."
      : "This browser uses the shared extension shell with capability gating.",
  ];

  if (!webgpu) {
    notes.push("WebGPU is unavailable; local multimodal will stay disabled.");
  }

  return {
    flavor,
    webgpu,
    wasmThreads,
    crossOriginIsolated,
    opfs,
    indexeddb,
    sidePanel,
    sidebarAction,
    offscreenDocument,
    backgroundServiceWorker,
    notes,
  };
}
