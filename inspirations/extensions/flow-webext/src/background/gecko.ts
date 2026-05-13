import { installCommonBackgroundHandlers } from "./common";

async function openPrimarySurface() {
  const browserApi = (globalThis as typeof globalThis & {
    browser?: Record<string, any>;
  }).browser;
  if (browserApi?.sidebarAction?.open) {
    await browserApi.sidebarAction.open();
  }
}

void installCommonBackgroundHandlers(openPrimarySurface);
