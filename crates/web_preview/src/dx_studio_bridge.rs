pub(crate) const DX_STUDIO_BRIDGE_SCRIPT: &str = r#"
(() => {
  const BRIDGE_KEY = "__zedWebPreviewDxStudio";
  if (window[BRIDGE_KEY]) return;

  const DEFAULT_BREAKPOINTS = {
    xs: 0,
    sm: 640,
    md: 768,
    lg: 1024,
    xl: 1280,
    "2xl": 1536
  };
  const BREAKPOINT_ORDER = ["xs", "sm", "md", "lg", "xl", "2xl"];
  const EDITABLE_SELECTOR = [
    "[data-dx-editable-text]",
    "[data-dx-edit-id]",
    "[data-dx-editable-section]",
    "[data-dx-insert-slot]",
    "[data-dx-reorder-group]",
    "[data-dx-design-token]",
    "[data-dx-media-slot]",
    "[data-dx-component]",
    "[data-dx-section]"
  ].join(",");
  const SURFACE_SELECTOR = [
    "[data-dx-edit-id]",
    "[data-dx-editable-section]",
    "[data-dx-component]",
    "[data-dx-section]",
    "[data-dx-source]",
    "[data-dx-route]"
  ].join(",");

  const state = {
    overlay: null,
    overlayCleanup: null,
    refreshScheduled: false,
    cleanup: null,
    selected: null,
    lastReceipt: null,
    statusText: null,
    hotReloadStatus: null
  };

  const post = (payload) => {
    try {
      window.ipc.postMessage(JSON.stringify(payload));
    } catch (_error) {}
  };

  const limitText = (value, max) => {
    if (value == null) return null;
    const text = String(value).trim();
    return text ? text.slice(0, max) : null;
  };

  const cssSelector = (element) => {
    if (!(element instanceof Element)) return null;
    if (element.id) return `#${CSS.escape(element.id)}`;
    const parts = [];
    let node = element;
    while (node && node.nodeType === Node.ELEMENT_NODE && parts.length < 6) {
      let selector = node.tagName.toLowerCase();
      if (node.classList.length) {
        selector += Array.from(node.classList)
          .slice(0, 3)
          .map((cls) => `.${CSS.escape(cls)}`)
          .join("");
      }
      const parent = node.parentElement;
      if (parent) {
        const siblings = Array.from(parent.children).filter((child) => child.tagName === node.tagName);
        if (siblings.length > 1) {
          selector += `:nth-of-type(${siblings.indexOf(node) + 1})`;
        }
      }
      parts.unshift(selector);
      node = parent;
    }
    return parts.join(" > ");
  };

  const dxAttributes = (element) => {
    const attributes = {};
    if (!(element instanceof Element)) return attributes;
    for (const attribute of Array.from(element.attributes || [])) {
      if ((attribute.name.startsWith("data-dx-") || attribute.name === "data-visual-audit") && attribute.value) {
        attributes[attribute.name] = limitText(attribute.value, 360);
      }
    }
    return attributes;
  };

  const roundedRect = (element) => {
    if (!(element instanceof Element)) return null;
    const rect = element.getBoundingClientRect();
    return {
      x: Math.round(rect.x),
      y: Math.round(rect.y),
      width: Math.round(rect.width),
      height: Math.round(rect.height)
    };
  };

  const fourSideValue = (style, prefix) => {
    const top = style.getPropertyValue(`${prefix}-top`) || "missing";
    const right = style.getPropertyValue(`${prefix}-right`) || "missing";
    const bottom = style.getPropertyValue(`${prefix}-bottom`) || "missing";
    const left = style.getPropertyValue(`${prefix}-left`) || "missing";
    if (top === right && right === bottom && bottom === left) return top;
    return `${top} ${right} ${bottom} ${left}`;
  };

  const computedStyleSnapshot = (element) => {
    if (!(element instanceof Element)) {
      return {
        schema: "zed.web_preview.dx_studio_style_metrics.v1",
        status: "missing",
        reason: "no_element"
      };
    }

    try {
      const style = window.getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      const value = (name) => limitText(style.getPropertyValue(name), 160) || "missing";
      const width = `${Math.round(rect.width)}px`;
      const height = `${Math.round(rect.height)}px`;
      return {
        schema: "zed.web_preview.dx_studio_style_metrics.v1",
        status: "computed",
        source: "dom-computed-style",
        size: {
          width,
          height,
          inline: `${width} x ${height}`,
          box_sizing: value("box-sizing")
        },
        spacing: {
          margin: fourSideValue(style, "margin"),
          padding: fourSideValue(style, "padding"),
          border_width: fourSideValue(style, "border-width")
        },
        layout: {
          display: value("display"),
          position: value("position"),
          gap: value("gap"),
          flex_direction: value("flex-direction"),
          grid_template_columns: value("grid-template-columns"),
          align_items: value("align-items"),
          justify_content: value("justify-content")
        },
        typography: {
          font_size: value("font-size"),
          font_weight: value("font-weight"),
          line_height: value("line-height"),
          color: value("color")
        },
        visual: {
          background_color: value("background-color"),
          border_radius: value("border-radius"),
          opacity: value("opacity"),
          transform: value("transform")
        }
      };
    } catch (error) {
      return {
        schema: "zed.web_preview.dx_studio_style_metrics.v1",
        status: "missing",
        reason: limitText(error?.message || "computed_style_unavailable", 160)
      };
    }
  };

  const nearest = (element, selector) => {
    try {
      return element instanceof Element ? element.closest(selector) : null;
    } catch (_error) {
      return null;
    }
  };

  const firstAttributeInChain = (element, names) => {
    let node = element instanceof Element ? element : null;
    while (node) {
      for (const name of names) {
        const value = node.getAttribute(name);
        if (value) return value;
      }
      node = node.parentElement;
    }
    return null;
  };

  const operationList = (element) => {
    const raw = firstAttributeInChain(element, ["data-dx-edit-ops", "data-dx-operation"]);
    if (!raw) return [];
    return Array.from(new Set(String(raw).split(",").map((operation) => operation.trim()).filter(Boolean)));
  };

  const classTokens = (element) => Array.from(element?.classList || []).slice(0, 80);
  const isResponsiveToken = (token) => /^(xs:|sm:|md:|lg:|xl:|2xl:)/.test(String(token || ""));

  const parseBreakpointRecord = (value) => {
    if (!value) return null;
    if (typeof value === "object") return value;
    try {
      return JSON.parse(String(value));
    } catch (_error) {
      return null;
    }
  };

  const breakpointMap = () => {
    const candidates = [
      window.__DX_STYLE_BREAKPOINTS__,
      window.__dxStyle?.breakpoints,
      document.documentElement?.getAttribute("data-dx-style-breakpoints"),
      document.documentElement?.getAttribute("data-dx-breakpoints"),
      document.querySelector("meta[name='dx-style-breakpoints']")?.getAttribute("content")
    ];
    for (const candidate of candidates) {
      const parsed = parseBreakpointRecord(candidate);
      if (!parsed) continue;
      const next = { ...DEFAULT_BREAKPOINTS };
      for (const name of BREAKPOINT_ORDER) {
        const value = Number(parsed[name]);
        if (Number.isFinite(value) && value >= 0) {
          next[name] = value;
        }
      }
      return { values: next, source: "dx-style" };
    }
    return { values: { ...DEFAULT_BREAKPOINTS }, source: "default" };
  };

  const breakpointSnapshot = (element) => {
    const width = Math.round(window.innerWidth || document.documentElement?.clientWidth || 0);
    const breakpointConfig = breakpointMap();
    const breakpoints = breakpointConfig.values;
    let active = "xs";
    for (const name of BREAKPOINT_ORDER) {
      if (width >= breakpoints[name]) active = name;
    }
    const classNames = classTokens(element);
    const responsiveTokens = classNames.filter(isResponsiveToken);
    const operations = operationList(element);
    const tokenScope = firstAttributeInChain(element, ["data-dx-token-scope", "data-dx-design-token"]);
    const styleSurface = firstAttributeInChain(element, ["data-dx-style-surface"]);
    const sourceOwnsResponsiveProps =
      operations.includes("update_design_token") && (Boolean(tokenScope) || responsiveTokens.length > 0);

    return {
      schema: "zed.web_preview.dx_studio_breakpoint.v1",
      width,
      active,
      breakpoints,
      dx_style_metadata: {
        detected: breakpointConfig.source !== "default",
        source: breakpointConfig.source,
        token_scope: tokenScope,
        style_surface: styleSurface,
        responsive_tokens: responsiveTokens.slice(0, 24)
      },
      responsive_props: responsiveTokens.length
        ? [{
            kind: "class_tokens",
            status: sourceOwnsResponsiveProps ? "contract_declared" : "read_only_detected_tokens",
            tokens: responsiveTokens.slice(0, 24)
          }]
        : [],
      editable_responsive_props: sourceOwnsResponsiveProps
        ? [{
            kind: "class_tokens",
            operation: "update_design_token",
            status: "contract_declared",
            active_breakpoint: active,
            token_scope: tokenScope,
            tokens: responsiveTokens.slice(0, 24)
          }]
        : []
    };
  };

  const styleEditPlan = (element, styleMetrics, breakpoint) => {
    const operations = operationList(element);
    const classNames = classTokens(element);
    const responsiveTokens = classNames.filter(isResponsiveToken);
    const designToken = firstAttributeInChain(element, ["data-dx-design-token"]);
    const tokenScope = firstAttributeInChain(element, ["data-dx-token-scope"]);
    const styleSurface = firstAttributeInChain(element, ["data-dx-style-surface"]);
    const hasDeclaredTokenOperation = operations.includes("update_design_token");
    const hasDeclaredStyleTarget =
      Boolean(designToken) || Boolean(tokenScope) || Boolean(styleSurface) || responsiveTokens.length > 0;
    const status = hasDeclaredTokenOperation
      ? (hasDeclaredStyleTarget ? "token_contract_ready" : "missing_declared_style_contract")
      : "read_only_computed_style";

    return {
      schema: "zed.web_preview.dx_studio_style_edit_plan.v1",
      status,
      operation: status === "token_contract_ready" ? "update_design_token" : null,
      computed_source: styleMetrics?.status || "missing",
      breakpoint: breakpoint?.active || null,
      viewport_width: breakpoint?.width || null,
      design_token: designToken || null,
      token_scope: tokenScope || null,
      style_surface: styleSurface || null,
      responsive_class_tokens: responsiveTokens.slice(0, 24),
      editable_responsive_props: breakpoint?.editable_responsive_props || [],
      computed_values: {
        size: styleMetrics?.size || null,
        spacing: styleMetrics?.spacing || null,
        layout: styleMetrics?.layout || null,
        visual: styleMetrics?.visual || null
      },
      policy: {
        never_write_inline_styles: true,
        requires_declared_dx_style_contract: true
      }
    };
  };

  const hierarchyFor = (element) => {
    const hierarchy = [];
    let node = element instanceof Element ? element : null;
    while (node && hierarchy.length < 10) {
      const attributes = dxAttributes(node);
      if (Object.keys(attributes).length > 0) {
        hierarchy.push({
          selector: cssSelector(node),
          tag: node.tagName.toLowerCase(),
          rect: roundedRect(node),
          attributes,
          source_file: attributes["data-dx-source"] || attributes["data-dx-source-file"] || null,
          edit_id: attributes["data-dx-edit-id"] || null,
          component: attributes["data-dx-component"] || null,
          section: attributes["data-dx-section"] || attributes["data-dx-editable-section"] || null
        });
      }
      node = node.parentElement;
    }
    return hierarchy;
  };

  const nearestParentSurface = (selected) => {
    let node = selected?.parentElement || null;
    while (node) {
      if (node.matches?.(SURFACE_SELECTOR)) return node;
      node = node.parentElement;
    }
    return null;
  };

  const selectedElementFor = (target) => {
    return nearest(target, "[data-dx-editable-text]")
      || nearest(target, EDITABLE_SELECTOR)
      || nearest(target, "[data-dx-source]")
      || nearest(target, "[data-dx-source-file]")
      || nearest(target, "[data-dx-route]")
      || (target instanceof Element ? target : null);
  };

  const editableAncestorElements = (target) => {
    const elements = [];
    let node = target instanceof Element ? target : null;
    while (node) {
      if (node.matches?.(EDITABLE_SELECTOR) || node.matches?.("[data-dx-source],[data-dx-source-file],[data-dx-route]")) {
        if (!elements.includes(node)) elements.push(node);
      }
      node = node.parentElement;
    }
    return elements.slice(0, 8);
  };

  const targetLabel = (element, index) => {
    const attrs = dxAttributes(element);
    return `${index}: ${attrs["data-dx-editable-text"]
      || attrs["data-dx-edit-id"]
      || attrs["data-dx-component"]
      || attrs["data-dx-section"]
      || attrs["data-dx-source"]
      || attrs["data-dx-source-file"]
      || element.tagName.toLowerCase()}`;
  };

  const chooseTargetElement = (target, mode) => {
    const directText = nearest(target, "[data-dx-editable-text]");
    if (mode === "edit_text" && directText) return directText;
    const choices = editableAncestorElements(target);
    if (choices.length <= 1) return choices[0] || selectedElementFor(target);
    const labels = choices.map(targetLabel).join("\n");
    const answer = window.prompt(`Choose DX Studio target:\n${labels}`, "0");
    if (answer == null) return null;
    const index = Number.parseInt(answer || "0", 10);
    if (!Number.isFinite(index) || index < 0 || index >= choices.length) return null;
    return choices[index];
  };

  const selectionSnapshot = (target) => {
    const selected = selectedElementFor(target);
    if (!selected) return null;
    const parent = nearestParentSurface(selected);
    const route = nearest(selected, "[data-dx-route]")
      || document.querySelector("[data-dx-route]")
      || document.documentElement;
    const attributes = dxAttributes(selected);
    const hierarchy = hierarchyFor(selected);
    const textMarker = attributes["data-dx-editable-text"]
      || firstAttributeInChain(selected, ["data-dx-editable-text"]);
    const editId = attributes["data-dx-edit-id"]
      || firstAttributeInChain(selected, ["data-dx-edit-id"]);
    const operations = operationList(selected);
    const sourceFile = attributes["data-dx-source"]
      || attributes["data-dx-source-file"]
      || firstAttributeInChain(selected, ["data-dx-source", "data-dx-source-file"]);
    const hotReloadTarget = attributes["data-dx-hot-reload-target"]
      || firstAttributeInChain(selected, ["data-dx-hot-reload-target", "data-dx-update-target"]);
    const classes = classTokens(selected);
    const breakpoint = breakpointSnapshot(selected);
    const styleMetrics = computedStyleSnapshot(selected);

    return {
      schema: "zed.web_preview.dx_studio_selection.v1",
      timestamp: new Date().toISOString(),
      url: window.location.href,
      title: document.title,
      route: firstAttributeInChain(selected, ["data-dx-route"]) || route?.getAttribute?.("data-dx-route") || null,
      selector: cssSelector(selected),
      tag: selected.tagName.toLowerCase(),
      text: limitText(selected.innerText || selected.textContent, 2000),
      attributes,
      edit_id: editId || null,
      edit_kind: attributes["data-dx-edit-kind"] || firstAttributeInChain(selected, ["data-dx-edit-kind"]),
      text_marker: textMarker || null,
      source_file: sourceFile || null,
      component: attributes["data-dx-component"] || firstAttributeInChain(selected, ["data-dx-component"]),
      section: attributes["data-dx-section"] || attributes["data-dx-editable-section"] || firstAttributeInChain(selected, ["data-dx-section", "data-dx-editable-section"]),
      insert_slot: attributes["data-dx-insert-slot"] || firstAttributeInChain(selected, ["data-dx-insert-slot"]),
      reorder_group: attributes["data-dx-reorder-group"] || firstAttributeInChain(selected, ["data-dx-reorder-group"]),
      design_token: attributes["data-dx-design-token"] || firstAttributeInChain(selected, ["data-dx-design-token"]),
      token_scope: attributes["data-dx-token-scope"] || firstAttributeInChain(selected, ["data-dx-token-scope"]),
      style_surface: attributes["data-dx-style-surface"] || firstAttributeInChain(selected, ["data-dx-style-surface"]),
      media_slot: attributes["data-dx-media-slot"] || firstAttributeInChain(selected, ["data-dx-media-slot"]),
      class_tokens: classes,
      responsive_class_tokens: classes.filter(isResponsiveToken),
      operations,
      hot_reload_target: hotReloadTarget || null,
      hot_reload_version_endpoint: "/_dx/hot-reload/version",
      rect: roundedRect(selected),
      parent: parent ? {
        selector: cssSelector(parent),
        tag: parent.tagName.toLowerCase(),
        rect: roundedRect(parent),
        attributes: dxAttributes(parent)
      } : null,
      route_root: route instanceof Element ? {
        selector: cssSelector(route),
        tag: route.tagName.toLowerCase(),
        rect: roundedRect(route),
        attributes: dxAttributes(route)
      } : null,
      hierarchy,
      breakpoint,
      style_metrics: styleMetrics,
      style_edit_plan: styleEditPlan(selected, styleMetrics, breakpoint),
      source_policy: {
        nearest_editable_wins: true,
        generated_runtime_files_require_manifest_permission: true,
        writes_require_rust_source_receipt: true
      }
    };
  };

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

  const beginCapture = (mode) => {
    const previousBridgeState = {
      selected: state.selected,
      statusText: state.statusText,
      lastReceipt: state.lastReceipt,
      hotReloadStatus: state.hotReloadStatus
    };
    if (state.cleanup) {
      try { state.cleanup(); } catch (_error) {}
    }
    const hoverOverlay = overlayRoot();
    hoverOverlay.style.cursor = "crosshair";
    const hoverBox = document.createElement("div");
    hoverBox.style.position = "fixed";
    hoverBox.style.border = "2px dashed rgba(96, 165, 250, 0.92)";
    hoverBox.style.background = "rgba(96, 165, 250, 0.08)";
    hoverBox.style.borderRadius = "6px";
    hoverBox.style.pointerEvents = "none";
    hoverOverlay.appendChild(hoverBox);

    const move = (event) => {
      const element = selectedElementFor(event.target);
      if (!element) return;
      const rect = element.getBoundingClientRect();
      hoverBox.style.left = `${Math.round(rect.left)}px`;
      hoverBox.style.top = `${Math.round(rect.top)}px`;
      hoverBox.style.width = `${Math.round(rect.width)}px`;
      hoverBox.style.height = `${Math.round(rect.height)}px`;
    };

    const cleanup = () => {
      document.removeEventListener("mousemove", move, true);
      document.removeEventListener("click", click, true);
      document.removeEventListener("keydown", keydown, true);
      if (state.overlay === hoverOverlay) {
        hoverOverlay.remove();
        state.overlay = null;
      }
      state.cleanup = null;
    };

    const compactUnique = (values) => Array.from(new Set(
      values
        .filter(Boolean)
        .map((value) => String(value).trim())
        .filter(Boolean)
    ));

    const computedStyleSummary = (stylePlan) => {
      const values = stylePlan?.computed_values || {};
      const summary = [
        values.size?.inline ? `size ${values.size.inline}` : null,
        values.spacing?.margin ? `margin ${values.spacing.margin}` : null,
        values.spacing?.padding ? `padding ${values.spacing.padding}` : null,
        values.layout?.display ? `display ${values.layout.display}` : null,
        values.visual?.background_color ? `bg ${values.visual.background_color}` : null
      ].filter(Boolean).join("; ");
      return limitText(summary, 220);
    };

    const styleTokenEditDefaults = (selection) => {
      const plan = selection.style_edit_plan || null;
      const status = plan?.status || "missing_style_edit_plan";
      const tokenCandidates = compactUnique([
        plan?.design_token,
        ...(plan?.responsive_class_tokens || []),
        selection.design_token,
        ...(selection.responsive_class_tokens || []),
        selection.token_scope,
        ...(selection.class_tokens || [])
      ]);
      const computedSummary = computedStyleSummary(plan);
      const receipt = {
        schema: "zed.web_preview.dx_studio_style_token_prefill.v1",
        source: plan?.schema ? "style_edit_plan" : "selection",
        status,
        operation: plan?.operation || "update_design_token",
        token_candidates: tokenCandidates.slice(0, 12),
        computed_summary: computedSummary,
        active_breakpoint: plan?.breakpoint || selection.breakpoint?.active || null,
        viewport_width: plan?.viewport_width || selection.breakpoint?.width || null,
        design_token: plan?.design_token || selection.design_token || null,
        token_scope: plan?.token_scope || selection.token_scope || null,
        style_surface: plan?.style_surface || selection.style_surface || null,
        policy: {
          rust_source_edit_must_verify_contract: true,
          no_inline_style_write: true
        }
      };
      return { status, tokenCandidates, computedSummary, receipt };
    };

    const restoreBridgeStateAfterPromptCancel = () => {
      state.lastReceipt = previousBridgeState.lastReceipt;
      state.hotReloadStatus = previousBridgeState.hotReloadStatus;
      if (!previousBridgeState.selected) {
        clearOverlay();
        return;
      }

      const element = elementForSelection(previousBridgeState.selected);
      const selection = element ? selectionSnapshot(element) || previousBridgeState.selected : previousBridgeState.selected;
      drawSelection(selection, previousBridgeState.statusText);
    };

    const promptCancelledSelectionStatus = () => {
      return state.lastReceipt ? receiptStatusText(state.lastReceipt) : null;
    };

    const restoreSelectionAfterValuePromptCancel = (selection) => {
      drawSelection(selection, promptCancelledSelectionStatus());
    };

    const promptOperation = (selection, preferredOperation = null) => {
      let operations = Array.from(new Set(selection.operations || []));
      if (selection.text_marker && !operations.includes("update_text_content")) {
        operations.unshift("update_text_content");
      }
      if (!operations.length) {
        drawSelection(selection, "no edit operation");
        return;
      }

      let operation = preferredOperation;
      if (!operation || !operations.includes(operation)) {
        const labels = operations.map((candidate, index) => `${index}: ${candidate}`).join("\n");
        const answer = window.prompt(`Choose DX Studio operation:\n${labels}`, "0");
        if (answer == null) {
          restoreBridgeStateAfterPromptCancel();
          return;
        }
        const index = Number.parseInt(answer || "0", 10);
        if (!Number.isFinite(index) || index < 0 || index >= operations.length) {
          drawSelection(selection, "operation refused");
          return;
        }
        operation = operations[index];
      }

      if (operation === "update_text_content") {
        if (!selection.text_marker && !operations.includes("update_text_content")) {
          drawSelection(selection, "no text contract");
          return;
        }
        const current = selection.text || "";
        const replacement = window.prompt("DX Studio text", current);
        if (replacement == null || replacement === current) {
          restoreSelectionAfterValuePromptCancel(selection);
          return;
        }
        void sendEditRequest(selection, {
          kind: "dx-studio-edit-request",
          action: "apply",
          operation,
          timestamp: new Date().toISOString(),
          url: window.location.href,
          title: document.title,
          selection,
          edit: {
            previous_text: current,
            replacement_text: replacement
          }
        }, "writing text source");
        return;
      }

      if (operation === "update_design_token") {
        const defaults = styleTokenEditDefaults(selection);
        const tokenCandidates = defaults.tokenCandidates;
        const stylePrompt = [
          `Style contract ${defaults.status || "unknown"}`,
          defaults.computedSummary ? `Computed ${defaults.computedSummary}` : null,
          "Rust source edit verifies declared markers before writing."
        ].filter(Boolean).join("\n");
        const oldToken = window.prompt(`DX Studio token to replace\n${stylePrompt}`, tokenCandidates[0] || "");
        if (!oldToken) {
          restoreSelectionAfterValuePromptCancel(selection);
          return;
        }
        const newToken = window.prompt(`DX Studio replacement token\n${stylePrompt}`, oldToken);
        if (!newToken || newToken === oldToken) {
          restoreSelectionAfterValuePromptCancel(selection);
          return;
        }
        const responsiveLayoutEdit = isResponsiveToken(oldToken) || isResponsiveToken(newToken);
        void sendEditRequest(selection, {
          kind: "dx-studio-edit-request",
          action: "apply",
          operation,
          timestamp: new Date().toISOString(),
          url: window.location.href,
          title: document.title,
          selection,
          edit: {
            old_token: oldToken,
            new_token: newToken,
            style_edit_plan: selection.style_edit_plan || null,
            style_edit_prefill: defaults.receipt,
            computed_summary: defaults.computedSummary,
            responsive_layout: responsiveLayoutEdit
              ? {
                  active_breakpoint: selection.breakpoint?.active || null,
                  viewport_width: selection.breakpoint?.width || null,
                  token_scope: selection.token_scope || selection.design_token || null,
                  responsive_policy: "use-existing-grid-and-design-tokens"
                }
              : null
          }
        }, "writing token source");
        return;
      }

      if (operation === "move_reorder_section") {
        const direction = window.prompt("Move DX section: up or down", "down");
        if (!direction) {
          restoreSelectionAfterValuePromptCancel(selection);
          return;
        }
        const normalizedDirection = direction.trim().toLowerCase();
        if (normalizedDirection !== "up" && normalizedDirection !== "down") {
          drawSelection(selection, "reorder direction refused");
          return;
        }
        void sendEditRequest(selection, {
          kind: "dx-studio-edit-request",
          action: "apply",
          operation,
          timestamp: new Date().toISOString(),
          url: window.location.href,
          title: document.title,
          selection,
          edit: {
            direction: normalizedDirection,
            reorder_group: selection.reorder_group || selection.attributes?.["data-dx-reorder-group"] || null
          }
        }, "reordering source");
        return;
      }

      void sendEditRequest(selection, {
        kind: "dx-studio-edit-request",
        action: "apply",
        operation,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        selection,
        edit: {
          requires_declared_source_template: true,
          insert_slot: selection.insert_slot || null,
          media_slot: selection.media_slot || null
        }
      }, "needs source template");
    };

    const click = (event) => {
      event.preventDefault();
      event.stopPropagation();
      const target = chooseTargetElement(event.target, mode);
      cleanup();
      if (!target) {
        restoreBridgeStateAfterPromptCancel();
        return;
      }
      const selection = selectionSnapshot(target);
      if (!selection) {
        restoreBridgeStateAfterPromptCancel();
        return;
      }
      drawSelection(selection, mode === "edit_text" ? "text edit" : mode === "edit_operation" ? "edit operation" : "selected");
      post({
        kind: "dx-studio-selection",
        action: mode,
        selection
      });

      if (mode === "edit_text") {
        promptOperation(selection, "update_text_content");
      } else if (mode === "edit_operation") {
        promptOperation(selection);
      }
    };

    const keydown = (event) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      cleanup();
      restoreBridgeStateAfterPromptCancel();
    };

    document.addEventListener("mousemove", move, true);
    document.addEventListener("click", click, true);
    document.addEventListener("keydown", keydown, true);
    state.cleanup = cleanup;
  };

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
"#;
