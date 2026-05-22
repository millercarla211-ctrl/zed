  const clearOverlay = () => {
    if (state.cleanup) {
      try { state.cleanup(); } catch (_error) {}
      state.cleanup = null;
    }
    if (state.overlayCleanup) {
      try { state.overlayCleanup(); } catch (_error) {}
      state.overlayCleanup = null;
    }
    if (state.overlay) {
      state.overlay.remove();
      state.overlay = null;
    }
  };

  const overlayRoot = () => {
    clearOverlay();
    const overlay = document.createElement("div");
    overlay.setAttribute("data-zed-dx-studio-overlay", "true");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "2147483646";
    overlay.style.pointerEvents = "none";
    overlay.style.contain = "layout style paint";
    overlay.style.font = "12px/1.35 system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
    document.documentElement.appendChild(overlay);
    state.overlay = overlay;
    return overlay;
  };

  const drawBox = (overlay, rect, options) => {
    if (!rect || rect.width <= 0 || rect.height <= 0) return null;
    const box = document.createElement("div");
    box.style.position = "fixed";
    box.style.left = `${rect.x}px`;
    box.style.top = `${rect.y}px`;
    box.style.width = `${rect.width}px`;
    box.style.height = `${rect.height}px`;
    box.style.border = options.border;
    box.style.borderRadius = options.radius || "6px";
    box.style.background = options.background || "transparent";
    box.style.boxShadow = options.shadow || "none";
    box.style.transform = "translateZ(0)";
    box.style.pointerEvents = "none";
    overlay.appendChild(box);
    if (options.label) {
      const label = document.createElement("div");
      label.textContent = options.label;
      label.style.position = "absolute";
      label.style.left = "-1px";
      label.style.top = options.labelPosition === "inside" ? "2px" : "-24px";
      label.style.maxWidth = "min(420px, 80vw)";
      label.style.overflow = "hidden";
      label.style.textOverflow = "ellipsis";
      label.style.whiteSpace = "nowrap";
      label.style.padding = "3px 7px";
      label.style.borderRadius = "5px";
      label.style.background = options.labelBackground || "rgba(13, 17, 23, 0.92)";
      label.style.color = options.labelColor || "rgb(240, 246, 252)";
      label.style.border = options.labelBorder || "1px solid rgba(148, 163, 184, 0.35)";
      label.style.boxShadow = "0 6px 18px rgba(0, 0, 0, 0.24)";
      box.appendChild(label);
    }
    return box;
  };

  const overlayLabel = (selection) => {
    const parts = [];
    if (selection.component) parts.push(selection.component);
    else if (selection.section) parts.push(selection.section);
    else if (selection.edit_id) parts.push(selection.edit_id);
    if (selection.source_file) parts.push(selection.source_file);
    parts.push(`${selection.breakpoint.active} ${selection.breakpoint.width}px`);
    return parts.filter(Boolean).join(" | ");
  };

  const pushAttributeSelector = (selectors, attribute, value) => {
    if (value) selectors.push(`[${attribute}="${CSS.escape(String(value))}"]`);
  };

  const elementForSelection = (selection) => {
    if (!selection) return null;
    const selectors = [];
    pushAttributeSelector(selectors, "data-dx-edit-id", selection.edit_id);
    pushAttributeSelector(selectors, "data-dx-editable-text", selection.text_marker);
    pushAttributeSelector(selectors, "data-dx-insert-slot", selection.insert_slot);
    pushAttributeSelector(selectors, "data-dx-media-slot", selection.media_slot);
    pushAttributeSelector(selectors, "data-dx-reorder-group", selection.reorder_group);
    pushAttributeSelector(selectors, "data-dx-design-token", selection.design_token);
    pushAttributeSelector(selectors, "data-dx-token-scope", selection.token_scope);
    pushAttributeSelector(selectors, "data-dx-style-surface", selection.style_surface);
    pushAttributeSelector(selectors, "data-dx-component", selection.component);
    pushAttributeSelector(selectors, "data-dx-section", selection.section);
    pushAttributeSelector(selectors, "data-dx-editable-section", selection.section);
    pushAttributeSelector(selectors, "data-dx-route", selection.route);
    pushAttributeSelector(selectors, "data-dx-source", selection.source_file);
    pushAttributeSelector(selectors, "data-dx-source-file", selection.source_file);
    if (selection.selector) selectors.push(selection.selector);
    for (const selector of selectors) {
      try {
        const element = document.querySelector(selector);
        if (element instanceof Element) return element;
      } catch (_error) {}
    }
    return null;
  };

  const scheduleSelectionRefresh = () => {
    if (!state.selected || state.refreshScheduled) return;
    state.refreshScheduled = true;
    window.requestAnimationFrame(() => {
      state.refreshScheduled = false;
      const element = elementForSelection(state.selected);
      if (!element) return;
      const selection = selectionSnapshot(element);
      if (!selection) return;
      drawSelection(selection, state.statusText);
    });
  };

  const installOverlayRefresh = () => {
    if (state.overlayCleanup) {
      try { state.overlayCleanup(); } catch (_error) {}
      state.overlayCleanup = null;
    }
    const onViewportChange = () => scheduleSelectionRefresh();
    window.addEventListener("resize", onViewportChange, { passive: true });
    window.addEventListener("scroll", onViewportChange, { passive: true, capture: true });
    state.overlayCleanup = () => {
      window.removeEventListener("resize", onViewportChange, { passive: true });
      window.removeEventListener("scroll", onViewportChange, { passive: true, capture: true });
    };
  };

  const drawSelection = (selection, statusText = null) => {
    if (!selection) return;
    state.selected = selection;
    state.statusText = statusText;
    persistSelection(selection);
    const overlay = overlayRoot();
    drawBox(overlay, selection.route_root?.rect, {
      border: "1px dashed rgba(148, 163, 184, 0.55)",
      radius: "8px",
      label: selection.route ? `route ${selection.route}` : "route/root",
      labelPosition: "inside",
      labelBackground: "rgba(15, 23, 42, 0.78)"
    });
    drawBox(overlay, selection.parent?.rect, {
      border: "1.5px dashed rgba(16, 185, 129, 0.78)",
      background: "rgba(16, 185, 129, 0.055)",
      label: selection.parent?.attributes?.["data-dx-section"]
        || selection.parent?.attributes?.["data-dx-component"]
        || selection.parent?.attributes?.["data-dx-edit-id"]
        || "parent surface",
      labelBackground: "rgba(6, 78, 59, 0.92)"
    });
    drawBox(overlay, selection.rect, {
      border: "2px dashed rgba(96, 165, 250, 0.98)",
      background: "rgba(96, 165, 250, 0.08)",
      shadow: "0 0 0 1px rgba(15, 23, 42, 0.55), 0 0 26px rgba(96, 165, 250, 0.18)",
      label: statusText ? `${overlayLabel(selection)} | ${statusText}` : overlayLabel(selection),
      labelBackground: "rgba(30, 64, 175, 0.94)"
    });
    drawBreakpointPanel(overlay, selection, statusText);
    installOverlayRefresh();
  };

  const receiptSourcePolicy = (receipt) => {
    const planPolicy = receipt?.source_edit_plan?.source_policy || null;
    const receiptPolicy = receipt?.source_policy || null;
    if (!planPolicy && !receiptPolicy) return null;
    return {
      ...(planPolicy || {}),
      ...(receiptPolicy || {})
    };
  };

  const policyFlagText = (enabled, label) => enabled === true ? label : null;

  const receiptSourcePolicyLines = (receipt) => {
    const policy = receiptSourcePolicy(receipt);
    if (!policy) return [];
    const lines = [
      policy.source_kind ? `Source policy ${policy.source_kind}` : null,
      policy.edit_allowed_by_policy === true ? "Policy write allowed" : null,
      policy.edit_allowed_by_policy === false ? "Policy write refused" : null,
      policyFlagText(policy.generated_runtime_file, "Generated/runtime source"),
      policyFlagText(policy.materialized_fallback, "Materialized fallback source"),
      policyFlagText(policy.manifest_allows_generated_edit, "Manifest allows generated edit"),
      policyFlagText(policy.failed_before_confirmed_write, "failed before confirmed write"),
      policyFlagText(policy.rollback_attempted_on_write_error, "rollback guard armed"),
      policyFlagText(policy.trusted_source_snapshot_required, "trusted source snapshot required"),
      policyFlagText(policy.stale_source_snapshot_refused, "stale source snapshots refused")
    ].filter(Boolean);
    return lines.slice(0, 6);
  };

  const receiptPolicyLineColor = (line) => {
    if (/refused|failed|Generated|Materialized/.test(line)) return "rgb(251,191,36)";
    if (/allowed|source_owned/.test(line)) return "rgb(134,239,172)";
    return "rgb(148,163,184)";
  };

  const recordHotReloadStatus = (status, reason, endpoint, target) => {
    state.hotReloadStatus = {
      status,
      reason,
      endpoint: limitText(endpoint, 180),
      target: limitText(target, 180)
    };
  };

  const hotReloadStatusLines = () => {
    const status = state.hotReloadStatus;
    if (!status) return [];
    const headline = status.status === "refresh_needed"
      ? "Hot reload refresh needed"
      : status.status === "seen"
        ? "Hot reload seen"
        : "Hot reload polling";
    return [
      headline,
      status.reason ? `Hot reload ${status.reason}` : null,
      status.target ? `Hot target ${status.target}` : null
    ].filter(Boolean).slice(0, 3);
  };

  const hotReloadLineColor = (line) => {
    if (/refresh needed|unavailable|timed out|failed/.test(line)) return "rgb(251,191,36)";
    if (/seen/.test(line)) return "rgb(134,239,172)";
    return "rgb(148,163,184)";
  };

  const drawBreakpointPanel = (overlay, selection, statusText) => {
    const panel = document.createElement("div");
    const props = selection.breakpoint?.editable_responsive_props || [];
    const spacing = selection.style_metrics?.spacing;
    const size = selection.style_metrics?.size;
    const layout = selection.style_metrics?.layout;
    const typography = selection.style_metrics?.typography;
    const visual = selection.style_metrics?.visual;
    const policyLines = receiptSourcePolicyLines(state.lastReceipt);
    const hotReloadLines = hotReloadStatusLines();
    panel.style.position = "fixed";
    panel.style.right = "12px";
    panel.style.bottom = "12px";
    panel.style.maxWidth = "min(360px, calc(100vw - 24px))";
    panel.style.padding = "10px 12px";
    panel.style.border = "1px solid rgba(148, 163, 184, 0.36)";
    panel.style.borderRadius = "7px";
    panel.style.background = "rgba(15, 23, 42, 0.93)";
    panel.style.color = "rgb(226, 232, 240)";
    panel.style.boxShadow = "0 18px 38px rgba(0, 0, 0, 0.34)";
    panel.style.pointerEvents = "none";
    panel.style.transform = "translateZ(0)";
    panel.innerHTML = [
      `<div style="font-weight:600;color:rgb(248,250,252)">DX Studio</div>`,
      `<div style="margin-top:4px;color:rgb(203,213,225)">Breakpoint ${selection.breakpoint.active} - ${selection.breakpoint.width}px</div>`,
      `<div style="margin-top:2px;color:rgb(148,163,184)">Source ${escapeHtml(selection.source_file || "unmapped")}</div>`,
      `<div style="margin-top:2px;color:rgb(148,163,184)">Ops ${escapeHtml((selection.operations || []).join(", ") || "read only")}</div>`,
      selection.design_token ? `<div style="margin-top:2px;color:rgb(148,163,184)">Token ${escapeHtml(selection.design_token)}</div>` : "",
      selection.style_surface ? `<div style="margin-top:2px;color:rgb(148,163,184)">Style ${escapeHtml(selection.style_surface)}</div>` : "",
      selection.reorder_group ? `<div style="margin-top:2px;color:rgb(148,163,184)">Group ${escapeHtml(selection.reorder_group)}</div>` : "",
      size ? `<div style="margin-top:6px;color:rgb(226,232,240)">Size ${escapeHtml(selection.style_metrics.size.inline)}</div>` : "",
      spacing ? `<div style="margin-top:2px;color:rgb(148,163,184)">Margin ${escapeHtml(selection.style_metrics.spacing.margin)}</div>` : "",
      spacing ? `<div style="margin-top:2px;color:rgb(148,163,184)">Padding ${escapeHtml(selection.style_metrics.spacing.padding)}</div>` : "",
      layout ? `<div style="margin-top:2px;color:rgb(148,163,184)">Layout ${escapeHtml(selection.style_metrics.layout.display)}</div>` : "",
      typography ? `<div style="margin-top:2px;color:rgb(148,163,184)">Type ${escapeHtml(typography.font_size)} / ${escapeHtml(typography.line_height)}</div>` : "",
      visual ? `<div style="margin-top:2px;color:rgb(148,163,184)">Visual ${escapeHtml(visual.background_color)} / ${escapeHtml(visual.border_radius)}</div>` : "",
      selection.style_metrics?.status === "missing" ? `<div style="margin-top:6px;color:rgb(251,191,36)">Style metrics missing: ${escapeHtml(selection.style_metrics.reason || "unavailable")}</div>` : "",
      selection.style_edit_plan ? `<div style="margin-top:6px;color:${selection.style_edit_plan.operation ? "rgb(134,239,172)" : "rgb(148,163,184)"}">Style edit ${escapeHtml(selection.style_edit_plan.status)}</div>` : "",
      selection.style_edit_plan?.operation ? `<div style="margin-top:2px;color:rgb(148,163,184)">Style op ${escapeHtml(selection.style_edit_plan.operation)}</div>` : "",
      `<div style="margin-top:2px;color:${props.length ? "rgb(134,239,172)" : "rgb(148,163,184)"}">Responsive props ${props.length ? "declared" : "not declared"}</div>`,
      ...policyLines.map((line) => `<div style="margin-top:2px;color:${receiptPolicyLineColor(line)}">${escapeHtml(line)}</div>`),
      ...hotReloadLines.map((line) => `<div style="margin-top:2px;color:${hotReloadLineColor(line)}">${escapeHtml(line)}</div>`),
      statusText ? `<div style="margin-top:6px;color:rgb(191,219,254)">${escapeHtml(statusText)}</div>` : ""
    ].join("");
    overlay.appendChild(panel);
  };

  const escapeHtml = (value) => String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");

  const persistSelection = (selection) => {
    try {
      sessionStorage.setItem("zed.dxStudio.lastSelection", JSON.stringify({
        edit_id: selection.edit_id,
        text_marker: selection.text_marker,
        insert_slot: selection.insert_slot,
        media_slot: selection.media_slot,
        reorder_group: selection.reorder_group,
        design_token: selection.design_token,
        token_scope: selection.token_scope,
        style_surface: selection.style_surface,
        component: selection.component,
        section: selection.section,
        route: selection.route,
        source_file: selection.source_file,
        timestamp: Date.now()
      }));
    } catch (_error) {}
  };

  const restoredSelectionStatusText = () => {
    if (state.lastReceipt?.status === "source_updated") return "selection restored";
    return state.lastReceipt ? receiptStatusText(state.lastReceipt) : null;
  };

  const restoreLastSelection = () => {
    let stored = null;
    try {
      stored = JSON.parse(sessionStorage.getItem("zed.dxStudio.lastSelection") || "null");
    } catch (_error) {
      stored = null;
    }
    if (!stored) return false;
    const element = elementForSelection(stored);
    if (!element) return false;
    const selection = selectionSnapshot(element);
    drawSelection(selection, restoredSelectionStatusText());
    return true;
  };

