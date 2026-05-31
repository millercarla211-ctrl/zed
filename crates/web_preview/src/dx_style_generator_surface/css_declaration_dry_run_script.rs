const DX_STYLE_CSS_DECLARATION_DRY_RUN_CONSTANTS_SCRIPT: &str = r##"    const cssDeclarationDryRunSchema = cssDeclarationDryRunContract.__schema || "unknown";
    const cssDeclarationDryRunSource = cssDeclarationDryRunContract.__source || "embedded:dx-style-css-declaration-dry-run-contract-fixture";
    const cssDeclarationDryRunMutationEnabled = cssDeclarationDryRunContract.source_mutation_enabled === true;
    const cssDeclarationDryRunContextKind = cssDeclarationDryRunContract.review_context_kind || "css_declaration";
    const cssDeclarationDryRunRequiredFields = Array.isArray(cssDeclarationDryRunContract.required_context_fields)
      ? cssDeclarationDryRunContract.required_context_fields
      : [];
    const cssDeclarationDryRunRequiredHintFields =
      Array.isArray(cssDeclarationDryRunContract.required_css_declaration_hint_fields)
        ? cssDeclarationDryRunContract.required_css_declaration_hint_fields
        : [];
    const cssDeclarationDryRunAcceptedSafety = Array.isArray(cssDeclarationDryRunContract.accepted_source_edit_safety)
      ? cssDeclarationDryRunContract.accepted_source_edit_safety
      : [];
    const cssDeclarationDryRunReviewReceiptFields = Array.isArray(cssDeclarationDryRunContract.review_receipt_fields)
      ? cssDeclarationDryRunContract.review_receipt_fields
      : [];
    const cssDeclarationDryRunMaxDeclarationBytes = Number(cssDeclarationDryRunContract.max_declaration_bytes || 0);
    const cssDeclarationDryRunMaxDiagnosticCount = Number(cssDeclarationDryRunContract.max_diagnostic_count || 0);
    const cssDeclarationDryRunMaxDiagnosticBytes = Number(cssDeclarationDryRunContract.max_diagnostic_bytes || 0);
    const cssDeclarationDryRunMaxSourcePathBytes = Number(cssDeclarationDryRunContract.max_source_path_bytes || 0);
    const cssDeclarationDryRunMaxSourceSpanBytes = Number(cssDeclarationDryRunContract.max_source_span_bytes || 0);
    const cssDeclarationDryRunMaxSourceDigestBytes = Number(cssDeclarationDryRunContract.max_source_digest_bytes || 0);
"##;

const DX_STYLE_CSS_DECLARATION_DRY_RUN_REVIEW_SCRIPT: &str = r##"    function cssDeclarationDryRunDiagnostics(context) {
      if (context?.context_kind !== "css_declaration") return [];
      const diagnostics = [];
      if (cssDeclarationDryRunSchema !== "dx.style.css-declaration-dry-run-contract") {
        diagnostics.push("css_declaration_dry_run_contract_missing");
      }
      if (cssDeclarationDryRunContextKind !== "css_declaration") {
        diagnostics.push("css_declaration_dry_run_context_kind_mismatch");
      }
      if (cssDeclarationDryRunMutationEnabled) {
        diagnostics.push("css_declaration_dry_run_contract_unexpected_mutation");
      }
      diagnostics.push(...cssDeclarationDryRunSourceLimitDiagnostics());
      for (const field of cssDeclarationDryRunRequiredFields) {
        if (context?.[field] === undefined || context?.[field] === null) {
          diagnostics.push(`css_declaration_missing_context_field:${field}`);
        }
      }
      if (cssDeclarationDryRunAcceptedSafety.length
        && !cssDeclarationDryRunAcceptedSafety.includes(context?.css_source_edit_safety || "")) {
        diagnostics.push("css_declaration_source_edit_safety_not_accepted_for_dry_run");
      }
      return diagnostics;
    }

    function cssDeclarationDryRunSourceLimitDiagnostics() {
      const diagnostics = [];
      const limitChecks = [
        {
          limit: cssDeclarationDryRunMaxSourcePathBytes,
          expected: sourceApplyByteLimits.sourcePath,
          missing: "css_declaration_dry_run_missing_source_path_byte_limit",
          mismatch: "css_declaration_dry_run_source_path_byte_limit_mismatch"
        },
        {
          limit: cssDeclarationDryRunMaxSourceSpanBytes,
          expected: sourceApplyByteLimits.sourceSpan,
          missing: "css_declaration_dry_run_missing_source_span_byte_limit",
          mismatch: "css_declaration_dry_run_source_span_byte_limit_mismatch"
        },
        {
          limit: cssDeclarationDryRunMaxSourceDigestBytes,
          expected: sourceApplyByteLimits.sourceDigest,
          missing: "css_declaration_dry_run_missing_source_digest_byte_limit",
          mismatch: "css_declaration_dry_run_source_digest_byte_limit_mismatch"
        }
      ];
      for (const { limit, expected, missing, mismatch } of limitChecks) {
        if (!Number.isInteger(limit) || limit <= 0) {
          diagnostics.push(missing);
        } else if (Number.isInteger(expected) && expected > 0 && limit !== expected) {
          diagnostics.push(mismatch);
        }
      }
      return diagnostics;
    }

    function cssDeclarationDryRunPreviewDiagnostics(preview) {
      const diagnostics = [];
      if (!preview || preview.status === "not_css_declaration_context") return diagnostics;
      if (preview.status !== "ready_for_review") {
        diagnostics.push("css_declaration_dry_run_preview_not_ready");
      }
      if (!Number.isInteger(cssDeclarationDryRunMaxDeclarationBytes) || cssDeclarationDryRunMaxDeclarationBytes <= 0) {
        diagnostics.push("css_declaration_dry_run_missing_declaration_byte_limit");
      } else if (exceedsContractLimit(preview.proposed_declaration || "", cssDeclarationDryRunMaxDeclarationBytes)) {
        diagnostics.push("css_declaration_dry_run_proposed_declaration_exceeds_contract_limit");
      }
      return diagnostics;
    }

    function cssDeclarationDryRunDiagnosticLimitDiagnostics(diagnostics, prefix) {
      const limitDiagnostics = [];
      if (!Number.isInteger(cssDeclarationDryRunMaxDiagnosticCount) || cssDeclarationDryRunMaxDiagnosticCount <= 0) {
        limitDiagnostics.push(`${prefix}_missing_diagnostic_count_limit`);
      } else if (diagnostics.length > cssDeclarationDryRunMaxDiagnosticCount) {
        limitDiagnostics.push(`${prefix}_diagnostics_exceed_contract_limit`);
      }
      if (!Number.isInteger(cssDeclarationDryRunMaxDiagnosticBytes) || cssDeclarationDryRunMaxDiagnosticBytes <= 0) {
        limitDiagnostics.push(`${prefix}_missing_diagnostic_byte_limit`);
      } else if (diagnostics.some((diagnostic) => exceedsContractLimit(diagnostic, cssDeclarationDryRunMaxDiagnosticBytes))) {
        limitDiagnostics.push(`${prefix}_diagnostic_exceeds_contract_limit`);
      }
      return limitDiagnostics;
    }

    function cssDeclarationDryRunBoundedDiagnostics(diagnostics, prefix) {
      return [
        ...diagnostics,
        ...cssDeclarationDryRunDiagnosticLimitDiagnostics(diagnostics, prefix)
      ];
    }

    function cssDeclarationDryRunContextDiagnostics(context) {
      const diagnostics = cssDeclarationDryRunDiagnostics(context);
      return context?.context_kind === "css_declaration"
        ? cssDeclarationDryRunBoundedDiagnostics(diagnostics, "css_declaration_dry_run")
        : diagnostics;
    }

    function cssDeclarationDryRunContextPreviewDiagnostics(preview, context = zedStyleContext) {
      const diagnostics = cssDeclarationDryRunPreviewDiagnostics(preview);
      return context?.context_kind === "css_declaration"
        ? cssDeclarationDryRunBoundedDiagnostics(diagnostics, "css_declaration_dry_run_preview")
        : diagnostics;
    }

    function cssDeclarationDryRunPreview(output, context = zedStyleContext) {
      if (context?.context_kind !== "css_declaration") {
        return {
          status: "not_css_declaration_context",
          reason: "The active Zed context is not a CSS declaration."
        };
      }
      const property = String(context?.css_property || "").trim().toLowerCase();
      if (!property) {
        return {
          status: "missing_css_property",
          reason: "The active CSS declaration context does not name a CSS property."
        };
      }
      const declarations = generatedCssDeclarations(output?.css);
      const declaration = declarations.find((entry) =>
        String(entry.property || "").trim().toLowerCase() === property
      ) || declarations[0] || null;
      if (!declaration) {
        return {
          status: "no_generated_declaration",
          property,
          reason: "The current generator output has no CSS declaration to review."
        };
      }
      const proposedDeclaration = `${declaration.property}: ${declaration.value}`;
      return {
        status: declaration.property.toLowerCase() === property
          ? "ready_for_review"
          : "fallback_generated_declaration",
        property: declaration.property,
        value: declaration.value,
        proposed_declaration: proposedDeclaration,
        css_declaration_hint: cssDeclarationHintPacket(context),
        source_edit_safety: context?.css_source_edit_safety || null,
        reason: declaration.property.toLowerCase() === property
          ? "Generated CSS declaration matches the active CSS property."
          : "Generated CSS declaration is available, but it does not match the active CSS property."
      };
    }

    function generatedCssDeclarations(css) {
      return String(css || "")
        .split(";")
        .map((part) => {
          const index = part.indexOf(":");
          if (index <= 0) return null;
          return {
            property: part.slice(0, index).trim(),
            value: part.slice(index + 1).trim()
          };
        })
        .filter((declaration) => declaration?.property && declaration?.value);
    }
"##;

pub(super) fn dx_style_css_declaration_dry_run_constants_script() -> &'static str {
    DX_STYLE_CSS_DECLARATION_DRY_RUN_CONSTANTS_SCRIPT
}

pub(super) fn dx_style_css_declaration_dry_run_review_script() -> &'static str {
    DX_STYLE_CSS_DECLARATION_DRY_RUN_REVIEW_SCRIPT
}
