  window[BRIDGE_KEY] = {
    schema: "zed.web_preview.dx_studio_bridge.v1",
    selectSurface() {
      beginCapture("select");
    },
    editText() {
      beginCapture("edit_text");
    },
    editSurface() {
      beginCapture("edit_operation");
    },
    clearOverlay,
    currentSelection() {
      return state.selected;
    },
    afterSourceEdit,
    restoreLastSelection
  };

  const attachBaseAliases = () => {
    if (!window.__zedWebPreview) return;
    window.__zedWebPreview.selectDxStudioSurface = window[BRIDGE_KEY].selectSurface;
    window.__zedWebPreview.editDxStudioText = window[BRIDGE_KEY].editText;
    window.__zedWebPreview.editDxStudioSurface = window[BRIDGE_KEY].editSurface;
    window.__zedWebPreview.restoreDxStudioSelection = window[BRIDGE_KEY].restoreLastSelection;
  };

  attachBaseAliases();
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      attachBaseAliases();
      window.setTimeout(restoreLastSelection, 120);
    }, { once: true });
  } else {
    window.setTimeout(() => {
      attachBaseAliases();
      restoreLastSelection();
    }, 120);
  }
})();