import browser from "webextension-polyfill";

import { installCommonBackgroundHandlers } from "./common";

async function openPrimarySurface() {
  const chromeApi = (globalThis as typeof globalThis & {
    chrome?: Record<string, any>;
  }).chrome;
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
  if (chromeApi?.sidePanel && typeof tab?.id === "number") {
    await chromeApi.sidePanel.open({ tabId: tab.id });
    return;
  }

  await browser.action.openPopup();
}

void installCommonBackgroundHandlers(openPrimarySurface);
