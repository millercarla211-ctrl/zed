import browser from "webextension-polyfill";

async function activeTabId() {
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
  return typeof tab?.id === "number" ? tab.id : null;
}

export async function installCommonBackgroundHandlers(openPrimarySurface: () => Promise<void>) {
  browser.runtime.onInstalled.addListener(async () => {
    await browser.contextMenus.removeAll();
    await browser.contextMenus.create({
      id: "flow-open",
      title: "Open Flow",
      contexts: ["all"],
    });
    await browser.contextMenus.create({
      id: "flow-overlay",
      title: "Toggle Flow Overlay",
      contexts: ["all"],
    });
  });

  browser.contextMenus.onClicked.addListener(async (info: any) => {
    if (info.menuItemId === "flow-open") {
      await openPrimarySurface();
      return;
    }

    if (info.menuItemId === "flow-overlay") {
      const tabId = await activeTabId();
      if (tabId != null) {
        await browser.tabs.sendMessage(tabId, { type: "flow:toggle-overlay" });
      }
    }
  });

  browser.commands.onCommand.addListener(async (command: string) => {
    if (command === "toggle-flow") {
      await openPrimarySurface();
    }
  });
}
