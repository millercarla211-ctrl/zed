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
        const rawOperationAnswer = answer.trim();
        if (!rawOperationAnswer) {
          restoreBridgeStateAfterPromptCancel();
          return;
        }
        if (!/^\d+$/.test(rawOperationAnswer)) {
          drawSelection(selection, "operation refused");
          return;
        }
        const index = Number.parseInt(rawOperationAnswer, 10);
        if (!Number.isSafeInteger(index) || index < 0 || index >= operations.length) {
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
