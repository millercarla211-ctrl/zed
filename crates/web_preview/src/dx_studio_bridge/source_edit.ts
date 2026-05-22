  const fetchHotReloadVersion = async (endpoint, target) => {
    const url = new URL(endpoint || "/_dx/hot-reload/version", window.location.href);
    if (target) url.searchParams.set("target", target);
    const response = await fetch(url.href, { cache: "no-store" });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const text = await response.text();
    try {
      const json = JSON.parse(text);
      return JSON.stringify(json);
    } catch (_error) {
      return text;
    }
  };

  const reselectAfterMutation = () => {
    for (const delay of [80, 250, 700, 1400]) {
      window.setTimeout(() => restoreLastSelection(), delay);
    }
  };

  const receiptStyleStatus = (receipt) => {
    return receipt?.style_edit_context?.plan_status
      || receipt?.style_edit_context?.style_edit_prefill?.status
      || null;
  };

  const receiptStatusDetailText = (receipt) => {
    if (!receipt) return "edit failed";
    const styleStatus = receiptStyleStatus(receipt);
    if (receipt.status_detail === "stale_source") {
      return "stale source refused: refresh selection";
    }
    if (receipt.operation === "update_design_token" && styleStatus === "missing_declared_style_contract") {
      return "missing dx-style token contract: edit refused";
    }
    if (receipt.status === "source_updated" && receipt.operation === "update_design_token" && receipt.style_edit_context) {
      return "source updated with dx-style context";
    }
    if (receipt.status === "source_updated") return "source updated";
    return receipt.status_detail || receipt.error || "edit failed";
  };

  const receiptStatusText = (receipt) => {
    if (!receipt) return "edit failed";
    const detail = receiptStatusDetailText(receipt);
    const styleStatus = receiptStyleStatus(receipt);
    if (styleStatus && receipt.operation === "update_design_token" && !detail.includes(styleStatus)) {
      return limitText(`${detail} (${styleStatus})`, 96) || "edit failed";
    }
    return limitText(detail, 96) || "edit failed";
  };

  const sendEditRequest = async (selection, payload, statusText) => {
    try {
      payload.hot_reload_before_version = await fetchHotReloadVersion(
        selection.hot_reload_version_endpoint || "/_dx/hot-reload/version",
        selection.hot_reload_target || null
      );
    } catch (_error) {
      payload.hot_reload_before_version = null;
    }
    state.hotReloadStatus = null;
    post(payload);
    drawSelection(selection, statusText);
  };

  const renderSourceEditReceipt = (receipt) => {
    state.hotReloadStatus = null;
    state.lastReceipt = receipt;
    if (state.selected) {
      drawSelection(state.selected, receiptStatusText(receipt));
    }
    return receipt?.status === "source_updated";
  };

  const afterSourceEdit = async (receipt) => {
    if (!renderSourceEditReceipt(receipt)) {
      reselectAfterMutation();
      return;
    }
    reselectAfterMutation();
    const endpoint = receipt.hot_reload?.version_endpoint || "/_dx/hot-reload/version";
    const target = receipt.hot_reload?.target || state.selected?.hot_reload_target || null;
    recordHotReloadStatus("polling", "waiting for DX-WWW version", endpoint, target);

    let before = receipt.hot_reload?.before_version || null;
    if (!before) {
      try {
        before = await fetchHotReloadVersion(endpoint, target);
      } catch (_error) {
        recordHotReloadStatus("refresh_needed", "version endpoint unavailable", endpoint, target);
        if (state.selected) drawSelection(state.selected, "source updated, refresh needed");
        return;
      }
    }

    let attempts = 0;
    const timer = window.setInterval(async () => {
      attempts += 1;
      try {
        const current = await fetchHotReloadVersion(endpoint, target);
        if (current !== before) {
          window.clearInterval(timer);
          recordHotReloadStatus("seen", "version changed", endpoint, target);
          restoreLastSelection();
          if (state.selected) drawSelection(state.selected, "hot reload seen");
        } else if (attempts >= 14) {
          window.clearInterval(timer);
          recordHotReloadStatus("refresh_needed", "version poll timed out", endpoint, target);
          if (state.selected) drawSelection(state.selected, "source updated, refresh needed");
        }
      } catch (_error) {
        window.clearInterval(timer);
        recordHotReloadStatus("refresh_needed", "version poll failed", endpoint, target);
        if (state.selected) drawSelection(state.selected, "source updated, refresh needed");
      }
    }, 500);
  };

