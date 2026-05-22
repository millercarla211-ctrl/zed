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
    const rawTargetAnswer = answer.trim();
    if (!rawTargetAnswer) return null;
    if (!/^\d+$/.test(rawTargetAnswer)) return null;
    const index = Number.parseInt(rawTargetAnswer, 10);
    if (!Number.isSafeInteger(index) || index < 0 || index >= choices.length) return null;
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
