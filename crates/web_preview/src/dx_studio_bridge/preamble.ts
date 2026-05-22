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

