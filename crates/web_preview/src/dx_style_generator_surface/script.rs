use super::css_declaration_dry_run_script::{
    dx_style_css_declaration_dry_run_constants_script,
    dx_style_css_declaration_dry_run_review_script,
};
use super::source_apply_session_script::{
    dx_style_source_apply_session_constants_script, dx_style_source_apply_session_handler_script,
};

const DX_STYLE_GENERATOR_SCRIPT: &str = r##"    const catalogPayload = __DX_STYLE_GENERATOR_CATALOG_JSON__;
    const catalog = Array.isArray(catalogPayload) ? catalogPayload : catalogPayload.entries || [];
    const controls = __DX_STYLE_GENERATOR_CONTROLS_JSON__;
    const recipes = __DX_STYLE_GENERATOR_RECIPES_JSON__;
    const sourceApplyContract = __DX_STYLE_SOURCE_APPLY_CONTRACT_JSON__;
    const cssDeclarationDryRunContract = __DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_JSON__;
    const groupContextContract = __DX_STYLE_GROUP_CONTEXT_CONTRACT_JSON__;
    const reverseCssDeltaContract = __DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_JSON__;

    const state = {
      generator: "linear-gradient",
      from: "#38bdf8",
      to: "#22c55e",
      angle: 120,
      radius: 18,
      blur: 36,
      columns: 3,
      gap: 16,
      duration: 700,
      easing: "cubic-bezier(.22,1,.36,1)",
      catalogQuery: ""
    };
    const sourceContextPayload = __DX_STYLE_CONTEXT_JSON_STRING__;
    const catalogSchema = catalogPayload.__schema || "unknown";
    const catalogSource = catalogPayload.__source || "embedded:dx-style-catalog-fixture";
    const controlSchema = controls.__schema || "unknown";
    const controlSource = controls.__source || "embedded:dx-style-control-fixture";
    const recipeSchema = recipes.__schema || "unknown";
    const recipeSource = recipes.__source || "embedded:dx-style-recipe-fixture";
    const recipeValueKeys = Array.isArray(recipes.__value_keys) ? recipes.__value_keys : [];
    const recipePreviewAnatomyParts = Array.isArray(recipes.__preview_anatomy_parts)
      ? recipes.__preview_anatomy_parts
      : [];
    const recipePreviewAnatomyPartSet = new Set(recipePreviewAnatomyParts);
    const sourceApplyContractSchema = sourceApplyContract.__schema || "unknown";
    const sourceApplyContractSource = sourceApplyContract.__source || "embedded:dx-style-source-apply-contract-fixture";
    const sourceApplyContractVersion = sourceApplyContract.schema_version || "unknown";
    const sourceApplyContractScope = sourceApplyContract.scope || "unknown";
    const sourceApplyIpcKind = sourceApplyContract.ipc_kind || "dx-style-source-apply";
    const sourceApplyReceiptSchema = sourceApplyContract.receipt_schema || "unknown";
__DX_STYLE_SOURCE_APPLY_SESSION_CONSTANTS__
    const sourceApplyMutationEnabled = sourceApplyContract.source_mutation_enabled === true;
    const sourceApplyRequiredHandlerCapabilities = Array.isArray(sourceApplyContract.required_native_handler_capabilities)
      ? sourceApplyContract.required_native_handler_capabilities
      : [];
    const sourceApplyReviewContextKinds = Array.isArray(sourceApplyContract.review_context_kinds)
      ? sourceApplyContract.review_context_kinds
      : [];
    const sourceApplyMutationContextKinds = Array.isArray(sourceApplyContract.mutation_context_kinds_when_enabled)
      ? sourceApplyContract.mutation_context_kinds_when_enabled
      : [];
    const sourceApplyReviewReceiptFields = Array.isArray(sourceApplyContract.review_receipt_fields)
      ? sourceApplyContract.review_receipt_fields
      : [];
    const sourceApplyRequiredEditorGuards = Array.isArray(sourceApplyContract.required_editor_guards)
      ? sourceApplyContract.required_editor_guards
      : [];
    const reverseCssDeltaReplacementPolicyGuard = "reverse CSS delta replacement policy match";
__DX_STYLE_CSS_DECLARATION_DRY_RUN_CONSTANTS__
    const groupContextContractSchema = groupContextContract.__schema || "unknown";
    const groupContextContractSource = groupContextContract.__source || "embedded:dx-style-group-context-fixture";
    const groupContextMaxAliasBytes = Number(groupContextContract.max_alias_bytes || 0);
    const groupContextMaxUtilityCount = Number(groupContextContract.max_utility_count || 0);
    const groupContextMaxUtilityBytes = Number(groupContextContract.max_utility_bytes || 0);
    const groupContextCandidateMin = Number(groupContextContract.candidate_min_utility_count || 0);
    const reverseCssDeltaContractSchema = reverseCssDeltaContract.__schema || "unknown";
    const reverseCssDeltaContractSource = reverseCssDeltaContract.__source || "embedded:dx-style-reverse-css-delta-contract-fixture";
    const reverseCssDeltaMutationEnabled = reverseCssDeltaContract.source_mutation_enabled === true;
    const reverseCssDeltaSupportedProperties = Array.isArray(reverseCssDeltaContract.supported_properties)
      ? reverseCssDeltaContract.supported_properties
      : [];
    const reverseCssDeltaRequiredGuards = Array.isArray(reverseCssDeltaContract.required_editor_guards)
      ? reverseCssDeltaContract.required_editor_guards
      : [];
    const reverseCssDeltaRequiredProvenanceFields = Array.isArray(reverseCssDeltaContract.required_preview_provenance_fields)
      ? reverseCssDeltaContract.required_preview_provenance_fields
      : [];
    const reverseCssDeltaFallbackReviewProperties = Array.isArray(reverseCssDeltaContract.fallback_review_properties)
      ? reverseCssDeltaContract.fallback_review_properties
      : [];
    const reverseCssDeltaFallbackReviewPropertySet = new Set(
      reverseCssDeltaFallbackReviewProperties.map((property) => String(property || "").toLowerCase())
    );
    const reverseCssDeltaExistingUtilityRequiredProperties = Array.isArray(reverseCssDeltaContract.existing_utility_required_properties)
      ? reverseCssDeltaContract.existing_utility_required_properties
      : [];
    const reverseCssDeltaExistingUtilityRequiredPropertySet = new Set(
      reverseCssDeltaExistingUtilityRequiredProperties.map((property) => String(property || "").toLowerCase())
    );
    const reverseCssDeltaPayloadLimits = {
      replacementUtilities: contractNumberLimit(reverseCssDeltaContract, "max_replacement_utilities"),
      replacementUtilityBytes: contractNumberLimit(reverseCssDeltaContract, "max_replacement_utility_bytes"),
      replacementSourceDeclarationBytes: contractNumberLimit(reverseCssDeltaContract, "max_replacement_source_declaration_bytes")
    };
    const reverseCssDeltaSupportedProvenanceFields = new Set([
      "group_status",
      "group_alias",
      "group_syntax",
      "group_expansion_status",
      "group_registry_receipt",
      "reverse_css_map_receipt",
      "reverse_css_map_status",
      "group_source_state",
      "group_utility_count"
    ]);
    const reverseCssDeltaExample = reverseCssDeltaContract.example_preview || null;
    const sourceApplyByteLimits = {
      sourcePath: contractByteLimit("max_source_path_bytes"),
      className: contractByteLimit("max_class_name_bytes"),
      css: contractByteLimit("max_css_bytes"),
      generator: contractByteLimit("max_generator_id_bytes"),
      sourceSpan: contractByteLimit("max_source_span_bytes"),
      sourceDigest: contractByteLimit("max_source_digest_bytes"),
      previewKind: contractByteLimit("max_preview_kind_bytes"),
      previewAnatomyPart: contractByteLimit("max_preview_anatomy_part_bytes"),
      previewAnatomyParts: contractByteLimit("max_preview_anatomy_parts"),
      dryRunEditPreviews: contractByteLimit("max_dry_run_edit_previews"),
      dryRunReplacementText: contractByteLimit("max_dry_run_replacement_text_bytes")
    };
    const sourceDigestPrefix = "fnv1a64:";
    const expectedContextSchema = sourceApplyContract.active_context_schema || "zed.dx_style.active_context.v1";
    const metadataDiagnostics = generatorMetadataDiagnostics();
    const textEncoder = new TextEncoder();
    let zedStyleContext = null;
    try {
      zedStyleContext = sourceContextPayload ? JSON.parse(sourceContextPayload) : null;
    } catch (_) {
      zedStyleContext = null;
    }
    const contextGeneratorHint = generatorForContext(zedStyleContext);
    const contextGenerator = contextGeneratorHint?.id || null;
    const contextGeneratorSource = contextGeneratorHint?.source || null;
    if (contextGenerator) state.generator = contextGenerator;
    installSourceApplyHandler();

    const catalogEl = document.getElementById("catalog");
    const generatorSearchEl = document.getElementById("generatorSearch");
    const selectEl = document.getElementById("generatorSelect");
    const controlsEl = document.getElementById("controls");
    const sampleEl = document.getElementById("sample");
    const outputEl = document.getElementById("output");
    const sourceStatusEl = document.getElementById("sourceStatus");
    const metadataStatusEl = document.getElementById("metadataStatus");
    const applyButtonEl = document.getElementById("applyButton");
    const reviewApplyButtonEl = document.getElementById("reviewApplyButton");
    const copyClassButtonEl = document.getElementById("copyClassButton");
    const copyCssButtonEl = document.getElementById("copyCssButton");
    const copyReviewButtonEl = document.getElementById("copyReviewButton");
    const patchReviewEl = document.getElementById("patchReview");

    function setGenerator(generator) {
      state.generator = generator;
      render();
    }

    function generatorForToken(token) {
      if (!token) return null;
      const match = catalog.find(([, , , hints = []]) =>
        hints.some((hint) => tokenMatchesHint(token, hint))
      );
      return match?.[0] || null;
    }

    function generatorForContext(context) {
      if (context?.css_generator && catalogHasGenerator(context.css_generator)) {
        return { id: context.css_generator, source: "css_declaration" };
      }
      const groupUtilities = Array.isArray(context?.group_context?.utilities)
        ? context.group_context.utilities
        : [];
      for (const token of groupUtilities) {
        const generator = generatorForToken(token);
        if (generator) return { id: generator, source: "group_utilities" };
      }
      const direct = generatorForToken(context?.token || "");
      if (direct) return { id: direct, source: "active_token" };
      const tokens = Array.isArray(context?.attribute_tokens) ? context.attribute_tokens : [];
      for (const token of tokens) {
        const generator = generatorForToken(token);
        if (generator) return { id: generator, source: "attribute_tokens" };
      }
      return null;
    }

    function tokenMatchesHint(token, hint) {
      if (!hint) return false;
      const startsWildcard = hint.startsWith("*");
      const endsWildcard = hint.endsWith("*");
      const pattern = hint.replace(/^\*/, "").replace(/\*$/, "");
      if (startsWildcard && endsWildcard) return token.includes(pattern);
      if (startsWildcard) return token.endsWith(pattern);
      if (endsWildcard) return token.startsWith(pattern);
      if (hint.includes("*")) {
        const escaped = hint.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*");
        return new RegExp(`^${escaped}$`).test(token);
      }
      return token === hint;
    }

    function orderedCatalog() {
      if (!contextGenerator) return catalog;
      const active = catalog.find(([id]) => id === contextGenerator);
      if (!active) return catalog;
      return [active, ...catalog.filter(([id]) => id !== contextGenerator)];
    }

    function catalogSearchText(entry) {
      const [id, label, category, hints = [], preferredOutput, sourceEditSafety] = entry || [];
      return [id, label, category, preferredOutput, sourceEditSafety, ...(Array.isArray(hints) ? hints : [])]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
    }

    function catalogEntryForGenerator(generatorId) {
      return catalog.find(([id]) => id === generatorId) || null;
    }

    function catalogMetadataForGenerator(generatorId) {
      const entry = catalogEntryForGenerator(generatorId);
      return {
        category: entry?.[2] || "unknown",
        preferredOutput: entry?.[4] || "unknown",
        sourceEditSafety: entry?.[5] || "unknown"
      };
    }

    function filteredCatalog() {
      const query = state.catalogQuery.trim().toLowerCase();
      if (!query) return orderedCatalog();
      const terms = query.split(/\s+/).filter(Boolean);
      return orderedCatalog().filter((entry) => {
        const haystack = catalogSearchText(entry);
        return terms.every((term) => haystack.includes(term));
      });
    }

    function catalogGeneratorIds() {
      return catalog
        .map(([id]) => id)
        .filter((id) => typeof id === "string" && id.length > 0);
    }

    function catalogHasGenerator(generatorId) {
      return catalog.some(([id]) => id === generatorId);
    }

    function metadataKeys(source) {
      return Object.keys(source || {}).filter((id) =>
        id !== "default" && !id.startsWith("__")
      );
    }

    function templatePlaceholderKeys(template) {
      const keys = new Set();
      for (const match of String(template || "").matchAll(/\{([a-z0-9_]+)\}/g)) {
        keys.add(match[1]);
      }
      return [...keys];
    }

    function generatorMetadataDiagnostics() {
      const ids = catalogGeneratorIds();
      const catalogIdSet = new Set(ids);
      const recipeValueKeySet = new Set(recipeValueKeys);
      const missingControls = ids.filter((id) => !Array.isArray(controls[id]?.controls));
      const missingRecipes = ids.filter((id) =>
        !recipes[id]?.classTemplate || !recipes[id]?.cssTemplate
      );
      const extraControls = metadataKeys(controls).filter((id) => !catalogIdSet.has(id));
      const extraRecipes = metadataKeys(recipes).filter((id) => !catalogIdSet.has(id));
      const unsupportedRecipePlaceholders = ids.flatMap((id) => {
        const recipe = recipes[id] || {};
        const placeholders = templatePlaceholderKeys(
          `${recipe.classTemplate || ""}\n${recipe.cssTemplate || ""}`
        );
        return placeholders
          .filter((key) => !recipeValueKeySet.has(key))
          .map((key) => `${id}:${key}`);
      });
      const missingPreviewAnatomy = ids.filter((id) =>
        !Array.isArray(recipes[id]?.previewAnatomy) || !recipes[id].previewAnatomy.length
      );
      const unsupportedPreviewAnatomy = ids.flatMap((id) => {
        const parts = Array.isArray(recipes[id]?.previewAnatomy)
          ? recipes[id].previewAnatomy
          : [];
        return parts
          .filter((part) => !recipePreviewAnatomyPartSet.has(part))
          .map((part) => `${id}:${part}`);
      });
      const hasMetadataDrift =
        missingControls.length
        || missingRecipes.length
        || extraControls.length
        || extraRecipes.length
        || unsupportedRecipePlaceholders.length
        || missingPreviewAnatomy.length
        || unsupportedPreviewAnatomy.length;
      return {
        status: hasMetadataDrift ? "drift" : "aligned",
        generatorCount: ids.length,
        missingControls,
        missingRecipes,
        extraControls,
        extraRecipes,
        unsupportedRecipePlaceholders,
        missingPreviewAnatomy,
        unsupportedPreviewAnatomy
      };
    }

    function controlFromDefinition(definition) {
      if (!definition?.key || !definition?.label) return null;
      const inputType = definition.input === "range" || definition.input === "color"
        ? definition.input
        : "text";
      const wrapper = document.createElement("label");
      wrapper.className = "control";
      wrapper.textContent = definition.label;
      const input = document.createElement("input");
      input.type = inputType;
      input.value = state[definition.key] ?? "";
      for (const name of ["min", "max", "step"]) {
        if (definition[name] !== undefined) input.setAttribute(name, definition[name]);
      }
      input.addEventListener("input", () => {
        state[definition.key] = inputType === "range" ? Number(input.value) : input.value;
        updatePreview();
      });
      wrapper.append(input);
      return wrapper;
    }

    function activeControlDefinitions() {
      const definitions = controls[state.generator]?.controls || controls.default?.controls || [];
      return Array.isArray(definitions) ? definitions : [];
    }

    function renderControls() {
      controlsEl.replaceChildren();
      for (const definition of activeControlDefinitions()) {
        const control = controlFromDefinition(definition);
        if (control) controlsEl.append(control);
      }
    }

    function generatorOutput() {
      const [id] = catalog.find(([candidate]) => candidate === state.generator) || [];
      const recipe = activeRecipe(id);
      if (!recipe) {
        return {
          className: "",
          css: "",
          previewKind: "hero-card",
          previewAnatomy: ["preview-label"],
          recipeMissing: true
        };
      }
      const values = recipeValues();
      return {
        className: applyRecipeTemplate(recipe.classTemplate, values),
        css: applyRecipeTemplate(recipe.cssTemplate, values),
        previewKind: recipe.previewKind || "hero-card",
        previewAnatomy: previewAnatomy(recipe)
      };
    }

    function activeRecipe(id) {
      return recipes[id] || null;
    }

    function previewAnatomy(recipe) {
      const parts = Array.isArray(recipe?.previewAnatomy)
        ? recipe.previewAnatomy.filter((part) => typeof part === "string" && part.length)
        : [];
      return parts.length ? parts : fallbackPreviewAnatomy(recipe?.previewKind || "hero-card");
    }

    function fallbackPreviewAnatomy(kind) {
      switch (kind) {
        case "layout-items":
          return ["layout-items"];
        case "timeline":
          return ["timeline-track", "timeline-label"];
        case "swatch-pair":
          return ["swatch-row", "color-transition-label"];
        case "text-card":
          return ["preview-title", "preview-subtitle"];
        default:
          return ["preview-label"];
      }
    }

    function recipeValues() {
      const halfBlur = Math.round(state.blur / 2);
      const thirdBlur = Math.round(state.blur / 3);
      return {
        from: state.from,
        to: state.to,
        angle: state.angle,
        radius: state.radius,
        blur: state.blur,
        columns: state.columns,
        gap: state.gap,
        duration: state.duration,
        easing: state.easing,
        half_blur: halfBlur,
        third_blur: thirdBlur,
        blur_eighth: Math.round(state.blur / 8),
        blur_quarter: Math.round(state.blur / 4),
        text_shadow_blur: Math.max(1, thirdBlur),
        glass_blur: Math.max(8, halfBlur),
        angle_sixth: Math.round(state.angle / 6),
        angle_quarter: Math.round(state.angle / 4),
        gap_plus_20: state.gap + 20,
        css_linear: `linear-gradient(${state.angle}deg, ${state.from}, ${state.to})`,
        css_radial: `radial-gradient(circle at center, ${state.from}, ${state.to})`,
        css_conic: `conic-gradient(from ${state.angle}deg, ${state.from}, ${state.to}, ${state.from})`,
        css_mesh: `radial-gradient(at 18% 22%, ${state.from}, transparent 52%), radial-gradient(at 82% 18%, ${state.to}, transparent 48%), radial-gradient(at 54% 82%, #a78bfa, transparent 46%), #0f172a`,
        css_noise: `radial-gradient(circle at 20% 20%, ${state.from}33 0 2px, transparent 3px), radial-gradient(circle at 80% 40%, ${state.to}33 0 2px, transparent 3px), #0f172a`
      };
    }

    function applyRecipeTemplate(template, values) {
      return String(template || "").replace(/\{([a-z0-9_]+)\}/g, (_, key) =>
        values[key] === undefined ? `{${key}}` : String(values[key])
      );
    }

    function contractNumberLimit(contract, field) {
      const value = Number(contract?.[field]);
      return Number.isFinite(value) && value > 0 ? value : null;
    }

    function contractByteLimit(field) {
      return contractNumberLimit(sourceApplyContract, field);
    }

    function utf8ByteLength(value) {
      return textEncoder.encode(String(value || "")).length;
    }

    function sourceSpanByteLength(context) {
      const start = context?.source_span?.start_byte;
      const end = context?.source_span?.end_byte;
      return Number.isInteger(start) && Number.isInteger(end) && end >= start
        ? end - start
        : null;
    }

    function sourceLengthReady(context) {
      return Number.isInteger(context?.source_len_bytes) && context.source_len_bytes >= 0;
    }

    function sourceSpanReady(context) {
      const start = context?.source_span?.start_byte;
      const end = context?.source_span?.end_byte;
      if (!Number.isInteger(start) || !Number.isInteger(end)) return false;
      if (start > end) return false;
      if (!sourceLengthReady(context)) return false;
      return end <= context.source_len_bytes;
    }

    function exceedsContractLimit(value, limit) {
      return Number.isInteger(limit) && utf8ByteLength(value) > limit;
    }

    function sourceApplyPayloadDiagnostics(context, output) {
      const diagnostics = [];
      if (exceedsContractLimit(state.generator, sourceApplyByteLimits.generator)) {
        diagnostics.push("generator_id_exceeds_contract_limit");
      }
      if (output?.recipeMissing) {
        diagnostics.push("generator_recipe_missing");
      }
      if (exceedsContractLimit(context?.source_path || "", sourceApplyByteLimits.sourcePath)) {
        diagnostics.push("source_path_exceeds_contract_limit");
      }
      if (exceedsContractLimit(output?.className || "", sourceApplyByteLimits.className)) {
        diagnostics.push("class_name_exceeds_contract_limit");
      }
      if (exceedsContractLimit(output?.css || "", sourceApplyByteLimits.css)) {
        diagnostics.push("css_exceeds_contract_limit");
      }
      if (exceedsContractLimit(context?.source_digest || "", sourceApplyByteLimits.sourceDigest)) {
        diagnostics.push("source_digest_exceeds_contract_limit");
      }
      if (exceedsContractLimit(output?.previewKind || "", sourceApplyByteLimits.previewKind)) {
        diagnostics.push("preview_kind_exceeds_contract_limit");
      }
      const previewAnatomy = Array.isArray(output?.previewAnatomy) ? output.previewAnatomy : [];
      if (Number.isInteger(sourceApplyByteLimits.previewAnatomyParts)
        && previewAnatomy.length > sourceApplyByteLimits.previewAnatomyParts) {
        diagnostics.push("preview_anatomy_parts_exceeds_contract_limit");
      }
      if (Number.isInteger(sourceApplyByteLimits.previewAnatomyPart)
        && previewAnatomy.some((part) => exceedsContractLimit(part, sourceApplyByteLimits.previewAnatomyPart))) {
        diagnostics.push("preview_anatomy_part_exceeds_contract_limit");
      }
      diagnostics.push(
        ...reverseCssDeltaReplacementPayloadDiagnostics(reverseCssDeltaPreview(output))
      );
      const sourceSpanBytes = sourceSpanByteLength(context);
      if (Number.isInteger(sourceApplyByteLimits.sourceSpan)
        && Number.isInteger(sourceSpanBytes)
        && sourceSpanBytes > sourceApplyByteLimits.sourceSpan) {
        diagnostics.push("source_span_exceeds_contract_limit");
      }
      if (Number.isInteger(context?.source_span?.start_byte)
        && Number.isInteger(context?.source_span?.end_byte)
        && context.source_span.start_byte > context.source_span.end_byte) {
        diagnostics.push("source_span_start_exceeds_end");
      }
      if (sourceLengthReady(context)
        && Number.isInteger(context?.source_span?.end_byte)
        && context.source_span.end_byte > context.source_len_bytes) {
        diagnostics.push("source_span_exceeds_source_length");
      }
      return diagnostics;
    }

    function sourceDigestReady(context) {
      return typeof context?.source_digest === "string"
        && new RegExp(`^${sourceDigestPrefix}[0-9a-fA-F]{16}$`).test(context.source_digest);
    }

    function reverseCssDeltaContractDiagnostics() {
      const diagnostics = [];
      if (reverseCssDeltaContractSchema !== "dx.style.grouped-class-reverse-css-delta-contract") {
        diagnostics.push("reverse_css_delta_contract_missing");
      }
      if (reverseCssDeltaMutationEnabled) {
        diagnostics.push("reverse_css_delta_contract_mutation_enabled");
      }
      if (!reverseCssDeltaRequiredGuards.includes("reverse CSS delta preview provenance match")) {
        diagnostics.push("reverse_css_delta_contract_missing_provenance_guard");
      }
      if (!reverseCssDeltaRequiredProvenanceFields.length) {
        diagnostics.push("reverse_css_delta_contract_missing_required_provenance_fields");
      }
      if (!Number.isInteger(reverseCssDeltaPayloadLimits.replacementUtilities)) {
        diagnostics.push("reverse_css_delta_contract_missing_replacement_utility_count_limit");
      }
      if (!Number.isInteger(reverseCssDeltaPayloadLimits.replacementUtilityBytes)) {
        diagnostics.push("reverse_css_delta_contract_missing_replacement_utility_byte_limit");
      }
      if (!Number.isInteger(reverseCssDeltaPayloadLimits.replacementSourceDeclarationBytes)) {
        diagnostics.push("reverse_css_delta_contract_missing_replacement_source_declaration_limit");
      }
      for (const field of reverseCssDeltaRequiredProvenanceFields) {
        if (!reverseCssDeltaSupportedProvenanceFields.has(field)) {
          diagnostics.push(`reverse_css_delta_contract_unsupported_provenance_field:${field}`);
        }
      }
      return diagnostics;
    }

    function reverseCssDeltaContextField(field) {
      return ({
        group_status: "status",
        group_alias: "alias",
        group_syntax: "syntax",
        group_expansion_status: "expansion_status",
        group_registry_receipt: "registry_receipt",
        reverse_css_map_receipt: "reverse_css_map_receipt",
        reverse_css_map_status: "reverse_css_map_status",
        group_source_state: "source_state",
        group_utility_count: "utility_count"
      })[field] || null;
    }

    function hasOwn(source, field) {
      return !!source && Object.prototype.hasOwnProperty.call(source, field);
    }

    function reverseCssDeltaPreviewProvenanceDiagnostics(preview, group) {
      const diagnostics = [];
      for (const field of reverseCssDeltaRequiredProvenanceFields) {
        if (!reverseCssDeltaSupportedProvenanceFields.has(field)) continue;
        const contextField = reverseCssDeltaContextField(field);
        if (!contextField) continue;
        if (!hasOwn(preview, field)) {
          diagnostics.push(`reverse_css_delta_preview_missing:${field}`);
          continue;
        }
        if (!hasOwn(group, contextField)) {
          diagnostics.push(`reverse_css_delta_context_missing:${contextField}`);
          continue;
        }
        const previewValue = preview[field] ?? null;
        const contextValue = group[contextField] ?? null;
        if (previewValue !== contextValue) {
          diagnostics.push(`reverse_css_delta_preview_mismatch:${field}`);
        }
      }
      if (preview?.status === "ready_for_review" && !group?.reverse_css_map_status) {
        diagnostics.push("reverse_css_delta_preview_missing_reverse_map_provenance");
      }
      return diagnostics;
    }

    function reverseCssDeltaReplacementPayloadDiagnostics(preview) {
      const diagnostics = [];
      if (preview?.status !== "ready_for_review") return diagnostics;
      if (exceedsContractLimit(preview.target_utility || "", reverseCssDeltaPayloadLimits.replacementUtilityBytes)) {
        diagnostics.push("reverse_css_delta_target_utility_exceeds_contract_limit");
      }
      if (!Array.isArray(preview.replacement_utilities)) {
        diagnostics.push("reverse_css_delta_replacement_utilities_missing");
        return diagnostics;
      }
      if (Number.isInteger(reverseCssDeltaPayloadLimits.replacementUtilities)
        && preview.replacement_utilities.length > reverseCssDeltaPayloadLimits.replacementUtilities) {
        diagnostics.push("reverse_css_delta_replacement_utility_count_exceeds_contract_limit");
      }
      for (const utility of preview.replacement_utilities) {
        if (typeof utility !== "string") {
          diagnostics.push("reverse_css_delta_replacement_utility_non_string");
          continue;
        }
        if (!utility.length) {
          diagnostics.push("reverse_css_delta_replacement_utility_empty");
          continue;
        }
        if (exceedsContractLimit(utility, reverseCssDeltaPayloadLimits.replacementUtilityBytes)) {
          diagnostics.push("reverse_css_delta_replacement_utility_exceeds_contract_limit");
        }
      }
      const groupAlias = zedStyleContext?.group_context?.alias || preview.group_alias || "";
      if (groupAlias) {
        const expectedSourceDeclaration = `@${groupAlias}(${preview.replacement_utilities.join(" ")})`;
        if (exceedsContractLimit(expectedSourceDeclaration, reverseCssDeltaPayloadLimits.replacementSourceDeclarationBytes)) {
          diagnostics.push("reverse_css_delta_expected_source_declaration_exceeds_contract_limit");
        }
      }
      if (exceedsContractLimit(
        preview.replacement_source_declaration || "",
        reverseCssDeltaPayloadLimits.replacementSourceDeclarationBytes
      )) {
        diagnostics.push("reverse_css_delta_replacement_source_declaration_exceeds_contract_limit");
      }
      return [...new Set(diagnostics)];
    }

    function sourceApplyContractHasGuard(guard) {
      return sourceApplyRequiredEditorGuards.includes(guard);
    }

    function reverseCssDeltaReplacementPolicyDiagnostics(preview, group) {
      const diagnostics = [];
      if (preview?.status !== "ready_for_review") return diagnostics;
      if (group?.alias && Array.isArray(preview.replacement_utilities)) {
        const expectedSourceDeclaration = `@${group.alias}(${preview.replacement_utilities.join(" ")})`;
        if (!preview.replacement_source_declaration) {
          diagnostics.push("reverse_css_delta_replacement_source_declaration_missing");
        } else if (preview.replacement_source_declaration !== expectedSourceDeclaration) {
          diagnostics.push("reverse_css_delta_replacement_source_declaration_mismatch");
        }
      }
      if (preview.replacement_existing_utility_required !== true) return diagnostics;
      if (preview.replacement_existing_utility_found !== true) {
        diagnostics.push("reverse_css_delta_replacement_existing_utility_missing");
      }
      if (!Array.isArray(preview.replacement_utilities)) {
        diagnostics.push("reverse_css_delta_replacement_utilities_missing");
      }
      const expectedUtilityCount = Number.isInteger(group?.utility_count)
        ? group.utility_count
        : null;
      if (expectedUtilityCount === null) {
        diagnostics.push("reverse_css_delta_replacement_utility_count_missing");
      } else if (Array.isArray(preview.replacement_utilities)
        && preview.replacement_utilities.length !== expectedUtilityCount) {
        diagnostics.push("reverse_css_delta_replacement_utility_count_changed");
      }
      return diagnostics;
    }

__DX_STYLE_SOURCE_APPLY_SESSION_HANDLER__

    function contextSchemaSupported(context) {
      return context?.schema === expectedContextSchema;
    }

    function contextKindSupported(context, supportedKinds) {
      if (!supportedKinds.length) return true;
      return supportedKinds.includes(context?.context_kind || "");
    }

    function sourceApplyBlocker(applyGate, metadataAligned, context, output) {
      if (!metadataAligned) return "metadata_drift";
      if (!context) return "no_zed_style_context";
      if (!contextSchemaSupported(context)) return "unsupported_context_schema";
      if (!contextKindSupported(context, sourceApplyMutationContextKinds)) {
        return context?.context_kind === "css_declaration"
          ? "css_declaration_requires_css_dry_run_receipt"
          : "unsupported_mutation_context_kind";
      }
      if (!context.source_path) return "missing_source_path";
      if (!Number.isInteger(context.source_span?.start_byte)
        || !Number.isInteger(context.source_span?.end_byte)) {
        return "missing_source_span";
      }
      if (!sourceLengthReady(context)) return "missing_source_length";
      if (!sourceSpanReady(context)) return "invalid_source_span";
      if (!sourceDigestReady(context)) return "missing_or_invalid_source_digest";
      const payloadDiagnostics = sourceApplyPayloadDiagnostics(context, output);
      if (payloadDiagnostics.length) return payloadDiagnostics[0];
      const cssDeclarationPreviewDiagnostics = cssDeclarationDryRunContextPreviewDiagnostics(
        cssDeclarationDryRunPreview(output, context),
        context
      );
      if (cssDeclarationPreviewDiagnostics.length) return cssDeclarationPreviewDiagnostics[0];
      if (!applyGate?.can_enable_apply) return applyGate?.state || "apply_gate_blocked";
      if (!applyGate?.editor_write_bridge?.can_apply) return "editor_write_bridge_not_ready";
      if (sourceApplyContractSchema !== "dx.style.grouped-class-source-apply-contract") {
        return "source_apply_contract_missing";
      }
      if (sourceApplyIpcKind !== "dx-style-source-apply") {
        return "source_apply_ipc_kind_mismatch";
      }
      if (sourceApplySessionKind !== "zed.web_preview.dx_style.source_apply_session") {
        return "source_apply_session_kind_missing";
      }
      if (!sourceApplySessionToken) {
        return "source_apply_session_missing";
      }
      if (sourceApplyMaxSessionTokenBytes
        && utf8ByteLength(sourceApplySessionToken) > sourceApplyMaxSessionTokenBytes) {
        return "source_apply_session_token_exceeds_contract_limit";
      }
      if (reverseCssDeltaContractSchema !== "dx.style.grouped-class-reverse-css-delta-contract") {
        return "reverse_css_delta_contract_missing";
      }
      if (!sourceApplyContractHasGuard(reverseCssDeltaReplacementPolicyGuard)) {
        return "source_apply_contract_missing_replacement_policy_guard";
      }
      const reverseDeltaDiagnostics = reverseCssDeltaContractDiagnostics();
      if (reverseDeltaDiagnostics.length) return reverseDeltaDiagnostics[0];
      const reverseDeltaPreview = reverseCssDeltaPreview(output);
      const reverseDeltaPreviewDiagnostics = reverseCssDeltaPreviewProvenanceDiagnostics(
        reverseDeltaPreview,
        context?.group_context || null
      );
      if (sourceApplyMutationEnabled && reverseDeltaPreviewDiagnostics.length) {
        return reverseDeltaPreviewDiagnostics[0];
      }
      const reverseDeltaReplacementPolicyDiagnostics =
        reverseCssDeltaReplacementPolicyDiagnostics(
          reverseDeltaPreview,
          context?.group_context || null
        );
      if (sourceApplyMutationEnabled && reverseDeltaReplacementPolicyDiagnostics.length) {
        return reverseDeltaReplacementPolicyDiagnostics[0];
      }
      if (!sourceApplyMutationEnabled) return "source_apply_contract_review_only";
      const handlerState = sourceApplyHandlerState();
      if (handlerState === "review_only") return "native_apply_writer_missing";
      if (handlerState !== "ready") return "native_apply_handler_missing";
      return "ready";
    }

    function sourceApplyReady(applyGate, metadataAligned, context, output) {
      return sourceApplyBlocker(applyGate, metadataAligned, context, output) === "ready";
    }

    function sourceApplyReviewBlocker(metadataAligned, context, output) {
      if (!metadataAligned) return "metadata_drift";
      if (!context) return "no_zed_style_context";
      if (!contextSchemaSupported(context)) return "unsupported_context_schema";
      if (!contextKindSupported(context, sourceApplyReviewContextKinds)) {
        return "unsupported_review_context_kind";
      }
      if (!context.source_path) return "missing_source_path";
      if (!Number.isInteger(context.source_span?.start_byte)
        || !Number.isInteger(context.source_span?.end_byte)) {
        return "missing_source_span";
      }
      if (!sourceLengthReady(context)) return "missing_source_length";
      if (!sourceSpanReady(context)) return "invalid_source_span";
      if (!sourceDigestReady(context)) return "missing_or_invalid_source_digest";
      const payloadDiagnostics = sourceApplyPayloadDiagnostics(context, output);
      if (payloadDiagnostics.length) return payloadDiagnostics[0];
      const cssDeclarationDiagnostics = cssDeclarationDryRunContextDiagnostics(context);
      if (cssDeclarationDiagnostics.length) return cssDeclarationDiagnostics[0];
      const cssDeclarationPreviewDiagnostics = cssDeclarationDryRunContextPreviewDiagnostics(
        cssDeclarationDryRunPreview(output, context),
        context
      );
      if (cssDeclarationPreviewDiagnostics.length) return cssDeclarationPreviewDiagnostics[0];
      if (sourceApplyContractSchema !== "dx.style.grouped-class-source-apply-contract") {
        return "source_apply_contract_missing";
      }
      if (sourceApplyIpcKind !== "dx-style-source-apply") {
        return "source_apply_ipc_kind_mismatch";
      }
      if (sourceApplySessionKind !== "zed.web_preview.dx_style.source_apply_session") {
        return "source_apply_session_kind_missing";
      }
      if (!sourceApplySessionToken) {
        return "source_apply_session_missing";
      }
      if (sourceApplyMaxSessionTokenBytes
        && utf8ByteLength(sourceApplySessionToken) > sourceApplyMaxSessionTokenBytes) {
        return "source_apply_session_token_exceeds_contract_limit";
      }
      if (reverseCssDeltaContractSchema !== "dx.style.grouped-class-reverse-css-delta-contract") {
        return "reverse_css_delta_contract_missing";
      }
      if (!sourceApplyContractHasGuard(reverseCssDeltaReplacementPolicyGuard)) {
        return "source_apply_contract_missing_replacement_policy_guard";
      }
      const reverseDeltaDiagnostics = reverseCssDeltaContractDiagnostics();
      if (reverseDeltaDiagnostics.length) return reverseDeltaDiagnostics[0];
      if (!sourceApplyReviewHandler()) return "native_review_handler_missing";
      return "ready";
    }

    function sourceApplyReviewReady(metadataAligned, context, output) {
      return sourceApplyReviewBlocker(metadataAligned, context, output) === "ready";
    }

__DX_STYLE_CSS_DECLARATION_DRY_RUN_REVIEW__

    function tokenFromReverseDeltaValue(value, mapping) {
      const text = String(value || "").trim();
      const strategy = String(mapping?.value_strategy || "design_token_suffix");
      if (strategy === "display_keyword") return displayReverseDeltaToken(text);
      if (strategy === "arbitrary_bracket_value") return arbitraryReverseDeltaToken(text);
      if (strategy === "drop_shadow_function") {
        if (!text.startsWith("drop-shadow(") || !text.endsWith(")")) return null;
        return arbitraryReverseDeltaToken(text.slice("drop-shadow(".length, -1));
      }
      if (strategy === "backdrop_blur_function") {
        if (!text.startsWith("blur(") || !text.endsWith(")")) return null;
        return arbitraryReverseDeltaToken(text.slice("blur(".length, -1));
      }
      if (strategy === "align_items_keyword") return alignItemsReverseDeltaToken(text);
      if (strategy === "justify_content_keyword") return justifyContentReverseDeltaToken(text);
      if (strategy === "align_content_keyword") return alignContentReverseDeltaToken(text);
      if (strategy === "grid_track_repeat_count") return gridTrackRepeatCountToken(text);
      if (strategy === "transition_property_value") return transitionPropertyReverseDeltaToken(text);
      if (strategy === "transition_timing_function_value") return transitionTimingFunctionReverseDeltaToken(text);
      if (strategy === "arbitrary_css_property_value") return arbitraryCssPropertyReverseDeltaToken(mapping?.property, text);
      const prefix = String(mapping?.token_prefix || "");
      if (!prefix || !text.startsWith(prefix) || !text.endsWith(")")) return null;
      const token = text.slice(prefix.length, -1).trim();
      return token || null;
    }

    function targetUtilityFromReverseDelta(mapping, token) {
      const strategy = String(mapping?.value_strategy || "design_token_suffix");
      const prefix = String(mapping?.utility_prefix || "");
      const text = String(token || "");
      if (strategy === "display_keyword") return text;
      if (strategy === "margin_token_suffix" && text.startsWith("-")) {
        return `-${prefix}${text.slice(1)}`;
      }
      if (strategy === "arbitrary_css_property_value") return text;
      return `${prefix}${text}`;
    }

    function displayReverseDeltaToken(value) {
      switch (String(value || "").trim()) {
        case "block": return "block";
        case "inline-block": return "inline-block";
        case "inline": return "inline";
        case "flex": return "flex";
        case "inline-flex": return "inline-flex";
        case "grid": return "grid";
        case "inline-grid": return "inline-grid";
        case "contents": return "contents";
        case "flow-root": return "flow-root";
        case "none": return "hidden";
        default: return null;
      }
    }

    function alignItemsReverseDeltaToken(value) {
      switch (String(value || "").trim()) {
        case "normal": return "normal";
        case "stretch": return "stretch";
        case "center": return "center";
        case "flex-start":
        case "start": return "start";
        case "flex-end":
        case "end": return "end";
        case "baseline": return "baseline";
        default: return null;
      }
    }

    function justifyContentReverseDeltaToken(value) {
      switch (String(value || "").trim()) {
        case "normal": return "normal";
        case "center": return "center";
        case "flex-start":
        case "start": return "start";
        case "flex-end":
        case "end": return "end";
        case "space-between": return "between";
        case "space-around": return "around";
        case "space-evenly": return "evenly";
        case "stretch": return "stretch";
        default: return null;
      }
    }

    function alignContentReverseDeltaToken(value) {
      switch (String(value || "").trim()) {
        case "normal": return "normal";
        case "center": return "center";
        case "flex-start":
        case "start": return "start";
        case "flex-end":
        case "end": return "end";
        case "space-between": return "between";
        case "space-around": return "around";
        case "space-evenly": return "evenly";
        case "baseline": return "baseline";
        case "stretch": return "stretch";
        default: return null;
      }
    }

    function gridTrackRepeatCountToken(value) {
      const match = String(value || "").replace(/\s+/g, "").match(/^repeat\((\d{1,2}),minmax\(0,1fr\)\)$/);
      if (!match || match[1] === "0") return null;
      return match[1];
    }

    function transitionPropertyReverseDeltaToken(value) {
      switch (String(value || "").trim()) {
        case "none": return "none";
        case "all": return "all";
        case "color, background-color, border-color, text-decoration-color, fill, stroke": return "colors";
        case "opacity": return "opacity";
        case "box-shadow": return "shadow";
        case "transform": return "transform";
        default: return null;
      }
    }

    function transitionTimingFunctionReverseDeltaToken(value) {
      switch (String(value || "").trim()) {
        case "linear": return "linear";
        case "cubic-bezier(0.4, 0, 1, 1)": return "in";
        case "cubic-bezier(0, 0, 0.2, 1)": return "out";
        case "cubic-bezier(0.4, 0, 0.2, 1)": return "in-out";
        default: return null;
      }
    }

    function arbitraryReverseDeltaToken(value) {
      const text = String(value || "").trim();
      if (!text || text.length > 256 || /[\[\];\n\r]/.test(text)) return null;
      return `[${text.split(/\s+/).join("_")}]`;
    }

    function arbitraryCssPropertyReverseDeltaToken(property, value) {
      const token = arbitraryReverseDeltaToken(value);
      const propertyName = String(property || "").trim();
      if (!token || !/^[a-z-]+$/.test(propertyName)) return null;
      return `[${propertyName}:${token.slice(1, -1)}]`;
    }

    function utilityMatchesReverseDeltaFamily(utility, mapping) {
      const text = String(utility || "");
      const utilityPrefix = String(mapping?.utility_prefix || "");
      const property = String(mapping?.property || "").toLowerCase();
      const strategy = String(mapping?.value_strategy || "");
      if (!utilityPrefix) return isDisplayUtility(text);
      if ((property === "background" || property === "background-image") && utilityPrefix === "bg-") {
        return isBackgroundImageUtility(text);
      }
      if (property === "transform" && utilityPrefix === "transform-") {
        return isTransformUtility(text);
      }
      if (strategy === "arbitrary_css_property_value") {
        return isArbitraryCssPropertyUtility(text, property);
      }
      if (utilityPrefix === "gap-") return isBaseGapUtility(text);
      if (utilityPrefix === "border-") return isBorderColorUtility(text);
      if (utilityPrefix === "outline-") return isOutlineColorUtility(text);
      if (utilityPrefix === "shadow-") return isShadowEffectUtility(text);
      if (utilityPrefix === "text-shadow-") return isTextShadowEffectUtility(text);
      if (utilityPrefix === "drop-shadow-") return isDropShadowEffectUtility(text);
      if (utilityPrefix === "transition-") return isTransitionPropertyUtility(text);
      if (isMarginUtilityPrefix(utilityPrefix)) return isMarginUtility(text, utilityPrefix);
      return text.startsWith(utilityPrefix);
    }

    function isDisplayUtility(utility) {
      return [
        "block",
        "inline-block",
        "inline",
        "flex",
        "inline-flex",
        "grid",
        "inline-grid",
        "contents",
        "flow-root",
        "hidden"
      ].includes(utility);
    }

    function isMarginUtilityPrefix(utilityPrefix) {
      return ["m-", "mx-", "my-", "mt-", "mr-", "mb-", "ml-"].includes(utilityPrefix);
    }

    function isMarginUtility(utility, utilityPrefix) {
      const text = String(utility || "");
      return text.startsWith(utilityPrefix) || text.replace(/^-/, "").startsWith(utilityPrefix);
    }

    function isBaseGapUtility(utility) {
      const text = String(utility || "");
      return text.startsWith("gap-") && !text.startsWith("gap-x-") && !text.startsWith("gap-y-");
    }

    function isBorderColorUtility(utility) {
      const suffix = String(utility || "").replace(/^border-/, "");
      if (!utility.startsWith("border-") || !suffix) return false;
      if (/^\d+$/.test(suffix)) return false;
      return !["solid", "dashed", "dotted", "double", "hidden", "none"].includes(suffix);
    }

    function isOutlineColorUtility(utility) {
      const suffix = String(utility || "").replace(/^outline-/, "");
      if (!utility.startsWith("outline-") || !suffix) return false;
      if (/^\d+$/.test(suffix) || suffix.startsWith("offset-")) return false;
      return !["solid", "dashed", "dotted", "double", "hidden", "none"].includes(suffix);
    }

    function isBackgroundImageUtility(utility) {
      const text = String(utility || "");
      if (!text.startsWith("bg-")) return false;
      const suffix = text.replace(/^bg-/, "");
      return isArbitraryOrCssVariableToken(suffix)
        || suffix === "none"
        || suffix.startsWith("linear-")
        || suffix.startsWith("radial-")
        || suffix.startsWith("conic-")
        || suffix.startsWith("gradient-to-")
        || suffix.startsWith("image-")
        || suffix.startsWith("url-");
    }

    function isTransformUtility(utility) {
      const text = String(utility || "");
      if (["transform", "transform-gpu", "transform-cpu", "transform-3d", "transform-flat"].includes(text)) {
        return true;
      }
      const suffix = text.replace(/^transform-/, "");
      return text.startsWith("transform-") && isArbitraryOrCssVariableToken(suffix);
    }

    function isArbitraryCssPropertyUtility(utility, property) {
      const text = String(utility || "");
      const propertyName = String(property || "").trim();
      return text.startsWith(`[${propertyName}:`) && text.endsWith("]");
    }

    function isShadowEffectUtility(utility) {
      if (["shadow", "shadow-sm", "shadow-md", "shadow-lg", "shadow-xl", "shadow-2xl", "shadow-inner", "shadow-none"].includes(utility)) {
        return true;
      }
      const suffix = utility.replace(/^shadow-/, "");
      return utility.startsWith("shadow-") && isArbitraryOrCssVariableToken(suffix);
    }

    function isTextShadowEffectUtility(utility) {
      if (["text-shadow", "text-shadow-sm", "text-shadow-md", "text-shadow-lg", "text-shadow-none"].includes(utility)) {
        return true;
      }
      const suffix = utility.replace(/^text-shadow-/, "");
      return utility.startsWith("text-shadow-") && isArbitraryOrCssVariableToken(suffix);
    }

    function isDropShadowEffectUtility(utility) {
      if (["drop-shadow", "drop-shadow-sm", "drop-shadow-md", "drop-shadow-lg", "drop-shadow-xl", "drop-shadow-2xl", "drop-shadow-none"].includes(utility)) {
        return true;
      }
      const suffix = utility.replace(/^drop-shadow-/, "");
      return utility.startsWith("drop-shadow-") && isArbitraryOrCssVariableToken(suffix);
    }

    function isTransitionPropertyUtility(utility) {
      if ([
        "transition",
        "transition-none",
        "transition-all",
        "transition-colors",
        "transition-opacity",
        "transition-shadow",
        "transition-transform"
      ].includes(utility)) {
        return true;
      }
      const suffix = utility.replace(/^transition-/, "");
      return utility.startsWith("transition-") && isArbitraryOrCssVariableToken(suffix);
    }

    function isArbitraryOrCssVariableToken(suffix) {
      return (suffix.startsWith("[") && suffix.endsWith("]"))
        || (suffix.startsWith("(--") && suffix.endsWith(")"));
    }

    function replacementRequiresExistingUtility(mapping) {
      return reverseCssDeltaExistingUtilityRequiredPropertySet.has(
        String(mapping?.property || "").toLowerCase()
      );
    }

    function replacementUtilitiesForDelta(utilities, mapping, targetUtility) {
      let replaced = false;
      const next = utilities.map((utility) => {
        if (!replaced && utilityMatchesReverseDeltaFamily(utility, mapping)) {
          replaced = true;
          return targetUtility;
        }
        return utility;
      });
      if (!replaced) next.push(targetUtility);
      return { utilities: next, replaced };
    }

    function isFallbackReverseDeltaMapping(mapping) {
      const strategy = String(mapping?.value_strategy || "");
      const property = String(mapping?.property || "").toLowerCase();
      return reverseCssDeltaFallbackReviewPropertySet.has(property)
        || (strategy === "display_keyword" && !reverseCssDeltaFallbackReviewProperties.length);
    }

    function reverseCssDeltaPreviewProvenance(group) {
      return {
        group_status: group?.status || null,
        group_alias: group?.alias || null,
        group_syntax: group?.syntax || null,
        group_expansion_status: group?.expansion_status || null,
        group_registry_receipt: group?.registry_receipt || null,
        reverse_css_map_receipt: group?.reverse_css_map_receipt || null,
        reverse_css_map_status: group?.reverse_css_map_status || null,
        group_source_state: group?.source_state || null,
        group_utility_count: Number.isInteger(group?.utility_count) ? group.utility_count : null
      };
    }

    function reverseCssDeltaPreview(output) {
      const group = zedStyleContext?.group_context || null;
      const provenance = reverseCssDeltaPreviewProvenance(group);
      const utilities = Array.isArray(group?.utilities) ? group.utilities : [];
      if (!utilities.length) {
        return { ...provenance, status: "no_group_utilities", reason: "No active grouped utilities are available for reverse CSS delta review." };
      }
      const declarations = generatedCssDeclarations(output?.css);
      if (!declarations.length) {
        return { ...provenance, status: "no_generated_declarations", reason: "Current generator output has no simple CSS declarations to review." };
      }
      let firstUnsupportedValue = null;
      let fallbackStrategyPreview = null;
      for (const declaration of declarations) {
        const mappings = reverseCssDeltaSupportedProperties.filter((entry) =>
          String(entry.property || "").toLowerCase() === declaration.property.toLowerCase()
        );
        if (!mappings.length) continue;
        let declarationHadUnsupportedValue = false;
        for (const mapping of mappings) {
          const token = tokenFromReverseDeltaValue(declaration.value, mapping);
          if (!token) {
            declarationHadUnsupportedValue = true;
            continue;
          }
          const targetUtility = targetUtilityFromReverseDelta(mapping, token);
          const replacementExistingUtilityRequired = replacementRequiresExistingUtility(mapping);
          const replacement = replacementUtilitiesForDelta(
            utilities,
            mapping,
            targetUtility
          );
          if (replacementExistingUtilityRequired && !replacement.replaced) {
            firstUnsupportedValue ||= {
              ...provenance,
              status: "unsupported_value",
              property: declaration.property,
              value: declaration.value,
              target_utility: targetUtility,
              replacement_existing_utility_required: true,
              replacement_existing_utility_found: false,
              reason: "Generated declaration requires an existing same-family source utility before review."
            };
            continue;
          }
          const replacementUtilities = replacement.utilities;
          const preview = {
            ...provenance,
            status: utilities.includes(targetUtility) ? "no_change" : "ready_for_review",
            property: declaration.property,
            value: declaration.value,
            target_utility: targetUtility,
            replacement_utilities: replacementUtilities,
            replacement_existing_utility_required: replacementExistingUtilityRequired,
            replacement_existing_utility_found: replacement.replaced,
            replacement_source_declaration: group?.alias
              ? `@${group.alias}(${replacementUtilities.join(" ")})`
              : null,
            reason: "Generated declaration can be reviewed against source-owned grouped atomics."
          };
          if (isFallbackReverseDeltaMapping(mapping)) {
            fallbackStrategyPreview ||= preview;
            continue;
          }
          return preview;
        }
        if (declarationHadUnsupportedValue) {
          firstUnsupportedValue ||= {
            ...provenance,
            status: "unsupported_value",
            property: declaration.property,
            value: declaration.value,
            reason: "Generated declaration does not use a supported DX Style token form."
          };
          continue;
        }
      }
      if (fallbackStrategyPreview) return fallbackStrategyPreview;
      if (firstUnsupportedValue) return firstUnsupportedValue;
      return { ...provenance, status: "unsupported_declaration", reason: "Current generator output has no declaration covered by the reverse CSS delta contract." };
    }

    function sourceApplyRequest(output) {
      const cssDeclarationPreview = cssDeclarationDryRunPreview(output, zedStyleContext);
      const reverseDeltaPreview = reverseCssDeltaPreview(output);
      return {
        generator: state.generator,
        source_path: zedStyleContext?.source_path || null,
        source_span: zedStyleContext?.source_span || null,
        source_digest: zedStyleContext?.source_digest || null,
        source_len_bytes: zedStyleContext?.source_len_bytes || null,
        source_apply_session: {
          kind: sourceApplySessionKind,
          token: sourceApplySessionToken
        },
        output,
        context: zedStyleContext,
        metadata: metadataDiagnostics,
        contract: sourceApplyContract,
        css_declaration_dry_run_contract: cssDeclarationDryRunContract,
        css_declaration_dry_run_diagnostics: cssDeclarationDryRunContextDiagnostics(zedStyleContext),
        css_declaration_dry_run_preview: cssDeclarationPreview,
        css_declaration_dry_run_preview_diagnostics: cssDeclarationDryRunContextPreviewDiagnostics(cssDeclarationPreview),
        reverse_css_delta_contract: reverseCssDeltaContract,
        reverse_css_delta_preview: reverseDeltaPreview,
        reverse_css_delta_replacement_payload_diagnostics:
          reverseCssDeltaReplacementPayloadDiagnostics(reverseDeltaPreview)
      };
    }

    function handleReviewApplyClick() {
      const metadataAligned = metadataDiagnostics.status === "aligned";
      const output = generatorOutput();
      const handler = sourceApplyReviewHandler();
      const blocker = sourceApplyReviewBlocker(metadataAligned, zedStyleContext, output);
      if (blocker !== "ready" || !handler) {
        sourceStatusEl.textContent = `Source review refused: ${blocker}.`;
        return;
      }

      try {
        handler(sourceApplyRequest(output));
        sourceStatusEl.textContent = "Source apply review request sent to the native handler.";
      } catch (error) {
        sourceStatusEl.textContent = `Source review failed before native handoff: ${error?.message || error}`;
      }
    }

    function handleApplyClick() {
      const applyGate = zedStyleContext?.apply_gate || null;
      const metadataAligned = metadataDiagnostics.status === "aligned";
      const output = generatorOutput();
      const handler = sourceApplyHandler();
      if (!sourceApplyReady(applyGate, metadataAligned, zedStyleContext, output) || !handler) {
        sourceStatusEl.textContent = "Source apply refused: native Web Preview apply handler is unavailable.";
        return;
      }

      try {
        handler(sourceApplyRequest(output));
        sourceStatusEl.textContent = "Source apply request sent to the native handler.";
      } catch (error) {
        sourceStatusEl.textContent = `Source apply failed before native handoff: ${error?.message || error}`;
      }
    }

    function reviewPacket(output) {
      const applyGate = zedStyleContext?.apply_gate || null;
      const cssDeclarationDiagnostics = cssDeclarationDryRunContextDiagnostics(zedStyleContext);
      const cssDeclarationPreview = cssDeclarationDryRunPreview(output, zedStyleContext);
      const cssDeclarationPreviewDiagnostics = cssDeclarationDryRunContextPreviewDiagnostics(cssDeclarationPreview);
      const reverseDeltaPreview = reverseCssDeltaPreview(output);
      return {
        schema: "zed.web_preview.dx_style_generator_review_packet.v1",
        generator: state.generator,
        generator_metadata: catalogMetadataForGenerator(state.generator),
        output,
        context: {
          schema: zedStyleContext?.schema || null,
          status: zedStyleContext?.status || null,
          context_kind: zedStyleContext?.context_kind || null,
          source_path: zedStyleContext?.source_path || null,
          source_span: zedStyleContext?.source_span || null,
          source_digest: zedStyleContext?.source_digest || null,
          source_len_bytes: zedStyleContext?.source_len_bytes || null,
          token: zedStyleContext?.token || null,
          css_property: zedStyleContext?.css_property || null,
          css_source_edit_safety: zedStyleContext?.css_source_edit_safety || null,
          group_context: zedStyleContext?.group_context || null
        },
        source_apply: {
          review_ready: sourceApplyReviewReady(
            metadataDiagnostics.status === "aligned",
            zedStyleContext,
            output
          ),
          review_blocker: sourceApplyReviewBlocker(
            metadataDiagnostics.status === "aligned",
            zedStyleContext,
            output
          ),
          ready: sourceApplyReady(
            zedStyleContext?.apply_gate || null,
            metadataDiagnostics.status === "aligned",
            zedStyleContext,
            output
          ),
          blocker: sourceApplyBlocker(
            zedStyleContext?.apply_gate || null,
            metadataDiagnostics.status === "aligned",
            zedStyleContext,
            output
          ),
          contract_schema: sourceApplyContractSchema,
          receipt_schema: sourceApplyReceiptSchema,
          mutation_enabled: sourceApplyMutationEnabled,
          source_apply_session: sourceApplySessionReviewPacket(),
          editor_write_bridge: editorWriteBridgeReviewPacket(applyGate),
          source_write_readiness: sourceWriteReadinessPacket(applyGate, output),
          review_receipt_fields: sourceApplyReviewReceiptFields,
          css_declaration_dry_run_contract: {
            schema: cssDeclarationDryRunSchema,
            source: cssDeclarationDryRunSource,
            dry_run_receipt_schema: cssDeclarationDryRunContract.dry_run_receipt_schema || null,
            mutation_enabled: cssDeclarationDryRunMutationEnabled,
            max_declaration_bytes: cssDeclarationDryRunMaxDeclarationBytes,
            max_diagnostic_count: cssDeclarationDryRunMaxDiagnosticCount,
            max_diagnostic_bytes: cssDeclarationDryRunMaxDiagnosticBytes,
            max_source_path_bytes: cssDeclarationDryRunMaxSourcePathBytes,
            max_source_span_bytes: cssDeclarationDryRunMaxSourceSpanBytes,
            max_source_digest_bytes: cssDeclarationDryRunMaxSourceDigestBytes,
            review_receipt_fields: cssDeclarationDryRunReviewReceiptFields,
            accepted_source_edit_safety: cssDeclarationDryRunAcceptedSafety
          },
          css_declaration_dry_run_diagnostics: cssDeclarationDiagnostics,
          css_declaration_dry_run_preview: cssDeclarationPreview,
          css_declaration_dry_run_preview_diagnostics: cssDeclarationPreviewDiagnostics,
          dry_run_review: dryRunReviewPacket(applyGate)
        },
        reverse_css_delta_preview: reverseDeltaPreview,
        reverse_css_delta_replacement_payload_diagnostics:
          reverseCssDeltaReplacementPayloadDiagnostics(reverseDeltaPreview),
        metadata: metadataDiagnostics
      };
    }

    function dryRunReviewPacket(applyGate) {
      if (!applyGate) {
        return {
          trusted_receipt_present: false,
          receipt_match: "no_apply_gate",
          receipt_path: null,
          receipt_summary: null,
          structured_edit_preview_count: 0,
          structured_edit_previews: [],
          receipt_mismatch: null
        };
      }
      const structuredEditPreviews = dryRunStructuredEditPreviews(applyGate);
      return {
        trusted_receipt_present: applyGate.trusted_dry_run_receipt_present === true,
        receipt_match: applyGate.receipt_match || "unknown",
        receipt_path: applyGate.receipt_path || null,
        receipt_summary: applyGate.receipt_summary || null,
        structured_edit_preview_count: structuredEditPreviews.length,
        structured_edit_previews: structuredEditPreviews,
        receipt_mismatch: applyGate.receipt_mismatch || null
      };
    }

    function dryRunStructuredEditPreviews(applyGate) {
      const previews = Array.isArray(applyGate?.receipt_summary?.edit_previews)
        ? applyGate.receipt_summary.edit_previews
        : [];
      const limit = Number.isInteger(sourceApplyByteLimits.dryRunEditPreviews)
        ? sourceApplyByteLimits.dryRunEditPreviews
        : 3;
      return previews.slice(0, limit).map((edit) => ({
        source_path: edit?.source_path || null,
        start_byte: Number.isInteger(edit?.start_byte) ? edit.start_byte : null,
        end_byte: Number.isInteger(edit?.end_byte) ? edit.end_byte : null,
        replacement_text: typeof edit?.replacement_text === "string" ? edit.replacement_text : null,
        replacement: edit?.replacement || null
      }));
    }

    function sourceApplySessionReviewPacket() {
      const tokenByteLength = utf8ByteLength(sourceApplySessionToken || "");
      return {
        kind: sourceApplySessionKind,
        token_present: typeof sourceApplySessionToken === "string" && sourceApplySessionToken.length > 0,
        token_byte_length: tokenByteLength,
        within_contract_limit: !sourceApplyMaxSessionTokenBytes
          || tokenByteLength <= sourceApplyMaxSessionTokenBytes
      };
    }

    function editorWriteBridgeReviewPacket(applyGate) {
      const bridge = applyGate?.editor_write_bridge || null;
      if (!bridge) {
        return {
          present: false,
          state: "missing",
          can_apply: false,
          can_mutate_source: false,
          preflight_schema: null,
          preflight_schema_version: null,
          preflight_scope: null,
          preflight_fixture_path: null,
          summary: null,
          reason: "editor write bridge preflight is missing",
          runtime_validation_required: true,
          native_handler_state: sourceApplyHandlerState(),
          required_receipt_count: 0,
          required_guard_count: 0,
          required_editor_guard_count: 0,
          required_native_handler_count: 0,
          required_handler_capability_count: 0,
          required_native_handler_capability_count: 0,
          required_receipts: [],
          required_editor_guards: [],
          required_native_handlers: [],
          required_native_handler_capabilities: []
        };
      }
      const requiredReceipts = Array.isArray(bridge.required_receipts) ? bridge.required_receipts : [];
      const requiredGuards = Array.isArray(bridge.required_editor_guards) ? bridge.required_editor_guards : [];
      const requiredHandlers = Array.isArray(bridge.required_native_handlers) ? bridge.required_native_handlers : [];
      const requiredCapabilities = Array.isArray(bridge.required_native_handler_capabilities)
        ? bridge.required_native_handler_capabilities
        : [];
      return {
        present: true,
        state: bridge.state || "not_enabled",
        can_apply: bridge.can_apply === true,
        can_mutate_source: bridge.can_mutate_source === true,
        preflight_schema: bridge.preflight_schema || null,
        preflight_schema_version: Number.isInteger(bridge.preflight_schema_version)
          ? bridge.preflight_schema_version
          : null,
        preflight_scope: bridge.preflight_scope || null,
        preflight_fixture_path: bridge.preflight_fixture_path || null,
        summary: bridge.summary || null,
        reason: bridge.reason || null,
        runtime_validation_required: bridge.runtime_validation_required !== false,
        native_handler_state: sourceApplyHandlerState(),
        required_receipt_count: requiredReceipts.length,
        required_guard_count: requiredGuards.length,
        required_editor_guard_count: requiredGuards.length,
        required_native_handler_count: requiredHandlers.length,
        required_handler_capability_count: requiredCapabilities.length,
        required_native_handler_capability_count: requiredCapabilities.length,
        required_receipts: requiredReceipts,
        required_editor_guards: requiredGuards,
        required_native_handlers: requiredHandlers,
        required_native_handler_capabilities: requiredCapabilities
      };
    }

    function sourceApplyMutationCapabilityDeclared() {
      const handler = typeof window.__DX_STYLE_SOURCE_APPLY__ === "function"
        ? window.__DX_STYLE_SOURCE_APPLY__
        : null;
      return handler?.can_mutate_source === true;
    }

    function sourceWriteReadinessPacket(applyGate, output) {
      const context = zedStyleContext || null;
      const metadataAligned = metadataDiagnostics.status === "aligned";
      const bridge = editorWriteBridgeReviewPacket(applyGate);
      const session = sourceApplySessionReviewPacket();
      const dryRun = dryRunReviewPacket(applyGate);
      const handlerState = sourceApplyHandlerState();
      const webPreviewDeclaredMutationCapability = sourceApplyMutationCapabilityDeclared();
      const reviewBlocker = sourceApplyReviewBlocker(metadataAligned, context, output);
      const mutationBlocker = sourceApplyBlocker(applyGate, metadataAligned, context, output);
      const reverseDeltaPreview = reverseCssDeltaPreview(output);
      const reverseDeltaReplacementPolicyDiagnostics =
        reverseCssDeltaReplacementPolicyDiagnostics(
          reverseDeltaPreview,
          context?.group_context || null
        );
      const missingRequirements = [];
      if (!sourceApplyMutationEnabled) missingRequirements.push("source_mutation_contract_disabled");
      if (!sourceApplyContractHasGuard(reverseCssDeltaReplacementPolicyGuard)) {
        missingRequirements.push("source_apply_contract_missing_replacement_policy_guard");
      }
      if (!metadataAligned) missingRequirements.push("metadata_alignment_missing");
      if (!context) missingRequirements.push("active_style_context_missing");
      if (context && !contextSchemaSupported(context)) missingRequirements.push("active_context_schema_unsupported");
      if (context && !contextKindSupported(context, sourceApplyMutationContextKinds)) {
        missingRequirements.push("mutation_context_kind_unsupported");
      }
      if (!applyGate?.can_enable_apply) missingRequirements.push("apply_gate_not_ready");
      if (applyGate?.trusted_dry_run_receipt_present !== true) {
        missingRequirements.push("trusted_dry_run_receipt_missing");
      }
      if (applyGate?.receipt_match !== "active_source_matched") {
        missingRequirements.push("active_source_receipt_match_missing");
      }
      if (!applyGate?.receipt_path) missingRequirements.push("receipt_path_missing");
      if (!dryRun.structured_edit_preview_count) {
        missingRequirements.push("cursor_scoped_dry_run_edit_review_missing");
      }
      missingRequirements.push("native_active_editor_source_revalidation_missing");
      if (bridge.can_apply !== true) missingRequirements.push("editor_write_bridge_not_ready");
      if (bridge.can_mutate_source !== true) {
        missingRequirements.push("mutation_capable_editor_write_bridge_missing");
      }
      if (!webPreviewDeclaredMutationCapability) {
        missingRequirements.push("web_preview_mutation_capability_missing");
      }
      if (handlerState !== "ready") missingRequirements.push("native_writer_can_mutate_false");
      if (bridge.runtime_validation_required === true) {
        missingRequirements.push("runtime_webview_build_proof_missing");
      }
      if (sourceApplyMutationEnabled) {
        missingRequirements.push(...reverseDeltaReplacementPolicyDiagnostics);
      }
      const safeToMutate = missingRequirements.length === 0 && mutationBlocker === "ready";
      return {
        schema: "zed.web_preview.dx_style.source_write_readiness.v1",
        status: safeToMutate ? "ready" : "not_ready",
        safe_to_mutate: safeToMutate,
        mutation_ready: safeToMutate,
        source_mutation_enabled: sourceApplyMutationEnabled,
        review_ready: reviewBlocker === "ready",
        review_blocker: reviewBlocker,
        mutation_blocker: mutationBlocker,
        source_context_present: !!context,
        source_context_schema_supported: context ? contextSchemaSupported(context) : false,
        source_context_kind: context?.context_kind || null,
        source_span_present: Number.isInteger(context?.source_span?.start_byte)
          && Number.isInteger(context?.source_span?.end_byte),
        source_span_valid: context ? sourceSpanReady(context) : false,
        source_digest_valid: context ? sourceDigestReady(context) : false,
        source_length_present: Number.isInteger(context?.source_len_bytes),
        session_token_present: session.token_present,
        session_token_within_contract_limit: session.within_contract_limit,
        dry_run_receipt_present: dryRun.trusted_receipt_present,
        dry_run_receipt_match: dryRun.receipt_match,
        dry_run_structured_edit_preview_count: dryRun.structured_edit_preview_count,
        reverse_delta_replacement_policy_guard_present: sourceApplyContractHasGuard(
          reverseCssDeltaReplacementPolicyGuard
        ),
        reverse_delta_replacement_policy_diagnostics: reverseDeltaReplacementPolicyDiagnostics,
        native_revalidation_status: "not_performed_in_web_preview",
        editor_write_bridge_state: bridge.state,
        editor_write_bridge_summary: bridge.summary,
        editor_write_bridge_can_apply: bridge.can_apply,
        editor_write_bridge_can_mutate_source: bridge.can_mutate_source,
        runtime_validation_required: bridge.runtime_validation_required,
        web_preview_declared_mutation_capability: webPreviewDeclaredMutationCapability,
        native_handler_state: handlerState,
        missing_requirements: missingRequirements
      };
    }

    async function copyTextToClipboard(value) {
      const text = String(value || "");
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
        return;
      }
      const textarea = document.createElement("textarea");
      textarea.value = text;
      textarea.setAttribute("readonly", "true");
      textarea.style.position = "fixed";
      textarea.style.left = "-9999px";
      document.body.append(textarea);
      textarea.select();
      try {
        document.execCommand("copy");
      } finally {
        textarea.remove();
      }
    }

    async function handleCopy(kind) {
      const output = generatorOutput();
      const payload = kind === "class"
        ? output.className
        : kind === "css"
          ? output.css
          : JSON.stringify(reviewPacket(output), null, 2);
      try {
        await copyTextToClipboard(payload);
        sourceStatusEl.textContent = `Copied ${kind} to clipboard.`;
      } catch (error) {
        sourceStatusEl.textContent = `Copy failed: ${error?.message || error}`;
      }
    }

    function renderPatchReview(applyGate, output) {
      const bridgeReview = renderBridgeReview(applyGate, output);
      const groupReview = renderGroupContextReview();
      const generatorReview = renderGeneratorSafetyReview();
      const summary = applyGate?.receipt_summary || null;
      if (!summary) {
        const mismatch = applyGate?.receipt_mismatch || null;
        const mismatchItems = Array.isArray(mismatch?.reasons) && mismatch.reasons.length
          ? `<ul>${mismatch.reasons.map((reason) => `<li>${escapeHtml(reason)}</li>`).join("")}</ul>`
          : "";
        const closest = mismatch?.closest_candidate
          ? `<span>Closest: ${escapeHtml(mismatch.closest_candidate.receipt_path || "unknown receipt")} (${Number(mismatch.closest_candidate.match_score || 0)}/3) - ${escapeHtml(mismatch.closest_candidate.reason || "not applicable")}</span>`
          : "";
        const checked = Number(mismatch?.checked_receipt_count || 0);
        const checkedLine = checked ? `<span>Checked ${checked} trusted dry-run receipt(s).</span>` : "";
        patchReviewEl.innerHTML = `<strong>Patch review</strong><span>${escapeHtml(applyGate?.reason || "No trusted dry-run receipt is available.")}</span>${checkedLine}${closest}${mismatchItems}${generatorReview}${groupReview}${bridgeReview}`;
        return;
      }
      const editItems = Array.isArray(summary.edits) && summary.edits.length
        ? `<ul>${summary.edits.map((edit) => `<li>${escapeHtml(edit)}</li>`).join("")}</ul>`
        : `<span>No edit preview lines were included in the trusted receipt.</span>`;
      const structuredEditItems = Array.isArray(summary.edit_previews) && summary.edit_previews.length
        ? `<ul>${summary.edit_previews.map((edit) =>
            `<li>${escapeHtml(edit.source_path || "unknown source")}:${Number(edit.start_byte || 0)}..${Number(edit.end_byte || 0)} -> ${escapeHtml(edit.replacement_text || edit.replacement || "missing replacement")}</li>`
          ).join("")}</ul>`
        : "";
      patchReviewEl.innerHTML = `
        <strong>Patch review</strong>
        <dl>
          <dt>Intent</dt><dd>${escapeHtml(summary.intent || "unknown")}</dd>
          <dt>Status</dt><dd>${escapeHtml(summary.status || "ready")}</dd>
          <dt>Edits</dt><dd>${Number(summary.edit_count || 0)}</dd>
          <dt>Message</dt><dd>${escapeHtml(summary.message || "Ready for review")}</dd>
        </dl>
        ${editItems}
        ${structuredEditItems ? `<span>Structured edit previews</span>${structuredEditItems}` : ""}
        ${generatorReview}
        ${groupReview}
        ${bridgeReview}
      `;
    }

    function renderGeneratorSafetyReview() {
      const metadata = catalogMetadataForGenerator(state.generator);
      return `
        <strong>Generator source safety</strong>
        <dl>
          <dt>Category</dt><dd>${escapeHtml(metadata.category)}</dd>
          <dt>Output</dt><dd>${escapeHtml(metadata.preferredOutput)}</dd>
          <dt>Edit safety</dt><dd>${escapeHtml(metadata.sourceEditSafety)}</dd>
        </dl>
      `;
    }

    function renderBridgeReview(applyGate, output) {
      const sourceApplyContractReview = renderSourceApplyContractReview(output);
      const bridge = applyGate?.editor_write_bridge || null;
      if (!bridge) return sourceApplyContractReview;
      const receipts = Array.isArray(bridge.required_receipts)
        ? bridge.required_receipts.map((receipt) => `<li>${escapeHtml(receipt)}</li>`).join("")
        : "";
      const guards = Array.isArray(bridge.required_editor_guards)
        ? bridge.required_editor_guards.map((guard) => `<li>${escapeHtml(guard)}</li>`).join("")
        : "";
      const handlers = Array.isArray(bridge.required_native_handlers)
        ? bridge.required_native_handlers.map((handler) => `<li>${escapeHtml(handler)}</li>`).join("")
        : "";
      const handlerCapabilities = Array.isArray(bridge.required_native_handler_capabilities)
        ? bridge.required_native_handler_capabilities.map((capability) => `<li>${escapeHtml(capability)}</li>`).join("")
        : "";
      return `
        ${sourceApplyContractReview}
        <strong>Write bridge preflight</strong>
        <dl>
          <dt>State</dt><dd>${escapeHtml(bridge.state || "not_enabled")}</dd>
          <dt>Summary</dt><dd>${escapeHtml(bridge.summary || "preflight not ready")}</dd>
          <dt>Schema</dt><dd>${escapeHtml(bridge.preflight_schema || "unknown")}</dd>
          <dt>Runtime</dt><dd>${bridge.runtime_validation_required ? "validation required" : "not required"}</dd>
        </dl>
        ${receipts ? `<span>Required receipts</span><ul>${receipts}</ul>` : ""}
        ${guards ? `<span>Required guards</span><ul>${guards}</ul>` : ""}
        ${handlers ? `<span>Required native handlers</span><ul>${handlers}</ul>` : ""}
        ${handlerCapabilities ? `<span>Required handler capabilities</span><ul>${handlerCapabilities}</ul>` : ""}
      `;
    }

    function renderGroupContextReview() {
      const group = zedStyleContext?.group_context || null;
      const utilities = Array.isArray(group?.utilities) && group.utilities.length
        ? `<ul>${group.utilities.map((utility) => `<li>${escapeHtml(utility)}</li>`).join("")}</ul>`
        : "";
      return `
        <strong>Grouped class context</strong>
        <dl>
          <dt>Schema</dt><dd>${escapeHtml(groupContextContractSchema)}</dd>
          <dt>Source</dt><dd>${escapeHtml(groupContextContractSource)}</dd>
          <dt>Status</dt><dd>${escapeHtml(group?.status || "none")}</dd>
          <dt>Alias</dt><dd>${escapeHtml(group?.alias || "none")}</dd>
          <dt>Syntax</dt><dd>${escapeHtml(group?.syntax || "not_grouped")}</dd>
          <dt>Expansion</dt><dd>${escapeHtml(group?.expansion_status || "not_available")}</dd>
          <dt>Registry</dt><dd>${escapeHtml(group?.registry_receipt || "not available")}</dd>
          <dt>Reverse CSS map</dt><dd>${escapeHtml(group?.reverse_css_map_receipt || "not available")}</dd>
          <dt>Reverse CSS status</dt><dd>${escapeHtml(group?.reverse_css_map_status || "not available")}</dd>
          <dt>Utilities</dt><dd>${Number(group?.utility_count || 0)} / ${groupContextMaxUtilityCount || "unknown"}</dd>
        </dl>
        ${group?.source_state ? `<span>${escapeHtml(group.source_state)}</span>` : ""}
        ${utilities}
      `;
    }

    function renderSourceApplyContractReview(output) {
      const reverseCssDeltaReview = renderReverseCssDeltaContractReview(output);
      const cssDryRunReview = renderCssDeclarationDryRunContractReview();
      const sourceWriteReadiness = sourceWriteReadinessPacket(zedStyleContext?.apply_gate || null, output);
      const handlerCapabilities = sourceApplyRequiredHandlerCapabilities.length
        ? sourceApplyRequiredHandlerCapabilities.map((capability) => `<li>${escapeHtml(capability)}</li>`).join("")
        : "";
      const receiptFields = sourceApplyReviewReceiptFields.length
        ? sourceApplyReviewReceiptFields.map((field) => `<li>${escapeHtml(field)}</li>`).join("")
        : "";
      const reviewKinds = sourceApplyReviewContextKinds.length
        ? sourceApplyReviewContextKinds.map((kind) => `<li>${escapeHtml(kind)}</li>`).join("")
        : "";
      const mutationKinds = sourceApplyMutationContextKinds.length
        ? sourceApplyMutationContextKinds.map((kind) => `<li>${escapeHtml(kind)}</li>`).join("")
        : "";
      const payloadDiagnostics = sourceApplyPayloadDiagnostics(zedStyleContext, output);
      const diagnostics = payloadDiagnostics.length
        ? `<span>Payload diagnostics</span><ul>${payloadDiagnostics.map((diagnostic) => `<li>${escapeHtml(diagnostic)}</li>`).join("")}</ul>`
        : `<span>Payload diagnostics: within source-owned contract limits</span>`;
      const readinessGaps = sourceWriteReadiness.missing_requirements.length
        ? sourceWriteReadiness.missing_requirements.map((requirement) => `<li>${escapeHtml(requirement)}</li>`).join("")
        : "";
      return `
        ${reverseCssDeltaReview}
        ${cssDryRunReview}
        <strong>Source apply contract</strong>
        <dl>
          <dt>Schema</dt><dd>${escapeHtml(sourceApplyContractSchema)}</dd>
          <dt>IPC</dt><dd>${escapeHtml(sourceApplyIpcKind)}</dd>
          <dt>Receipt</dt><dd>${escapeHtml(sourceApplyReceiptSchema)}</dd>
          <dt>Mutation</dt><dd>${sourceApplyMutationEnabled ? "enabled" : "review only"}</dd>
          <dt>Source path max</dt><dd>${sourceApplyByteLimits.sourcePath || "unknown"} bytes</dd>
          <dt>Class max</dt><dd>${sourceApplyByteLimits.className || "unknown"} bytes</dd>
          <dt>CSS max</dt><dd>${sourceApplyByteLimits.css || "unknown"} bytes</dd>
          <dt>Span max</dt><dd>${sourceApplyByteLimits.sourceSpan || "unknown"} bytes</dd>
          <dt>Digest max</dt><dd>${sourceApplyByteLimits.sourceDigest || "unknown"} bytes</dd>
          <dt>Dry-run edits max</dt><dd>${sourceApplyByteLimits.dryRunEditPreviews || "unknown"} preview(s)</dd>
          <dt>Replacement max</dt><dd>${sourceApplyByteLimits.dryRunReplacementText || "unknown"} bytes</dd>
          <dt>Preview kind max</dt><dd>${sourceApplyByteLimits.previewKind || "unknown"} bytes</dd>
          <dt>Preview anatomy max</dt><dd>${sourceApplyByteLimits.previewAnatomyParts || "unknown"} part(s)</dd>
          <dt>Write readiness</dt><dd>${escapeHtml(sourceWriteReadiness.status)}</dd>
          <dt>Write blocker</dt><dd>${escapeHtml(sourceWriteReadiness.mutation_blocker)}</dd>
          <dt>Write safe</dt><dd>${sourceWriteReadiness.safe_to_mutate ? "yes" : "no"}</dd>
        </dl>
        ${readinessGaps ? `<span>Write gaps</span><ul>${readinessGaps}</ul>` : ""}
        ${diagnostics}
        ${reviewKinds ? `<span>Review context kinds</span><ul>${reviewKinds}</ul>` : ""}
        ${mutationKinds ? `<span>Mutation context kinds</span><ul>${mutationKinds}</ul>` : ""}
        ${handlerCapabilities ? `<span>Required source-apply capabilities</span><ul>${handlerCapabilities}</ul>` : ""}
        ${receiptFields ? `<span>Review receipt fields</span><ul>${receiptFields}</ul>` : ""}
      `;
    }

    function renderCssDeclarationDryRunContractReview() {
      const preview = cssDeclarationDryRunPreview(generatorOutput());
      const requiredFields = cssDeclarationDryRunRequiredFields.length
        ? `<ul>${cssDeclarationDryRunRequiredFields.map((field) => `<li>${escapeHtml(field)}</li>`).join("")}</ul>`
        : "";
      const acceptedSafety = cssDeclarationDryRunAcceptedSafety.length
        ? `<ul>${cssDeclarationDryRunAcceptedSafety.map((safety) => `<li>${escapeHtml(safety)}</li>`).join("")}</ul>`
        : "";
      const receiptFields = cssDeclarationDryRunReviewReceiptFields.length
        ? `<ul>${cssDeclarationDryRunReviewReceiptFields.map((field) => `<li>${escapeHtml(field)}</li>`).join("")}</ul>`
        : "";
      const diagnostics = cssDeclarationDryRunContextDiagnostics(zedStyleContext);
      const previewDiagnostics = cssDeclarationDryRunContextPreviewDiagnostics(preview);
      const diagnosticItems = diagnostics.length
        ? `<ul>${diagnostics.map((diagnostic) => `<li>${escapeHtml(diagnostic)}</li>`).join("")}</ul>`
        : "<span>CSS declaration dry-run contract: ready for review contexts.</span>";
      const previewDiagnosticItems = previewDiagnostics.length
        ? `<ul>${previewDiagnostics.map((diagnostic) => `<li>${escapeHtml(diagnostic)}</li>`).join("")}</ul>`
        : "";
      return `
        <strong>CSS declaration dry-run contract</strong>
        <dl>
          <dt>Schema</dt><dd>${escapeHtml(cssDeclarationDryRunSchema)}</dd>
          <dt>Source</dt><dd>${escapeHtml(cssDeclarationDryRunSource)}</dd>
          <dt>Context</dt><dd>${escapeHtml(cssDeclarationDryRunContextKind)}</dd>
          <dt>Mutation</dt><dd>${cssDeclarationDryRunMutationEnabled ? "enabled" : "review only"}</dd>
          <dt>Declaration max</dt><dd>${cssDeclarationDryRunMaxDeclarationBytes || "unknown"} bytes</dd>
          <dt>Diagnostic max</dt><dd>${cssDeclarationDryRunMaxDiagnosticCount || "unknown"} item(s), ${cssDeclarationDryRunMaxDiagnosticBytes || "unknown"} bytes each</dd>
          <dt>Source limits</dt><dd>${cssDeclarationDryRunMaxSourcePathBytes || "unknown"} path bytes, ${cssDeclarationDryRunMaxSourceSpanBytes || "unknown"} span bytes, ${cssDeclarationDryRunMaxSourceDigestBytes || "unknown"} digest bytes</dd>
          <dt>Preview</dt><dd>${escapeHtml(preview.status || "not_available")}</dd>
        </dl>
        ${preview.proposed_declaration ? `<span>Proposed declaration</span><code>${escapeHtml(preview.proposed_declaration)}</code>` : ""}
        ${requiredFields ? `<span>Required CSS context fields</span>${requiredFields}` : ""}
        ${acceptedSafety ? `<span>Accepted source-edit safety</span>${acceptedSafety}` : ""}
        ${receiptFields ? `<span>CSS review receipt fields</span>${receiptFields}` : ""}
        ${diagnosticItems}
        ${previewDiagnosticItems ? `<span>Preview diagnostics</span>${previewDiagnosticItems}` : ""}
      `;
    }

    function renderReverseCssDeltaContractReview(output) {
      const deltaPreview = reverseCssDeltaPreview(output);
      const properties = reverseCssDeltaSupportedProperties.length
        ? `<ul>${reverseCssDeltaSupportedProperties.map((entry) => `<li>${escapeHtml(entry.property || "unknown")} -> ${escapeHtml(entry.utility_prefix || "unknown")} (${escapeHtml(entry.value_strategy || "design_token_suffix")})</li>`).join("")}</ul>`
        : "";
      const guards = reverseCssDeltaRequiredGuards.length
        ? `<ul>${reverseCssDeltaRequiredGuards.map((guard) => `<li>${escapeHtml(guard)}</li>`).join("")}</ul>`
        : "";
      const provenanceFields = reverseCssDeltaRequiredProvenanceFields.length
        ? `<ul>${reverseCssDeltaRequiredProvenanceFields.map((field) => `<li>${escapeHtml(field)}</li>`).join("")}</ul>`
        : "";
      const diagnostics = reverseCssDeltaContractDiagnostics();
      const contractDiagnostics = diagnostics.length
        ? `<ul>${diagnostics.map((diagnostic) => `<li>${escapeHtml(diagnostic)}</li>`).join("")}</ul>`
        : "";
      const previewProvenanceDiagnostics = reverseCssDeltaPreviewProvenanceDiagnostics(
        deltaPreview,
        zedStyleContext?.group_context || null
      );
      const replacementPayloadDiagnostics = reverseCssDeltaReplacementPayloadDiagnostics(deltaPreview);
      const previewDiagnostics = previewProvenanceDiagnostics.length
        ? `<ul>${previewProvenanceDiagnostics.map((diagnostic) => `<li>${escapeHtml(diagnostic)}</li>`).join("")}</ul>`
        : "";
      const replacementDiagnostics = replacementPayloadDiagnostics.length
        ? `<ul>${replacementPayloadDiagnostics.map((diagnostic) => `<li>${escapeHtml(diagnostic)}</li>`).join("")}</ul>`
        : "";
      const preview = reverseCssDeltaExample
        ? `<span>Example: ${escapeHtml(reverseCssDeltaExample.property || "unknown")} -> ${escapeHtml(reverseCssDeltaExample.target_utility || "not mapped")}</span>`
        : "";
      const livePreview = deltaPreview?.target_utility
        ? `<span>Live review: ${escapeHtml(deltaPreview.property)} -> ${escapeHtml(deltaPreview.target_utility)}</span>`
        : `<span>Live review: ${escapeHtml(deltaPreview?.status || "not_available")}</span>`;
      const replacement = Array.isArray(deltaPreview?.replacement_utilities) && deltaPreview.replacement_utilities.length
        ? `<ul>${deltaPreview.replacement_utilities.map((utility) => `<li>${escapeHtml(utility)}</li>`).join("")}</ul>`
        : "";
      return `
        <strong>Reverse CSS delta contract</strong>
        <dl>
          <dt>Schema</dt><dd>${escapeHtml(reverseCssDeltaContractSchema)}</dd>
          <dt>Source</dt><dd>${escapeHtml(reverseCssDeltaContractSource)}</dd>
          <dt>Mutation</dt><dd>${reverseCssDeltaMutationEnabled ? "enabled" : "review only"}</dd>
          <dt>Properties</dt><dd>${reverseCssDeltaSupportedProperties.length}</dd>
          <dt>Live status</dt><dd>${escapeHtml(deltaPreview?.status || "not_available")}</dd>
          <dt>Group alias</dt><dd>${escapeHtml(deltaPreview?.group_alias || "none")}</dd>
          <dt>Registry receipt</dt><dd>${escapeHtml(deltaPreview?.group_registry_receipt || "not available")}</dd>
          <dt>Reverse map</dt><dd>${escapeHtml(deltaPreview?.reverse_css_map_status || "not available")}</dd>
          <dt>Replacement policy</dt><dd>${deltaPreview?.replacement_existing_utility_required ? "existing utility required" : "append allowed"}${deltaPreview?.replacement_existing_utility_found ? ", existing utility found" : ""}</dd>
          <dt>Replacement limit</dt><dd>${reverseCssDeltaPayloadLimits.replacementUtilities || "unknown"} utilities, ${reverseCssDeltaPayloadLimits.replacementUtilityBytes || "unknown"} bytes each</dd>
        </dl>
        ${livePreview}
        ${deltaPreview?.replacement_source_declaration ? `<span>${escapeHtml(deltaPreview.replacement_source_declaration)}</span>` : ""}
        ${replacement ? `<span>Proposed grouped atomics</span>${replacement}` : ""}
        ${preview}
        ${properties ? `<span>Supported declaration deltas</span>${properties}` : ""}
        ${guards ? `<span>Required delta guards</span>${guards}` : ""}
        ${provenanceFields ? `<span>Required preview provenance</span>${provenanceFields}` : ""}
        ${contractDiagnostics ? `<span>Contract diagnostics</span>${contractDiagnostics}` : ""}
        ${previewDiagnostics ? `<span>Preview provenance diagnostics</span>${previewDiagnostics}` : ""}
        ${replacementDiagnostics ? `<span>Replacement payload diagnostics</span>${replacementDiagnostics}` : ""}
      `;
    }

    function escapeHtml(value) {
      return String(value).replace(/[&<>"']/g, (char) => ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;"
      }[char]));
    }

    function boundedPreviewItemCount() {
      return Math.max(2, Math.min(6, Number(state.columns || 3)));
    }

    function previewMarkupForOutput(output) {
      const parts = Array.isArray(output?.previewAnatomy) && output.previewAnatomy.length
        ? output.previewAnatomy
        : fallbackPreviewAnatomy(output?.previewKind || "hero-card");
      const markup = parts
        .map(previewPartMarkup)
        .filter(Boolean)
        .join("");
      return markup || previewPartMarkup("preview-label");
    }

    function previewPartMarkup(part) {
      switch (part) {
        case "layout-items":
          return Array.from({ length: boundedPreviewItemCount() }, (_, index) =>
            `<span class="preview-item">${index + 1}</span>`
          ).join("");
        case "timeline-track":
          return `<span class="timeline-track"><span></span><span></span><span></span></span>`;
        case "timeline-label":
          return `<span class="timeline-label">${escapeHtml(`${state.duration}ms ${state.easing}`)}</span>`;
        case "swatch-row":
          return `
            <span class="swatch-row">
              <span style="background:${escapeHtml(state.from)}"></span>
              <span style="background:${escapeHtml(state.to)}"></span>
            </span>
          `;
        case "color-transition-label":
          return `<span class="timeline-label">${escapeHtml(`${state.from} -> ${state.to}`)}</span>`;
        case "preview-title":
          return `<span class="preview-title">DX Style</span>`;
        case "preview-subtitle":
          return `<span class="preview-subtitle">Source-owned visual preview</span>`;
        case "preview-label":
          return "DX Style Preview";
        default:
          return "";
      }
    }

    function updatePreview() {
      const output = generatorOutput();
      const applyGate = zedStyleContext?.apply_gate || null;
      const metadataAligned = metadataDiagnostics.status === "aligned";
      const payloadDiagnostics = sourceApplyPayloadDiagnostics(zedStyleContext, output);
      const cssDeclarationDiagnostics = cssDeclarationDryRunContextDiagnostics(zedStyleContext);
      const cssDeclarationPreview = cssDeclarationDryRunPreview(output);
      const cssDeclarationPreviewDiagnostics = cssDeclarationDryRunContextPreviewDiagnostics(cssDeclarationPreview);
      const reverseDeltaContractDiagnostics = reverseCssDeltaContractDiagnostics();
      const reverseDeltaPreview = reverseCssDeltaPreview(output);
      const reverseDeltaPreviewProvenanceDiagnostics = reverseCssDeltaPreviewProvenanceDiagnostics(
        reverseDeltaPreview,
        zedStyleContext?.group_context || null
      );
      const reverseDeltaReplacementPayloadDiagnostics =
        reverseCssDeltaReplacementPayloadDiagnostics(reverseDeltaPreview);
      const applyReady = sourceApplyReady(applyGate, metadataAligned, zedStyleContext, output);
      const applyBlocker = sourceApplyBlocker(applyGate, metadataAligned, zedStyleContext, output);
      const groupContext = zedStyleContext?.group_context || null;
      const generatorMetadata = catalogMetadataForGenerator(state.generator);
      const contextLines = zedStyleContext ? [
        `context_schema: ${zedStyleContext.schema || "unknown"}`,
        `context_schema_supported: ${contextSchemaSupported(zedStyleContext)}`,
        `context_status: ${zedStyleContext.status || "unknown"}`,
        zedStyleContext.context_kind ? `context_kind: ${zedStyleContext.context_kind}` : null,
        zedStyleContext.source_path ? `source_path: ${zedStyleContext.source_path}` : null,
        zedStyleContext.token ? `active_token: ${zedStyleContext.token}` : null,
        zedStyleContext.css_property ? `css_property: ${zedStyleContext.css_property}` : null,
        zedStyleContext.css_generator ? `css_generator: ${zedStyleContext.css_generator}` : null,
        zedStyleContext.css_source_edit_safety ? `css_source_edit_safety: ${zedStyleContext.css_source_edit_safety}` : null,
        Array.isArray(zedStyleContext.attribute_tokens) ? `attribute_tokens: ${zedStyleContext.attribute_tokens.length}` : null,
        groupContext?.status ? `group_context: ${groupContext.status}` : null,
        groupContext?.alias ? `group_alias: ${groupContext.alias}` : null,
        groupContext?.syntax ? `group_syntax: ${groupContext.syntax}` : null,
        groupContext?.expansion_status ? `group_expansion_status: ${groupContext.expansion_status}` : null,
        groupContext?.registry_receipt ? `group_registry_receipt: ${groupContext.registry_receipt}` : null,
        groupContext?.reverse_css_map_receipt ? `reverse_css_map_receipt: ${groupContext.reverse_css_map_receipt}` : null,
        groupContext?.reverse_css_map_status ? `reverse_css_map_status: ${groupContext.reverse_css_map_status}` : null,
        Number.isInteger(groupContext?.utility_count) ? `group_utility_count: ${groupContext.utility_count}` : null,
        Number.isInteger(groupContext?.candidate_token_count) ? `group_candidate_token_count: ${groupContext.candidate_token_count}` : null,
        groupContext?.source_state ? `group_source_state: ${groupContext.source_state}` : null,
        contextGeneratorSource ? `suggested_generator_source: ${contextGeneratorSource}` : null,
        zedStyleContext.span ? `source_span: ${zedStyleContext.span}` : null,
        Number.isInteger(zedStyleContext.source_span?.start_byte) && Number.isInteger(zedStyleContext.source_span?.end_byte) ? `source_span_bytes: ${zedStyleContext.source_span.start_byte}..${zedStyleContext.source_span.end_byte}` : null,
        zedStyleContext.source_digest ? `source_digest: ${zedStyleContext.source_digest}` : null,
        Number.isInteger(zedStyleContext.source_len_bytes) ? `source_len_bytes: ${zedStyleContext.source_len_bytes}` : null,
        zedStyleContext.source_state ? `source_state: ${zedStyleContext.source_state}` : null,
        contextGenerator ? `suggested_generator: ${contextGenerator}` : null,
        zedStyleContext.apply_gate?.state ? `apply_gate: ${zedStyleContext.apply_gate.state}` : null,
        zedStyleContext.apply_gate?.receipt_match ? `receipt_match: ${zedStyleContext.apply_gate.receipt_match}` : null,
        zedStyleContext.apply_gate?.receipt_mismatch?.checked_receipt_count ? `receipt_mismatch_checked: ${zedStyleContext.apply_gate.receipt_mismatch.checked_receipt_count}` : null,
        Array.isArray(zedStyleContext.apply_gate?.receipt_mismatch?.reasons) ? `receipt_mismatch_reasons: ${zedStyleContext.apply_gate.receipt_mismatch.reasons.length}` : null,
        zedStyleContext.apply_gate?.receipt_mismatch?.closest_candidate?.receipt_path ? `receipt_closest_candidate: ${zedStyleContext.apply_gate.receipt_mismatch.closest_candidate.receipt_path}` : null,
        zedStyleContext.apply_gate?.editor_write_bridge?.state ? `editor_write_bridge: ${zedStyleContext.apply_gate.editor_write_bridge.state}` : null,
        zedStyleContext.apply_gate?.editor_write_bridge?.summary ? `editor_write_bridge_summary: ${zedStyleContext.apply_gate.editor_write_bridge.summary}` : null,
        zedStyleContext.apply_gate?.editor_write_bridge?.preflight_schema ? `editor_write_bridge_schema: ${zedStyleContext.apply_gate.editor_write_bridge.preflight_schema}` : null,
        zedStyleContext.apply_gate?.editor_write_bridge?.preflight_fixture_path ? `editor_write_bridge_fixture: ${zedStyleContext.apply_gate.editor_write_bridge.preflight_fixture_path}` : null,
        Array.isArray(zedStyleContext.apply_gate?.editor_write_bridge?.required_editor_guards) ? `editor_write_bridge_guards: ${zedStyleContext.apply_gate.editor_write_bridge.required_editor_guards.length}` : null,
        Array.isArray(zedStyleContext.apply_gate?.editor_write_bridge?.required_native_handlers) ? `editor_write_bridge_native_handlers: ${zedStyleContext.apply_gate.editor_write_bridge.required_native_handlers.length}` : null,
        Array.isArray(zedStyleContext.apply_gate?.editor_write_bridge?.required_native_handler_capabilities) ? `editor_write_bridge_handler_capabilities: ${zedStyleContext.apply_gate.editor_write_bridge.required_native_handler_capabilities.length}` : null,
        zedStyleContext.apply_gate?.editor_write_bridge?.reason ? `editor_write_bridge_reason: ${zedStyleContext.apply_gate.editor_write_bridge.reason}` : null,
        zedStyleContext.apply_gate?.reason ? `apply_gate_reason: ${zedStyleContext.apply_gate.reason}` : null,
        zedStyleContext.apply_gate?.receipt_path ? `dry_run_receipt: ${zedStyleContext.apply_gate.receipt_path}` : null,
        zedStyleContext.apply_gate?.receipt_summary?.intent ? `receipt_intent: ${zedStyleContext.apply_gate.receipt_summary.intent}` : null,
        zedStyleContext.apply_gate?.receipt_summary?.edit_count !== undefined ? `receipt_edits: ${zedStyleContext.apply_gate.receipt_summary.edit_count}` : null,
        zedStyleContext.apply_gate?.receipt_summary?.message ? `receipt_message: ${zedStyleContext.apply_gate.receipt_summary.message}` : null,
        Array.isArray(zedStyleContext.apply_gate?.receipt_summary?.edits) ? `receipt_edit_previews: ${zedStyleContext.apply_gate.receipt_summary.edits.length}` : null
      ].filter(Boolean) : ["context_status: no_active_zed_style_context"];
      const reviewReady = sourceApplyReviewReady(metadataAligned, zedStyleContext, output);
      const reviewBlocker = sourceApplyReviewBlocker(metadataAligned, zedStyleContext, output);
      reviewApplyButtonEl.disabled = !reviewReady;
      reviewApplyButtonEl.textContent = reviewReady ? "Review source" : "Review gated";
      applyButtonEl.disabled = !applyReady;
      applyButtonEl.textContent = applyReady ? "Apply" : "Apply gated";
      renderPatchReview(applyGate, output);
      const sourceStatus = !metadataAligned
        ? "DX Style metadata is out of sync."
        : zedStyleContext && !contextSchemaSupported(zedStyleContext)
          ? "Zed Style context schema is unsupported."
          : payloadDiagnostics.length
            ? "Source apply payload exceeds DX Style contract limits."
            : cssDeclarationDiagnostics.length || cssDeclarationPreviewDiagnostics.length
              ? "CSS declaration source review is gated by the DX Style dry-run contract."
              : applyGate?.reason || "Source apply is gated.";
      sourceStatusEl.textContent = zedStyleContext?.token
        ? `${zedStyleContext.token} - ${reviewReady && !applyReady ? "Native review available; mutation remains gated." : sourceStatus}`
        : metadataAligned
          ? "Source apply is gated by trusted spans."
          : "DX Style metadata is out of sync.";
      metadataStatusEl.className = metadataAligned ? "pill ready" : "pill blocked";
      metadataStatusEl.textContent = metadataAligned
        ? `Metadata aligned (${metadataDiagnostics.generatorCount})`
        : "Metadata drift";
      sampleEl.dataset.previewKind = output.previewKind || "hero-card";
      sampleEl.innerHTML = previewMarkupForOutput(output);
      sampleEl.style.cssText = `
        color: white;
        display: grid;
        place-items: center;
        min-height: 160px;
        padding: 24px;
        ${output.css}
      `;
      outputEl.textContent = [
        `generator: ${state.generator}`,
        `generator_category: ${generatorMetadata.category}`,
        `generator_preferred_output: ${generatorMetadata.preferredOutput}`,
        `generator_source_edit_safety: ${generatorMetadata.sourceEditSafety}`,
        `preview_kind: ${output.previewKind || "hero-card"}`,
        `preview_anatomy: ${(output.previewAnatomy || []).join(",")}`,
        `catalog_schema: ${catalogSchema}`,
        `catalog_source: ${catalogSource}`,
        `control_schema: ${controlSchema}`,
        `control_source: ${controlSource}`,
        `recipe_schema: ${recipeSchema}`,
        `recipe_source: ${recipeSource}`,
        `recipe_value_keys: ${recipeValueKeys.length}`,
        `recipe_preview_anatomy_parts: ${recipePreviewAnatomyParts.length}`,
        `group_context_contract_schema: ${groupContextContractSchema}`,
        `group_context_contract_source: ${groupContextContractSource}`,
        `group_context_max_alias_bytes: ${groupContextMaxAliasBytes || "unknown"}`,
        `group_context_max_utility_count: ${groupContextMaxUtilityCount || "unknown"}`,
        `group_context_max_utility_bytes: ${groupContextMaxUtilityBytes || "unknown"}`,
        `group_context_candidate_min_utility_count: ${groupContextCandidateMin || "unknown"}`,
        `reverse_css_delta_contract_schema: ${reverseCssDeltaContractSchema}`,
        `reverse_css_delta_contract_source: ${reverseCssDeltaContractSource}`,
        `reverse_css_delta_mutation_enabled: ${reverseCssDeltaMutationEnabled}`,
        `reverse_css_delta_supported_properties: ${reverseCssDeltaSupportedProperties.length}`,
        `reverse_css_delta_required_guards: ${reverseCssDeltaRequiredGuards.length}`,
        `reverse_css_delta_required_provenance_fields: ${reverseCssDeltaRequiredProvenanceFields.length}`,
        `reverse_css_delta_fallback_review_properties: ${reverseCssDeltaFallbackReviewProperties.length}`,
        `reverse_css_delta_existing_utility_required_properties: ${reverseCssDeltaExistingUtilityRequiredProperties.length}`,
        `reverse_css_delta_max_replacement_utilities: ${reverseCssDeltaPayloadLimits.replacementUtilities || "unknown"}`,
        `reverse_css_delta_max_replacement_utility_bytes: ${reverseCssDeltaPayloadLimits.replacementUtilityBytes || "unknown"}`,
        `reverse_css_delta_max_replacement_source_declaration_bytes: ${reverseCssDeltaPayloadLimits.replacementSourceDeclarationBytes || "unknown"}`,
        `reverse_css_delta_contract_diagnostics: ${reverseDeltaContractDiagnostics.length}`,
        ...reverseDeltaContractDiagnostics.map((diagnostic) => `reverse_css_delta_contract_diagnostic: ${diagnostic}`),
        `reverse_css_delta_preview_provenance_diagnostics: ${reverseDeltaPreviewProvenanceDiagnostics.length}`,
        ...reverseDeltaPreviewProvenanceDiagnostics.map((diagnostic) => `reverse_css_delta_preview_provenance_diagnostic: ${diagnostic}`),
        `reverse_css_delta_replacement_payload_diagnostics: ${reverseDeltaReplacementPayloadDiagnostics.length}`,
        ...reverseDeltaReplacementPayloadDiagnostics.map((diagnostic) => `reverse_css_delta_replacement_payload_diagnostic: ${diagnostic}`),
        reverseCssDeltaExample?.target_utility ? `reverse_css_delta_example_target: ${reverseCssDeltaExample.target_utility}` : null,
        `reverse_css_delta_live_status: ${reverseDeltaPreview.status || "not_available"}`,
        reverseDeltaPreview.target_utility ? `reverse_css_delta_live_target: ${reverseDeltaPreview.target_utility}` : null,
        reverseDeltaPreview.replacement_source_declaration ? `reverse_css_delta_live_source: ${reverseDeltaPreview.replacement_source_declaration}` : null,
        Array.isArray(reverseDeltaPreview.replacement_utilities) ? `reverse_css_delta_live_utilities: ${reverseDeltaPreview.replacement_utilities.length}` : null,
        reverseDeltaPreview.replacement_existing_utility_required !== undefined ? `reverse_css_delta_live_replacement_existing_utility_required: ${reverseDeltaPreview.replacement_existing_utility_required}` : null,
        reverseDeltaPreview.replacement_existing_utility_found !== undefined ? `reverse_css_delta_live_replacement_existing_utility_found: ${reverseDeltaPreview.replacement_existing_utility_found}` : null,
        reverseDeltaPreview.group_alias ? `reverse_css_delta_live_group_alias: ${reverseDeltaPreview.group_alias}` : null,
        reverseDeltaPreview.group_registry_receipt ? `reverse_css_delta_live_group_registry_receipt: ${reverseDeltaPreview.group_registry_receipt}` : null,
        reverseDeltaPreview.reverse_css_map_status ? `reverse_css_delta_live_reverse_map_status: ${reverseDeltaPreview.reverse_css_map_status}` : null,
        reverseDeltaPreview.reverse_css_map_receipt ? `reverse_css_delta_live_reverse_map_receipt: ${reverseDeltaPreview.reverse_css_map_receipt}` : null,
        `source_apply_contract_schema: ${sourceApplyContractSchema}`,
        `source_apply_contract_source: ${sourceApplyContractSource}`,
        `source_apply_contract_version: ${sourceApplyContractVersion}`,
        `source_apply_contract_scope: ${sourceApplyContractScope}`,
        `source_apply_ipc_kind: ${sourceApplyIpcKind}`,
        `source_apply_receipt_schema: ${sourceApplyReceiptSchema}`,
        `source_apply_context_schema: ${expectedContextSchema}`,
        `source_apply_mutation_enabled: ${sourceApplyMutationEnabled}`,
        `source_apply_required_editor_guards: ${sourceApplyRequiredEditorGuards.length}`,
        `source_apply_required_handler_capabilities: ${sourceApplyRequiredHandlerCapabilities.length}`,
        `source_apply_review_context_kinds: ${sourceApplyReviewContextKinds.length}`,
        `source_apply_mutation_context_kinds: ${sourceApplyMutationContextKinds.length}`,
        `source_apply_review_receipt_fields: ${sourceApplyReviewReceiptFields.length}`,
        `css_declaration_dry_run_contract_schema: ${cssDeclarationDryRunSchema}`,
        `css_declaration_dry_run_contract_source: ${cssDeclarationDryRunSource}`,
        `css_declaration_dry_run_context_kind: ${cssDeclarationDryRunContextKind}`,
        `css_declaration_dry_run_mutation_enabled: ${cssDeclarationDryRunMutationEnabled}`,
        `css_declaration_dry_run_required_context_fields: ${cssDeclarationDryRunRequiredFields.length}`,
        `css_declaration_dry_run_accepted_safety: ${cssDeclarationDryRunAcceptedSafety.length}`,
        `css_declaration_dry_run_review_receipt_fields: ${cssDeclarationDryRunReviewReceiptFields.length}`,
        `css_declaration_dry_run_max_declaration_bytes: ${cssDeclarationDryRunMaxDeclarationBytes || "unknown"}`,
        `css_declaration_dry_run_max_diagnostic_count: ${cssDeclarationDryRunMaxDiagnosticCount || "unknown"}`,
        `css_declaration_dry_run_max_diagnostic_bytes: ${cssDeclarationDryRunMaxDiagnosticBytes || "unknown"}`,
        `css_declaration_dry_run_max_source_path_bytes: ${cssDeclarationDryRunMaxSourcePathBytes || "unknown"}`,
        `css_declaration_dry_run_max_source_span_bytes: ${cssDeclarationDryRunMaxSourceSpanBytes || "unknown"}`,
        `css_declaration_dry_run_max_source_digest_bytes: ${cssDeclarationDryRunMaxSourceDigestBytes || "unknown"}`,
        `css_declaration_dry_run_diagnostics: ${cssDeclarationDiagnostics.length}`,
        ...cssDeclarationDiagnostics.map((diagnostic) => `css_declaration_dry_run_diagnostic: ${diagnostic}`),
        `css_declaration_dry_run_preview_diagnostics: ${cssDeclarationPreviewDiagnostics.length}`,
        ...cssDeclarationPreviewDiagnostics.map((diagnostic) => `css_declaration_dry_run_preview_diagnostic: ${diagnostic}`),
        `css_declaration_dry_run_preview_status: ${cssDeclarationPreview.status || "not_available"}`,
        cssDeclarationPreview.property ? `css_declaration_dry_run_preview_property: ${cssDeclarationPreview.property}` : null,
        cssDeclarationPreview.proposed_declaration ? `css_declaration_dry_run_preview_declaration: ${cssDeclarationPreview.proposed_declaration}` : null,
        `source_apply_max_source_path_bytes: ${sourceApplyByteLimits.sourcePath || "unknown"}`,
        `source_apply_max_class_name_bytes: ${sourceApplyByteLimits.className || "unknown"}`,
        `source_apply_max_css_bytes: ${sourceApplyByteLimits.css || "unknown"}`,
        `source_apply_max_generator_id_bytes: ${sourceApplyByteLimits.generator || "unknown"}`,
        `source_apply_max_source_span_bytes: ${sourceApplyContract.max_source_span_bytes || "unknown"}`,
        `source_apply_max_source_digest_bytes: ${sourceApplyByteLimits.sourceDigest || "unknown"}`,
        `source_apply_max_dry_run_edit_previews: ${sourceApplyByteLimits.dryRunEditPreviews || "unknown"}`,
        `source_apply_max_dry_run_replacement_text_bytes: ${sourceApplyByteLimits.dryRunReplacementText || "unknown"}`,
        `source_apply_max_preview_kind_bytes: ${sourceApplyByteLimits.previewKind || "unknown"}`,
        `source_apply_max_preview_anatomy_part_bytes: ${sourceApplyByteLimits.previewAnatomyPart || "unknown"}`,
        `source_apply_max_preview_anatomy_parts: ${sourceApplyByteLimits.previewAnatomyParts || "unknown"}`,
        `source_apply_payload_diagnostics: ${payloadDiagnostics.length}`,
        ...payloadDiagnostics.map((diagnostic) => `source_apply_payload_diagnostic: ${diagnostic}`),
        `metadata_status: ${metadataDiagnostics.status}`,
        `metadata_generators: ${metadataDiagnostics.generatorCount}`,
        `metadata_missing_controls: ${metadataDiagnostics.missingControls.length}`,
        `metadata_missing_recipes: ${metadataDiagnostics.missingRecipes.length}`,
        `metadata_extra_controls: ${metadataDiagnostics.extraControls.length}`,
        `metadata_extra_recipes: ${metadataDiagnostics.extraRecipes.length}`,
        `metadata_unsupported_placeholders: ${metadataDiagnostics.unsupportedRecipePlaceholders.length}`,
        `metadata_missing_preview_anatomy: ${metadataDiagnostics.missingPreviewAnatomy.length}`,
        `metadata_unsupported_preview_anatomy: ${metadataDiagnostics.unsupportedPreviewAnatomy.length}`,
        `web_preview_apply_handler: ${sourceApplyHandlerState()}`,
        `source_apply_review_ready: ${reviewReady}`,
        `source_apply_review_blocker: ${reviewBlocker}`,
        `source_apply_ready: ${applyReady}`,
        `source_apply_blocker: ${applyBlocker}`,
        `class: ${output.className}`,
        ...contextLines,
        "",
        output.css,
        "",
        "source_apply: disabled_until_trusted_grouped_class_source_span_and_dry_run_receipt"
      ].join("\n");
    }

    function renderCatalog() {
      catalogEl.replaceChildren();
      selectEl.replaceChildren();
      for (const [id, label] of orderedCatalog()) {
        const option = document.createElement("option");
        option.value = id;
        option.textContent = label;
        selectEl.append(option);
      }
      selectEl.value = state.generator;

      const visibleCatalog = filteredCatalog();
      if (!visibleCatalog.length) {
        const empty = document.createElement("div");
        empty.className = "catalog-empty";
        empty.textContent = "No generators match this filter.";
        catalogEl.append(empty);
        return;
      }
      for (const [id, label, category, _hints, preferredOutput, sourceEditSafety] of visibleCatalog) {
        const button = document.createElement("button");
        button.className = "generator";
        button.type = "button";
        button.setAttribute("aria-current", String(id === state.generator));
        button.innerHTML = `<strong>${escapeHtml(label)}</strong><span>${escapeHtml(category)} - ${escapeHtml(preferredOutput || "unknown")} - ${escapeHtml(sourceEditSafety || "unknown")}</span>`;
        button.addEventListener("click", () => setGenerator(id));
        catalogEl.append(button);
      }
    }

    function render() {
      renderCatalog();
      renderControls();
      updatePreview();
    }

    selectEl.addEventListener("change", () => setGenerator(selectEl.value));
    generatorSearchEl.addEventListener("input", () => {
      state.catalogQuery = generatorSearchEl.value;
      renderCatalog();
    });
    copyClassButtonEl.addEventListener("click", () => handleCopy("class"));
    copyCssButtonEl.addEventListener("click", () => handleCopy("css"));
    copyReviewButtonEl.addEventListener("click", () => handleCopy("review"));
    reviewApplyButtonEl.addEventListener("click", handleReviewApplyClick);
    applyButtonEl.addEventListener("click", handleApplyClick);
    render();
"##;

pub(super) fn dx_style_generator_script() -> String {
    DX_STYLE_GENERATOR_SCRIPT
        .replace(
            "__DX_STYLE_SOURCE_APPLY_SESSION_CONSTANTS__",
            dx_style_source_apply_session_constants_script(),
        )
        .replace(
            "__DX_STYLE_SOURCE_APPLY_SESSION_HANDLER__",
            dx_style_source_apply_session_handler_script(),
        )
        .replace(
            "__DX_STYLE_CSS_DECLARATION_DRY_RUN_CONSTANTS__",
            dx_style_css_declaration_dry_run_constants_script(),
        )
        .replace(
            "__DX_STYLE_CSS_DECLARATION_DRY_RUN_REVIEW__",
            dx_style_css_declaration_dry_run_review_script(),
        )
}
