import browser from "webextension-polyfill";

import type { QuickContextPayload } from "./protocol";

export const extensionBrowser = browser;

export async function getActiveTabId(): Promise<number | null> {
  const [tab] = await extensionBrowser.tabs.query({
    active: true,
    currentWindow: true,
  });
  return typeof tab?.id === "number" ? tab.id : null;
}

export async function requestQuickContext(): Promise<QuickContextPayload | null> {
  const tabId = await getActiveTabId();
  if (tabId == null) {
    return null;
  }

  try {
    const result = await extensionBrowser.tabs.sendMessage(tabId, {
      type: "flow:get-quick-context",
    });
    return result as QuickContextPayload;
  } catch {
    return null;
  }
}

export async function replaceSelection(text: string): Promise<boolean> {
  const tabId = await getActiveTabId();
  if (tabId == null) {
    return false;
  }

  try {
    await extensionBrowser.tabs.sendMessage(tabId, {
      type: "flow:replace-selection",
      text,
    });
    return true;
  } catch {
    return false;
  }
}

export async function toggleOverlay(): Promise<void> {
  const tabId = await getActiveTabId();
  if (tabId == null) {
    return;
  }

  await extensionBrowser.tabs.sendMessage(tabId, {
    type: "flow:toggle-overlay",
  });
}
