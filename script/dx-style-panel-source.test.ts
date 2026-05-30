import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");
const lineCount = (path: string) => read(path).split(/\r?\n/).length;

const embeddedRustRawString = (source: string, constName: string) => {
  const match = source.match(
    new RegExp(`const ${constName}: &str = r##"([\\s\\S]*?)"##;`),
  );
  assert.ok(match, `${constName} should be a Rust raw string constant`);
  return match[1];
};

const parseableDxStyleGeneratorScript = (source: string) => {
  const script = embeddedRustRawString(source, "DX_STYLE_GENERATOR_SCRIPT");
  const sourceApplySessionScript = read(
    "crates/web_preview/src/dx_style_generator_surface/source_apply_session_script.rs",
  );
  const cssDeclarationDryRunScript = read(
    "crates/web_preview/src/dx_style_generator_surface/css_declaration_dry_run_script.rs",
  );
  const replacements = {
    __DX_STYLE_GENERATOR_CATALOG_JSON__:
      '{"__schema":"dx.style.visual-generator-catalog","entries":[]}',
    __DX_STYLE_GENERATOR_CONTROLS_JSON__:
      '{"__schema":"dx.style.visual-generator-control-catalog","entries":[]}',
    __DX_STYLE_GENERATOR_RECIPES_JSON__:
      '{"__schema":"dx.style.visual-generator-recipe-catalog","entries":[]}',
    __DX_STYLE_SOURCE_APPLY_CONTRACT_JSON__:
      `{
        "__schema":"dx.style.grouped-class-source-apply-contract",
        "__source":"test:source-apply-contract",
        "schema_version":1,
        "scope":"source-owned IPC and review receipt requirements for grouped-class editor apply",
        "ipc_kind":"dx-style-source-apply",
        "receipt_schema":"zed.web_preview.dx_style_source_apply_receipt.v1",
        "active_context_schema":"zed.dx_style.active_context.v1",
        "source_apply_session_kind":"zed.web_preview.dx_style.source_apply_session",
        "source_mutation_enabled":false,
        "required_native_handler":"window.__DX_STYLE_SOURCE_APPLY__",
        "required_native_handler_capabilities":["can_review_request","can_mutate_source"],
        "review_context_kinds":["class_token","class_list","css_declaration"],
        "mutation_context_kinds_when_enabled":["class_token"],
        "required_editor_guards":["trusted Web Preview source-apply session","cursor-scoped dry-run structured edit preview"],
        "review_receipt_fields":["source_apply_session","dry_run_edit_review","source_write_readiness"],
        "max_source_apply_session_token_bytes":256,
        "max_dry_run_edit_previews":3,
        "max_dry_run_replacement_text_bytes":4096
      }`,
    __DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_JSON__:
      '{"__schema":"dx.style.css-declaration-dry-run-contract"}',
    __DX_STYLE_GROUP_CONTEXT_CONTRACT_JSON__:
      '{"__schema":"dx.style.grouped-class-web-preview-context"}',
    __DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_JSON__:
      '{"__schema":"dx.style.grouped-class-reverse-css-delta-contract"}',
    __DX_STYLE_CONTEXT_JSON_STRING__: '""',
    __DX_STYLE_SOURCE_APPLY_SESSION_CONSTANTS__: embeddedRustRawString(
      sourceApplySessionScript,
      "DX_STYLE_SOURCE_APPLY_SESSION_CONSTANTS_SCRIPT",
    ),
    __DX_STYLE_SOURCE_APPLY_SESSION_HANDLER__: embeddedRustRawString(
      sourceApplySessionScript,
      "DX_STYLE_SOURCE_APPLY_SESSION_HANDLER_SCRIPT",
    ),
    __DX_STYLE_CSS_DECLARATION_DRY_RUN_CONSTANTS__: embeddedRustRawString(
      cssDeclarationDryRunScript,
      "DX_STYLE_CSS_DECLARATION_DRY_RUN_CONSTANTS_SCRIPT",
    ),
    __DX_STYLE_CSS_DECLARATION_DRY_RUN_REVIEW__: embeddedRustRawString(
      cssDeclarationDryRunScript,
      "DX_STYLE_CSS_DECLARATION_DRY_RUN_REVIEW_SCRIPT",
    ),
    __DX_STYLE_SOURCE_APPLY_SESSION_TOKEN__: '"source-apply-session-test-token"',
  };
  let parseable = script;
  for (const [token, replacement] of Object.entries(replacements)) {
    parseable = parseable.split(token).join(replacement);
    assert.doesNotMatch(parseable, new RegExp(token));
  }
  assert.doesNotThrow(() => new Function(parseable));
  return parseable;
};

const dxStyleRoot = () => {
  const candidates = [
    process.env.DX_STYLE_ROOT,
    "G:/Dx/style",
    join(process.cwd(), "..", "style"),
  ];

  for (const candidate of candidates) {
    if (candidate && existsSync(join(candidate, "PLAN.md"))) return candidate;
  }

  assert.fail(
    "DX Style root should be discoverable from DX_STYLE_ROOT, G:/Dx/style, or ../style",
  );
};

const readStyle = (path: string) => read(join(dxStyleRoot(), path));

const expectedEditorWriteBridgeReceipts = [
  "dx.style.grouped-class-dry-run-receipt",
  "dx.style.grouped-class-source-digest",
  "dx.style.grouped-class-source-apply-contract",
  "zed.web_preview.dx_style_source_apply_receipt.v1",
  "zed.web_preview.dx_style.active_editor_source_revalidation",
];

const expectedEditorWriteBridgeGuards = [
  "active context schema match",
  "active source path match",
  "request source span matches active source span",
  "active source length match",
  "active source digest match",
  "trusted Web Preview source-apply session",
  "session-bound source identity",
  "native active editor source revalidation",
  "same-session native editor identity",
  "trusted grouped-class dry-run receipt",
  "cursor-scoped dry-run structured edit preview",
  "reverse CSS delta preview provenance match",
  "CSS declaration dry-run receipt for CSS contexts",
  "editor write bridge can_apply",
  "explicit user apply action",
  "authorized runtime validation",
];

const expectedEditorWriteBridgeHandlers = ["window.__DX_STYLE_SOURCE_APPLY__"];
const expectedEditorWriteBridgeCapabilities = ["can_review_request", "can_mutate_source"];

const rustStringVec = (source: string, field: string) => {
  const match = source.match(new RegExp(`${field}: vec!\\[([\\s\\S]*?)\\],`));
  assert.ok(match, `${field} should be a Rust vec`);
  return [...match[1].matchAll(/"([^"]+)"(?:\.to_string\(\))?/g)].map(
    (item) => item[1],
  );
};

const collectRecipeTriples = (source: string, idField: string) => {
  const pattern = new RegExp(
    `${idField}: "([^"]+)",[\\s\\S]*?class_template: "([^"]+)",[\\s\\S]*?css_template: "([^"]+)"`,
    "g",
  );
  return [...source.matchAll(pattern)]
    .map((match) => ({
      id: match[1],
      classTemplate: match[2],
      cssTemplate: match[3].replace(/\\n/g, "\n"),
    }))
    .filter((recipe) => recipe.id !== "default");
};

const templatePlaceholders = (template: string) =>
  [...template.matchAll(/\{([a-z0-9_]+)\}/g)].map((match) => match[1]);

const controlInputName = (name: string) => name.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();

const enumName = controlInputName;

const optionalNumber = (value: string) => {
  const match = value.match(/^Some\((-?\d+)\)$/);
  return match ? Number(match[1]) : undefined;
};

const collectControlEntries = (source: string) => {
  const specPattern =
    /const ([A-Z_]+): VisualGeneratorControlSpec = VisualGeneratorControlSpec \{\s*key: "([^"]+)",\s*label: "([^"]+)",\s*input: VisualGeneratorControlInput::([A-Za-z]+),\s*min: (None|Some\(-?\d+\)),\s*max: (None|Some\(-?\d+\)),\s*step: (None|Some\(-?\d+\)),\s*\};/g;
  const specs = new Map(
    [...source.matchAll(specPattern)].map((match) => {
      const control = {
        key: match[2],
        label: match[3],
        input: controlInputName(match[4]),
      };
      for (const [field, value] of [
        ["min", match[5]],
        ["max", match[6]],
        ["step", match[7]],
      ]) {
        const number = optionalNumber(value);
        if (number !== undefined) control[field] = number;
      }
      return [match[1], control];
    }),
  );

  const slicePattern =
    /const ([A-Z_]+): &\[VisualGeneratorControlSpec\] = &\[([^\]]+)\];/g;
  const slices = new Map(
    [...source.matchAll(slicePattern)].map((match) => [
      match[1],
      match[2]
        .split(",")
        .map((name) => name.trim())
        .filter(Boolean)
        .map((name) => specs.get(name)),
    ]),
  );

  const entryPattern =
    /VisualGeneratorControlCatalogEntry \{\s*generator_id: "([^"]+)",\s*controls: ([A-Z_]+),\s*\}/g;
  return [...source.matchAll(entryPattern)].map((match) => ({
    id: match[1],
    controls: slices.get(match[2]),
  }));
};

const collectVisualGeneratorEntries = (source: string) => {
  const pattern =
    /VisualGeneratorCatalogEntry \{\s*ordinal: (\d+),\s*generator_id: "([^"]+)",\s*label: "([^"]+)",\s*category: VisualGeneratorCategory::([A-Za-z]+),\s*applicable_class_families: &\[([\s\S]*?)\],\s*preferred_output: VisualGeneratorOutputPreference::([A-Za-z]+),\s*source_edit_safety: VisualGeneratorSourceEditSafety::([A-Za-z]+),\s*\}/g;
  return [...source.matchAll(pattern)].map((match) => ({
    ordinal: Number(match[1]),
    generator_id: match[2],
    label: match[3],
    category: enumName(match[4]),
    applicable_class_families: [...match[5].matchAll(/"([^"]+)"/g)].map(
      (family) => family[1],
    ),
    preferred_output: enumName(match[6]),
    source_edit_safety: enumName(match[7]),
  }));
};

const collectCssHintEntries = (source: string) => {
  const pattern =
    /VisualGeneratorCssDeclarationHintEntry \{\s*ordinal: (\d+),\s*property_pattern: "([^"]+)",\s*property_match: ([A-Za-z]+),\s*value_contains: &\[([\s\S]*?)\],\s*token_hint: "([^"]+)",\s*generator_id: "([^"]+)",\s*source_edit_safety: ([A-Za-z]+),\s*\}/g;
  return [...source.matchAll(pattern)].map((match) => ({
    ordinal: Number(match[1]),
    property_pattern: match[2],
    property_match: enumName(match[3]),
    value_contains: [...match[4].matchAll(/"([^"]+)"/g)].map((value) => value[1]),
    token_hint: match[5],
    generator_id: match[6],
    source_edit_safety: enumName(match[7]),
  }));
};

test("DX Style grouped-class read model is source-owned and editor-facing", () => {
  const readme = readStyle("README.md");
  const groupRegistry = readStyle("src/core/group/mod.rs");
  const contract = readStyle("src/core/engine/grouped_class_contract.rs");
  const readModel = readStyle("src/core/engine/grouped_class_read_model.rs");
  const cursorContext = readStyle("src/core/engine/grouped_class_cursor_context.rs");
  const dryRunReceipt = readStyle("src/core/engine/grouped_class_dry_run_receipt.rs");
  const editorWriteBridgePreflight = readStyle(
    "src/core/engine/grouped_class_editor_write_bridge.rs",
  );
  const editorWriteBridgeFixture = JSON.parse(
    readStyle("fixtures/grouped-class-editor-write-bridge-preflight.json"),
  );
  const sourceDigest = readStyle("src/core/engine/grouped_class_source_digest.rs");
  const sourceApplyContract = readStyle(
    "src/core/engine/grouped_class_source_apply.rs",
  );
  const sourceApplyFixture = JSON.parse(
    readStyle("fixtures/grouped-class-source-apply-contract.json"),
  );
  const groupWebPreviewContext = readStyle(
    "src/core/engine/grouped_class_web_preview_context.rs",
  );
  const groupWebPreviewContextFixture = JSON.parse(
    readStyle("fixtures/grouped-class-web-preview-context.json"),
  );
  const groupRegistryReceipt = readStyle(
    "src/core/engine/grouped_class_registry_receipt.rs",
  );
  const groupReverseCssMap = readStyle(
    "src/core/engine/grouped_class_reverse_css_map.rs",
  );
  const groupReverseCssDelta = readStyle(
    "src/core/engine/grouped_class_reverse_css_delta.rs",
  );
  const coreMod = readStyle("src/core/mod.rs");
  const groupRegistryReceiptFixture = JSON.parse(
    readStyle("fixtures/grouped-class-registry-receipt.json"),
  );
  const groupReverseCssMapFixture = JSON.parse(
    readStyle("fixtures/grouped-class-reverse-css-map.json"),
  );
  const groupReverseCssDeltaFixture = JSON.parse(
    readStyle("fixtures/grouped-class-reverse-css-delta-contract.json"),
  );
  const generatorCatalog = readStyle("src/core/engine/visual_generator_catalog.rs");
  const generatorCatalogFixture = JSON.parse(
    readStyle("fixtures/visual-generator-catalog.json"),
  );
  const generatorRecipeCatalog = readStyle(
    "src/core/engine/visual_generator_recipe_catalog.rs",
  );
  const generatorControlCatalog = readStyle(
    "src/core/engine/visual_generator_control_catalog.rs",
  );
  const generatorCssHintCatalog = readStyle(
    "src/core/engine/visual_generator_css_hint_catalog.rs",
  );
  const cssDeclarationDryRunContract = readStyle(
    "src/core/engine/css_declaration_dry_run.rs",
  );
  const generatorRecipeFixture = JSON.parse(
    readStyle("fixtures/visual-generator-recipe-catalog.json"),
  );
  const generatorControlFixture = JSON.parse(
    readStyle("fixtures/visual-generator-control-catalog.json"),
  );
  const generatorCssHintFixture = JSON.parse(
    readStyle("fixtures/visual-generator-css-declaration-hint-catalog.json"),
  );
  const cssDeclarationDryRunFixture = JSON.parse(
    readStyle("fixtures/css-declaration-dry-run-contract.json"),
  );
  const engineMod = readStyle("src/core/engine/mod.rs");
  const fixtureMirrorScript = readStyle("scripts/sync_zed_visual_generator_fixtures.mjs");
  const receiptFixtures = JSON.parse(
    readStyle("fixtures/grouped-class-dry-run-receipt-fixtures.json"),
  );
  const receiptFixturesGenerated = JSON.parse(
    read("crates/agent_ui/src/dx_style_panel/grouped-class-dry-run-receipt-fixtures.generated.json"),
  );

  assert.match(readme, /stable read model/);
  assert.match(readme, /dx\.style\.group-two-way-contract/);
  assert.match(readme, /foundation for Zed's Style panel/);
  assert.match(
    readme,
    /without treating browser-loaded generated CSS as the source of truth/,
  );

  assert.match(
    contract,
    /This contract is metadata only\. It must not change generated CSS bytes/,
  );
  assert.match(
    contract,
    /treat browser-loaded CSS as source of truth/,
  );
  assert.match(
    contract,
    /pub const GROUPED_CLASS_CONTRACT_SCHEMA: &str = "dx\.style\.group-two-way-contract";/,
  );
  assert.match(contract, /pub const GROUPED_CLASS_CONTRACT_VERSION: u8 = [12];/);
  assert.match(
    contract,
    /pub const GROUPED_CLASS_CONTRACT_SCOPE: &str = "group alias to atomic utilities, source declaration, generated css, .*invertibility metadata/,
  );
  assert.match(
    contract,
    /pub const GROUPED_CLASS_CONTRACT_CONSUMERS: &\[&str\] = &\["dx-style", "dx-check", "Zed", "Friday"\];/,
  );
  assert.match(contract, /read_model_capabilities/);
  assert.match(contract, /pub entries: Vec<GroupTwoWayEntry>/);
  assert.match(contract, /GroupTwoWayInvertibility/);
  assert.match(groupRegistry, /GroupTwoWaySizeEstimate/);
  assert.match(groupRegistry, /GroupTwoWayRecommendedRepresentation/);
  assert.match(groupRegistry, /estimated_saved_bytes_per_reuse/);
  assert.match(groupRegistry, /recommended_representation/);
  assert.match(groupRegistry, /CompactGroupCall/);
  assert.match(contract, /GROUPED_CLASS_CONTRACT_VERSION: u8 = 2/);
  assert.match(contract, /read_model_capabilities/);
  assert.match(readModel, /GroupedClassSourceSpan/);
  assert.match(readModel, /GroupedClassDryRunPatchPreview/);
  assert.match(readModel, /cursor_token_context: true/);
  assert.match(readModel, /grouping_efficiency_estimates: true/);
  assert.match(readModel, /broad_tsx_ast_rewrites: false/);
  assert.match(cursorContext, /GROUPED_CLASS_CURSOR_CONTEXT_SCHEMA/);
  assert.match(cursorContext, /grouped_class_cursor_context/);
  assert.match(cursorContext, /GroupedClassCursorToken/);
  assert.match(cursorContext, /DynamicExpression/);
  assert.match(cursorContext, /keeps_group_call_with_spaces_as_one_token/);
  assert.match(cursorContext, /without applying source edits/);
  assert.match(dryRunReceipt, /GROUPED_CLASS_DRY_RUN_RECEIPT_SCHEMA/);
  assert.match(dryRunReceipt, /GroupedClassDryRunReceipt/);
  assert.match(dryRunReceipt, /source_digest_algorithm/);
  assert.match(dryRunReceipt, /ready_from_source/);
  assert.match(dryRunReceipt, /grouped_class_source_digest_is_current/);
  assert.match(dryRunReceipt, /source_digest_verified/);
  assert.match(dryRunReceipt, /editor_write_bridge_required/);
  assert.match(dryRunReceipt, /not editor writes by/);
  assert.match(editorWriteBridgePreflight, /GROUPED_CLASS_EDITOR_WRITE_BRIDGE_SCHEMA/);
  assert.match(editorWriteBridgePreflight, /grouped-class-editor-write-bridge-preflight/);
  assert.match(editorWriteBridgePreflight, /can_mutate_source: false/);
  assert.match(editorWriteBridgePreflight, /runtime_validation_required: true/);
  assert.match(editorWriteBridgePreflight, /active source digest match/);
  assert.match(editorWriteBridgePreflight, /same-session native editor identity/);
  assert.match(editorWriteBridgePreflight, /cursor-scoped dry-run structured edit preview/);
  assert.match(editorWriteBridgePreflight, /authorized runtime validation/);
  assert.match(editorWriteBridgePreflight, /required_native_handlers/);
  assert.match(editorWriteBridgePreflight, /required_native_handler_capabilities/);
  assert.match(editorWriteBridgePreflight, /window\.__DX_STYLE_SOURCE_APPLY__/);
  assert.match(editorWriteBridgePreflight, /can_mutate_source/);
  assert.equal(
    editorWriteBridgeFixture.schema,
    "dx.style.grouped-class-editor-write-bridge-preflight",
  );
  assert.equal(editorWriteBridgeFixture.schema_version, 1);
  assert.equal(
    editorWriteBridgeFixture.scope,
    "preflight requirements for trusted grouped-class editor source writes",
  );
  assert.equal(editorWriteBridgeFixture.can_mutate_source, false);
  assert.equal(editorWriteBridgeFixture.status, "not_enabled");
  assert.equal(editorWriteBridgeFixture.runtime_validation_required, true);
  assert.deepEqual(editorWriteBridgeFixture.required_receipts, expectedEditorWriteBridgeReceipts);
  assert.deepEqual(editorWriteBridgeFixture.required_editor_guards, expectedEditorWriteBridgeGuards);
  assert.deepEqual(editorWriteBridgeFixture.required_native_handlers, expectedEditorWriteBridgeHandlers);
  assert.deepEqual(
    editorWriteBridgeFixture.required_native_handler_capabilities,
    expectedEditorWriteBridgeCapabilities,
  );
  assert.deepEqual(
    rustStringVec(editorWriteBridgePreflight, "required_receipts"),
    expectedEditorWriteBridgeReceipts,
  );
  assert.deepEqual(
    rustStringVec(editorWriteBridgePreflight, "required_editor_guards"),
    expectedEditorWriteBridgeGuards,
  );
  assert.doesNotMatch(
    editorWriteBridgePreflight,
    /bounded edit preview review/,
  );
  assert.ok(
    editorWriteBridgeFixture.required_native_handlers.includes(
      "window.__DX_STYLE_SOURCE_APPLY__",
    ),
  );
  assert.ok(
    editorWriteBridgeFixture.required_native_handler_capabilities.includes(
      "can_review_request",
    ),
  );
  assert.ok(
    editorWriteBridgeFixture.required_native_handler_capabilities.includes(
      "can_mutate_source",
    ),
  );
  assert.match(sourceDigest, /GROUPED_CLASS_SOURCE_DIGEST_SCHEMA/);
  assert.match(sourceDigest, /GROUPED_CLASS_SOURCE_DIGEST_ALGORITHM/);
  assert.match(sourceDigest, /GROUPED_CLASS_SOURCE_DIGEST_PREFIX/);
  assert.match(sourceDigest, /grouped_class_source_digest/);
  assert.match(sourceDigest, /fnv1a64/);
  assert.match(sourceApplyContract, /GROUPED_CLASS_SOURCE_APPLY_CONTRACT_SCHEMA/);
  assert.match(sourceApplyContract, /dx\.style\.grouped-class-source-apply-contract/);
  assert.match(sourceApplyContract, /GROUPED_CLASS_SOURCE_APPLY_IPC_KIND/);
  assert.match(sourceApplyContract, /dx-style-source-apply/);
  assert.match(sourceApplyContract, /GROUPED_CLASS_SOURCE_APPLY_ACTIVE_CONTEXT_SCHEMA/);
  assert.match(sourceApplyContract, /zed\.dx_style\.active_context\.v1/);
  assert.match(sourceApplyContract, /GROUPED_CLASS_SOURCE_APPLY_RECEIPT_SCHEMA/);
  assert.match(sourceApplyContract, /zed\.web_preview\.dx_style_source_apply_receipt\.v1/);
  assert.match(sourceApplyContract, /source_mutation_enabled: false/);
  assert.match(sourceApplyContract, /required_native_handler_capabilities/);
  assert.match(sourceApplyContract, /can_review_request/);
  assert.match(sourceApplyContract, /can_mutate_source/);
  assert.match(sourceApplyContract, /review_context_kinds/);
  assert.match(sourceApplyContract, /mutation_context_kinds_when_enabled/);
  assert.match(sourceApplyContract, /css_declaration/);
  assert.match(sourceApplyContract, /active context kind supported/);
  assert.match(sourceApplyContract, /review_receipt_fields/);
  assert.match(sourceApplyContract, /dry_run_review/);
  assert.match(sourceApplyContract, /dry_run_edit_review/);
  assert.match(sourceApplyContract, /source_write_readiness/);
  assert.match(sourceApplyContract, /context_kind/);
  assert.match(sourceApplyContract, /css_source_edit_safety/);
  assert.match(sourceApplyContract, /mutation_ready/);
  assert.match(sourceApplyContract, /GROUPED_CLASS_SOURCE_APPLY_MAX_SOURCE_DIGEST_BYTES/);
  assert.match(sourceApplyContract, /reverse CSS map receipt match/);
  assert.match(sourceApplyContract, /generated CSS declaration delta validation/);
  assert.match(sourceApplyContract, /CSS declaration dry-run receipt for CSS contexts/);
  assert.match(sourceApplyContract, /reverse CSS delta preview provenance match/);
  assert.match(sourceApplyContract, /cursor-scoped dry-run structured edit preview/);
  assert.match(sourceApplyContract, /trusted grouped-class dry-run receipt/);
  assert.match(
    sourceApplyContract,
    /GROUPED_CLASS_SOURCE_APPLY_MAX_DRY_RUN_EDIT_PREVIEWS: usize = 3/,
  );
  assert.match(
    sourceApplyContract,
    /GROUPED_CLASS_SOURCE_APPLY_MAX_DRY_RUN_REPLACEMENT_TEXT_BYTES: usize = 4096/,
  );
  assert.equal(sourceApplyFixture.schema, "dx.style.grouped-class-source-apply-contract");
  assert.equal(sourceApplyFixture.ipc_kind, "dx-style-source-apply");
  assert.equal(
    sourceApplyFixture.receipt_schema,
    "zed.web_preview.dx_style_source_apply_receipt.v1",
  );
  assert.equal(sourceApplyFixture.active_context_schema, "zed.dx_style.active_context.v1");
  assert.equal(sourceApplyFixture.source_mutation_enabled, false);
  assert.equal(
    sourceApplyFixture.required_native_handler,
    "window.__DX_STYLE_SOURCE_APPLY__",
  );
  assert.ok(
    sourceApplyFixture.required_native_handler_capabilities.includes(
      "can_review_request",
    ),
  );
  assert.ok(
    sourceApplyFixture.required_native_handler_capabilities.includes(
      "can_mutate_source",
    ),
  );
  assert.ok(
    sourceApplyFixture.required_editor_guards.includes(
      "active context kind supported",
    ),
  );
  assert.ok(
    sourceApplyFixture.required_editor_guards.includes(
      "session-bound source identity",
    ),
  );
  assert.ok(
    sourceApplyFixture.review_context_kinds.includes("css_declaration"),
  );
  assert.ok(
    sourceApplyFixture.mutation_context_kinds_when_enabled.includes("class_token"),
  );
  assert.ok(
    sourceApplyFixture.required_editor_guards.includes(
      "visual-generator metadata alignment",
    ),
  );
  assert.ok(
    sourceApplyFixture.required_editor_guards.includes(
      "reverse CSS map receipt match",
    ),
  );
  assert.ok(
    sourceApplyFixture.required_editor_guards.includes(
      "generated CSS declaration delta validation",
    ),
  );
  assert.ok(
    sourceApplyFixture.required_editor_guards.includes(
      "reverse CSS delta preview provenance match",
    ),
  );
  assert.ok(
    sourceApplyFixture.required_editor_guards.includes(
      "trusted grouped-class dry-run receipt",
    ),
  );
  assert.ok(
    sourceApplyFixture.required_editor_guards.includes(
      "cursor-scoped dry-run structured edit preview",
    ),
  );
  assert.ok(sourceApplyFixture.review_receipt_fields.includes("dry_run_review"));
  assert.ok(sourceApplyFixture.review_receipt_fields.includes("dry_run_edit_review"));
  assert.ok(sourceApplyFixture.review_receipt_fields.includes("source_write_readiness"));
  assert.ok(sourceApplyFixture.review_receipt_fields.includes("context_kind"));
  assert.ok(
    sourceApplyFixture.review_receipt_fields.includes("css_source_edit_safety"),
  );
  assert.ok(
    sourceApplyFixture.required_editor_guards.includes(
      "source-owned preview output metadata",
    ),
  );
  assert.ok(sourceApplyFixture.review_receipt_fields.includes("preview_output"));
  assert.ok(sourceApplyFixture.review_receipt_fields.includes("mutation_ready"));
  assert.equal(sourceApplyFixture.max_source_path_bytes, 4096);
  assert.equal(sourceApplyFixture.max_class_name_bytes, 4096);
  assert.equal(sourceApplyFixture.max_css_bytes, 32768);
  assert.equal(sourceApplyFixture.max_generator_id_bytes, 128);
  assert.equal(sourceApplyFixture.max_source_span_bytes, 16384);
  assert.equal(sourceApplyFixture.max_source_digest_bytes, 128);
  assert.equal(sourceApplyFixture.max_dry_run_edit_previews, 3);
  assert.equal(sourceApplyFixture.max_dry_run_replacement_text_bytes, 4096);
  assert.equal(sourceApplyFixture.max_preview_kind_bytes, 64);
  assert.equal(sourceApplyFixture.max_preview_anatomy_part_bytes, 64);
  assert.equal(sourceApplyFixture.max_preview_anatomy_parts, 8);
  assert.match(groupWebPreviewContext, /GROUPED_CLASS_WEB_PREVIEW_CONTEXT_SCHEMA/);
  assert.match(groupWebPreviewContext, /dx\.style\.grouped-class-web-preview-context/);
  assert.match(groupWebPreviewContext, /GROUPED_CLASS_WEB_PREVIEW_CONTEXT_ACTIVE_CONTEXT_SCHEMA/);
  assert.match(groupWebPreviewContext, /zed\.dx_style\.active_context\.v1/);
  assert.match(groupWebPreviewContext, /source_mutation_enabled: false/);
  assert.match(groupWebPreviewContext, /alias\(\)/);
  assert.match(groupWebPreviewContext, /alias\(atomic utilities\)/);
  assert.match(groupWebPreviewContext, /static atomic utility list/);
  assert.match(groupWebPreviewContext, /GROUPED_CLASS_WEB_PREVIEW_MAX_ALIAS_BYTES: usize = 128/);
  assert.match(groupWebPreviewContext, /GROUPED_CLASS_WEB_PREVIEW_MAX_UTILITY_COUNT: usize = 32/);
  assert.match(groupWebPreviewContext, /GROUPED_CLASS_WEB_PREVIEW_MAX_UTILITY_BYTES: usize = 256/);
  assert.match(
    groupWebPreviewContext,
    /GROUPED_CLASS_WEB_PREVIEW_CANDIDATE_MIN_UTILITY_COUNT: usize = 4/,
  );
  assert.equal(
    groupWebPreviewContextFixture.schema,
    "dx.style.grouped-class-web-preview-context",
  );
  assert.equal(groupWebPreviewContextFixture.active_context_schema, "zed.dx_style.active_context.v1");
  assert.equal(groupWebPreviewContextFixture.source_mutation_enabled, false);
  assert.equal(groupWebPreviewContextFixture.max_alias_bytes, 128);
  assert.equal(groupWebPreviewContextFixture.max_utility_count, 32);
  assert.equal(groupWebPreviewContextFixture.max_utility_bytes, 256);
  assert.equal(groupWebPreviewContextFixture.candidate_min_utility_count, 4);
  assert.ok(groupWebPreviewContextFixture.supported_token_shapes.includes("alias()"));
  assert.ok(groupWebPreviewContextFixture.context_fields.includes("group_context.utilities"));
  assert.ok(
    groupWebPreviewContextFixture.context_fields.includes("group_context.registry_receipt"),
  );
  assert.match(groupRegistryReceipt, /GROUPED_CLASS_REGISTRY_RECEIPT_SCHEMA/);
  assert.match(groupRegistryReceipt, /dx\.style\.grouped-class-registry-receipt/);
  assert.match(groupRegistryReceipt, /GROUPED_CLASS_REGISTRY_RECEIPT_FIXTURE_PATH/);
  assert.match(groupRegistryReceipt, /GROUPED_CLASS_REGISTRY_RECEIPT_DEFAULT_PATH/);
  assert.match(groupRegistryReceipt, /GROUPED_CLASS_SOURCE_DIGEST_ALGORITHM/);
  assert.match(groupRegistryReceipt, /grouped_class_registry_receipt/);
  assert.match(groupRegistryReceipt, /write_grouped_class_registry_receipt/);
  assert.match(groupRegistryReceipt, /fs::create_dir_all/);
  assert.match(groupRegistryReceipt, /fs::rename/);
  assert.match(groupRegistryReceipt, /registry_entries_verified: true/);
  assert.match(groupRegistryReceipt, /source_owned: true/);
  assert.match(coreMod, /write_group_registry_receipt_snapshot/);
  assert.match(coreMod, /grouped_class_source_digest\(html_bytes\)/);
  assert.match(coreMod, /group_registry_receipt_path/);
  assert.match(coreMod, /css_state_unchanged/);
  assert.match(coreMod, /drop\(state_guard\);[\s\S]*write_group_registry_receipt_snapshot/);
  assert.equal(
    groupRegistryReceiptFixture.schema,
    "dx.style.grouped-class-registry-receipt",
  );
  assert.equal(groupRegistryReceiptFixture.source_digest_algorithm, "fnv1a64");
  assert.equal(groupRegistryReceiptFixture.entry_count, 1);
  assert.equal(groupRegistryReceiptFixture.trust.registry_entries_verified, true);
  assert.equal(groupRegistryReceiptFixture.trust.source_digest_verified, true);
  assert.equal(groupRegistryReceiptFixture.trust.source_owned, true);
  assert.equal(groupRegistryReceiptFixture.entries[0].alias, "button");
  assert.ok(groupRegistryReceiptFixture.entries[0].utilities.includes("bg-primary"));
  assert.match(groupReverseCssMap, /GROUPED_CLASS_REVERSE_CSS_MAP_SCHEMA/);
  assert.match(groupReverseCssMap, /dx\.style\.grouped-class-reverse-css-map/);
  assert.match(groupReverseCssMap, /GROUPED_CLASS_REVERSE_CSS_MAP_DEFAULT_PATH/);
  assert.match(groupReverseCssMap, /GroupedClassReverseCssStatus/);
  assert.match(groupReverseCssMap, /ReadyForReview/);
  assert.match(groupReverseCssMap, /source_mutation_enabled: false/);
  assert.match(groupReverseCssMap, /editor_write_bridge_required: true/);
  assert.match(groupReverseCssMap, /write_grouped_class_reverse_css_map_receipt/);
  assert.match(groupReverseCssMap, /serialize_identifier/);
  assert.match(coreMod, /write_grouped_class_reverse_css_map_receipt/);
  assert.match(coreMod, /group_reverse_css_map_receipt_path/);
  assert.equal(
    groupReverseCssMapFixture.schema,
    "dx.style.grouped-class-reverse-css-map",
  );
  assert.equal(groupReverseCssMapFixture.trust.source_mutation_enabled, false);
  assert.equal(groupReverseCssMapFixture.trust.editor_write_bridge_required, true);
  assert.equal(groupReverseCssMapFixture.reviewable_entry_count, 1);
  assert.equal(groupReverseCssMapFixture.entries[0].selector, ".button");
  assert.equal(groupReverseCssMapFixture.entries[0].reverse_status, "ready_for_review");
  assert.ok(groupReverseCssMapFixture.entries[0].utilities.includes("bg-primary"));
  assert.match(groupReverseCssDelta, /GROUPED_CLASS_REVERSE_CSS_DELTA_SCHEMA/);
  assert.match(groupReverseCssDelta, /dx\.style\.grouped-class-reverse-css-delta-contract/);
  assert.match(groupReverseCssDelta, /GROUPED_CLASS_REVERSE_CSS_DELTA_SOURCE_MUTATION_ENABLED: bool = false/);
  assert.match(groupReverseCssDelta, /generated CSS declaration delta validation/);
  assert.match(groupReverseCssDelta, /reverse CSS map receipt match/);
  assert.match(groupReverseCssDelta, /reverse CSS delta preview provenance match/);
  assert.match(groupReverseCssDelta, /required_preview_provenance_fields/);
  assert.match(groupReverseCssDelta, /grouped_class_reverse_css_delta_preview/);
  assert.match(groupReverseCssDelta, /filter\(\|mapping\|[\s\S]{0,120}eq_ignore_ascii_case\(property\)/);
  assert.match(groupReverseCssDelta, /utility_matches_family/);
  assert.match(groupReverseCssDelta, /is_border_color_utility/);
  assert.match(groupReverseCssDelta, /GroupedClassReverseCssDeltaValueStrategy/);
  assert.match(groupReverseCssDelta, /ArbitraryBracketValue/);
  assert.match(groupReverseCssDelta, /MarginTokenSuffix/);
  assert.match(groupReverseCssDelta, /DisplayKeyword/);
  assert.match(groupReverseCssDelta, /DropShadowFunction/);
  assert.match(groupReverseCssDelta, /BackdropBlurFunction/);
  assert.match(groupReverseCssDelta, /AlignItemsKeyword/);
  assert.match(groupReverseCssDelta, /JustifyContentKeyword/);
  assert.match(groupReverseCssDelta, /AlignContentKeyword/);
  assert.match(groupReverseCssDelta, /GridTrackRepeatCount/);
  assert.match(groupReverseCssDelta, /TransitionPropertyValue/);
  assert.match(groupReverseCssDelta, /TransitionTimingFunctionValue/);
  assert.match(groupReverseCssDelta, /arbitrary_bracket_token/);
  assert.match(groupReverseCssDelta, /target_utility_from_token/);
  assert.match(groupReverseCssDelta, /is_background_image_utility/);
  assert.match(groupReverseCssDelta, /display_token_from_value/);
  assert.match(groupReverseCssDelta, /align_items_token_from_value/);
  assert.match(groupReverseCssDelta, /justify_content_token_from_value/);
  assert.match(groupReverseCssDelta, /align_content_token_from_value/);
  assert.match(groupReverseCssDelta, /grid_track_repeat_count_token/);
  assert.match(groupReverseCssDelta, /transition_property_token_from_value/);
  assert.match(groupReverseCssDelta, /transition_timing_function_token_from_value/);
  assert.match(groupReverseCssDelta, /is_display_utility/);
  assert.match(groupReverseCssDelta, /is_margin_utility/);
  assert.match(groupReverseCssDelta, /is_base_gap_utility/);
  assert.match(groupReverseCssDelta, /is_outline_color_utility/);
  assert.match(groupReverseCssDelta, /is_transition_property_utility/);
  assert.match(groupReverseCssDelta, /is_shadow_effect_utility/);
  assert.match(groupReverseCssDelta, /background-color/);
  assert.match(groupReverseCssDelta, /background-image/);
  assert.match(groupReverseCssDelta, /outline-color/);
  assert.match(groupReverseCssDelta, /accent-color/);
  assert.match(groupReverseCssDelta, /caret-color/);
  assert.match(groupReverseCssDelta, /display/);
  assert.match(groupReverseCssDelta, /margin-top/);
  assert.match(groupReverseCssDelta, /width/);
  assert.match(groupReverseCssDelta, /align-items/);
  assert.match(groupReverseCssDelta, /align-content/);
  assert.match(groupReverseCssDelta, /grid-template-columns/);
  assert.match(groupReverseCssDelta, /transition-property/);
  assert.match(groupReverseCssDelta, /transition-duration/);
  assert.match(groupReverseCssDelta, /transition-delay/);
  assert.match(groupReverseCssDelta, /transition-timing-function/);
  assert.match(groupReverseCssDelta, /clip-path/);
  assert.match(groupReverseCssDelta, /mask-image/);
  assert.match(groupReverseCssDelta, /box-shadow/);
  assert.match(groupReverseCssDelta, /backdrop-filter/);
  assert.match(groupReverseCssDelta, /padding-inline/);
  assert.match(groupReverseCssDelta, /calc\(var\(--spacing\) \* /);
  assert.match(groupReverseCssDelta, /var\(--color-/);
  assert.match(coreMod, /grouped_class_reverse_css_delta_contract/);
  assert.equal(
    groupReverseCssDeltaFixture.schema,
    "dx.style.grouped-class-reverse-css-delta-contract",
  );
  assert.equal(groupReverseCssDeltaFixture.source_mutation_enabled, false);
  assert.equal(groupReverseCssDeltaFixture.editor_write_bridge_required, true);
  assert.equal(groupReverseCssDeltaFixture.reverse_css_map_required, true);
  assert.ok(
    groupReverseCssDeltaFixture.required_editor_guards.includes(
      "reverse CSS delta preview provenance match",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.required_preview_provenance_fields.includes(
      "reverse_css_map_status",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) => entry.property === "padding-inline" && entry.utility_prefix === "px-",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "background-image" &&
        entry.utility_prefix === "bg-" &&
        entry.value_strategy === "arbitrary_bracket_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) => entry.property === "gap" && entry.utility_prefix === "gap-",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "outline-color" &&
        entry.utility_prefix === "outline-" &&
        entry.value_strategy === "design_token_suffix",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "accent-color" &&
        entry.utility_prefix === "accent-" &&
        entry.value_strategy === "design_token_suffix",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "caret-color" &&
        entry.utility_prefix === "caret-" &&
        entry.value_strategy === "design_token_suffix",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "gap" &&
        entry.utility_prefix === "gap-" &&
        entry.value_strategy === "arbitrary_bracket_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "padding" &&
        entry.utility_prefix === "p-" &&
        entry.value_strategy === "arbitrary_bracket_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "border-radius" &&
        entry.utility_prefix === "rounded-" &&
        entry.value_strategy === "arbitrary_bracket_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "clip-path" &&
        entry.utility_prefix === "clip-path-" &&
        entry.value_strategy === "arbitrary_bracket_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "mask-image" &&
        entry.utility_prefix === "mask-image-" &&
        entry.value_strategy === "arbitrary_bracket_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "display" &&
        entry.utility_prefix === "" &&
        entry.value_strategy === "display_keyword",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "margin-top" &&
        entry.utility_prefix === "mt-" &&
        entry.value_strategy === "margin_token_suffix",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "width" &&
        entry.utility_prefix === "w-" &&
        entry.value_strategy === "design_token_suffix",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "transition-property" &&
        entry.utility_prefix === "transition-" &&
        entry.value_strategy === "transition_property_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "transition-duration" &&
        entry.utility_prefix === "duration-" &&
        entry.value_strategy === "arbitrary_bracket_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "transition-timing-function" &&
        entry.utility_prefix === "ease-" &&
        entry.value_strategy === "transition_timing_function_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "align-items" &&
        entry.utility_prefix === "items-" &&
        entry.value_strategy === "align_items_keyword",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "justify-content" &&
        entry.utility_prefix === "justify-" &&
        entry.value_strategy === "justify_content_keyword",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "align-content" &&
        entry.utility_prefix === "content-" &&
        entry.value_strategy === "align_content_keyword",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "grid-template-columns" &&
        entry.utility_prefix === "grid-cols-" &&
        entry.value_strategy === "grid_track_repeat_count",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "box-shadow" &&
        entry.utility_prefix === "shadow-" &&
        entry.value_strategy === "arbitrary_bracket_value",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "filter" &&
        entry.utility_prefix === "drop-shadow-" &&
        entry.value_strategy === "drop_shadow_function",
    ),
  );
  assert.ok(
    groupReverseCssDeltaFixture.supported_properties.some(
      (entry) =>
        entry.property === "backdrop-filter" &&
        entry.utility_prefix === "backdrop-blur-" &&
        entry.value_strategy === "backdrop_blur_function",
    ),
  );
  assert.equal(groupReverseCssDeltaFixture.example_preview.status, "ready_for_review");
  assert.equal(groupReverseCssDeltaFixture.example_preview.target_utility, "bg-secondary");
  assert.ok(
    groupReverseCssDeltaFixture.example_preview.replacement_utilities.includes(
      "bg-secondary",
    ),
  );
  assert.match(generatorCatalog, /VISUAL_GENERATOR_CATALOG_SCHEMA/);
  assert.match(generatorCatalog, /VISUAL_GENERATOR_CATALOG_LIMIT: usize = 25/);
  assert.match(generatorCatalog, /VISUAL_GENERATOR_CATALOG_FIXTURE_PATH/);
  assert.match(generatorCatalog, /VisualGeneratorSourceEditSafety/);
  assert.equal(generatorCatalogFixture.schema, "dx.style.visual-generator-catalog");
  assert.equal(generatorCatalogFixture.generator_count, 25);
  assert.equal(generatorCatalogFixture.entries.length, 25);
  assert.deepEqual(
    generatorCatalogFixture.entries,
    collectVisualGeneratorEntries(generatorCatalog),
  );
  assert.match(generatorRecipeCatalog, /VISUAL_GENERATOR_RECIPE_CATALOG_SCHEMA/);
  assert.match(generatorRecipeCatalog, /VISUAL_GENERATOR_RECIPE_CATALOG_FIXTURE_PATH/);
  assert.match(generatorRecipeCatalog, /VISUAL_GENERATOR_RECIPE_RUNTIME_VALUE_KEYS/);
  assert.match(generatorRecipeCatalog, /VISUAL_GENERATOR_PREVIEW_ANATOMY_PARTS/);
  assert.match(generatorRecipeCatalog, /VisualGeneratorPreviewKind/);
  assert.match(generatorRecipeCatalog, /VisualGeneratorPreviewPart/);
  assert.match(generatorRecipeCatalog, /preview_kind: VisualGeneratorPreviewKind/);
  assert.match(generatorRecipeCatalog, /preview_anatomy: &'static \[VisualGeneratorPreviewPart\]/);
  assert.match(generatorRecipeCatalog, /VisualGeneratorRecipeCatalogEntry/);
  assert.match(generatorRecipeCatalog, /visual_generator_recipe_catalog_json/);
  assert.match(generatorRecipeCatalog, /grid-layout-editor/);
  assert.match(generatorRecipeCatalog, /keyframe-animation-timeline/);
  assert.match(generatorControlCatalog, /VISUAL_GENERATOR_CONTROL_CATALOG_SCHEMA/);
  assert.match(generatorControlCatalog, /VISUAL_GENERATOR_CONTROL_CATALOG_FIXTURE_PATH/);
  assert.match(generatorControlCatalog, /VisualGeneratorControlCatalogEntry/);
  assert.match(generatorControlCatalog, /visual_generator_control_catalog_json/);
  assert.match(generatorControlCatalog, /GRADIENT_CONTROLS/);
  assert.match(generatorControlCatalog, /RESPONSIVE_CONTROLS/);
  assert.equal(generatorRecipeFixture.schema, "dx.style.visual-generator-recipe-catalog");
  assert.equal(generatorRecipeFixture.entry_count, 25);
  assert.equal(generatorRecipeFixture.entries.length, 25);
  assert.ok(generatorRecipeFixture.runtime_value_keys.includes("css_linear"));
  assert.ok(generatorRecipeFixture.runtime_value_keys.includes("glass_blur"));
  assert.ok(generatorRecipeFixture.preview_anatomy_parts.includes("timeline-track"));
  assert.ok(generatorRecipeFixture.preview_anatomy_parts.includes("layout-items"));
  assert.ok(
    generatorRecipeFixture.entries.every((entry) => typeof entry.preview_kind === "string"),
  );
  const generatorRecipePreviewParts = new Set(generatorRecipeFixture.preview_anatomy_parts);
  assert.ok(
    generatorRecipeFixture.entries.every(
      (entry) =>
        Array.isArray(entry.preview_anatomy) &&
        entry.preview_anatomy.every(
          (part) => typeof part === "string" && generatorRecipePreviewParts.has(part),
        ),
    ),
  );
  assert.ok(
    generatorRecipeFixture.entries.some((entry) =>
      entry.preview_anatomy.includes("timeline-track"),
    ),
  );
  assert.ok(
    generatorRecipeFixture.entries.some((entry) =>
      entry.preview_anatomy.includes("layout-items"),
    ),
  );
  const generatorRecipeValueKeys = new Set(generatorRecipeFixture.runtime_value_keys);
  for (const entry of generatorRecipeFixture.entries) {
    const placeholders = [
      ...templatePlaceholders(entry.class_template),
      ...templatePlaceholders(entry.css_template),
    ];
    assert.deepEqual(
      placeholders.filter((key) => !generatorRecipeValueKeys.has(key)),
      [],
      `${entry.generator_id} should only use source-owned recipe value keys`,
    );
  }
  assert.equal(generatorControlFixture.schema, "dx.style.visual-generator-control-catalog");
  assert.equal(generatorControlFixture.entry_count, 25);
  assert.equal(generatorControlFixture.entries.length, 25);
  assert.deepEqual(
    generatorControlFixture.entries.map((entry) => ({
      id: entry.generator_id,
      controls: entry.controls,
    })),
    collectControlEntries(generatorControlCatalog),
  );
  assert.equal(
    generatorControlFixture.entries.find(
      (entry) => entry.generator_id === "grid-layout-editor",
    ).controls.some((control) => control.key === "columns"),
    true,
  );
  assert.match(generatorCssHintCatalog, /VISUAL_GENERATOR_CSS_HINT_CATALOG_SCHEMA/);
  assert.match(
    generatorCssHintCatalog,
    /dx\.style\.visual-generator-css-declaration-hint-catalog/,
  );
  assert.match(generatorCssHintCatalog, /VisualGeneratorCssDeclarationHintEntry/);
  assert.match(generatorCssHintCatalog, /visual_generator_css_hint_catalog_json/);
  assert.match(generatorCssHintCatalog, /property_match/);
  assert.match(generatorCssHintCatalog, /value_contains/);
  assert.match(generatorCssHintCatalog, /token_hint/);
  assert.match(generatorCssHintCatalog, /generator_id/);
  assert.equal(
    generatorCssHintFixture.schema,
    "dx.style.visual-generator-css-declaration-hint-catalog",
  );
  assert.equal(generatorCssHintFixture.entry_count, 38);
  assert.equal(generatorCssHintFixture.entries.length, 38);
  assert.deepEqual(generatorCssHintFixture.entries, collectCssHintEntries(generatorCssHintCatalog));
  assert.equal(
    generatorCssHintFixture.entries.find(
      (entry) => entry.property_pattern === "outline-color",
    ).token_hint,
    "outline-*",
  );
  const easingGenerator = generatorCatalogFixture.entries.find(
    (entry) => entry.generator_id === "easing-cubic-bezier-editor",
  );
  assert.ok(
    easingGenerator.applicable_class_families.includes(
      "[transition-timing-function:...]",
    ),
  );
  assert.equal(
    easingGenerator.applicable_class_families.includes(
      "[animation-timing-function:...]",
    ),
    false,
  );
  const easingCssHint = generatorCssHintFixture.entries.find(
    (entry) => entry.generator_id === "easing-cubic-bezier-editor",
  );
  assert.equal(easingCssHint.property_pattern, "transition-timing-function");
  assert.equal(easingCssHint.token_hint, "ease-*");
  assert.match(engineMod, /pub mod css_declaration_dry_run/);
  assert.match(cssDeclarationDryRunContract, /CSS_DECLARATION_DRY_RUN_CONTRACT_SCHEMA/);
  assert.match(cssDeclarationDryRunContract, /dx\.style\.css-declaration-dry-run-contract/);
  assert.match(cssDeclarationDryRunContract, /CSS_DECLARATION_DRY_RUN_CONTEXT_KIND/);
  assert.match(cssDeclarationDryRunContract, /source_mutation_enabled: false/);
  assert.match(cssDeclarationDryRunContract, /trusted CSS declaration dry-run receipt/);
  assert.match(cssDeclarationDryRunContract, /css_declaration_dry_run_preview/);
  assert.match(cssDeclarationDryRunContract, /css_declaration_dry_run_diagnostics/);
  assert.match(cssDeclarationDryRunContract, /css_declaration_dry_run_preview_diagnostics/);
  assert.match(cssDeclarationDryRunContract, /accepted_source_edit_safety/);
  assert.equal(
    cssDeclarationDryRunFixture.schema,
    "dx.style.css-declaration-dry-run-contract",
  );
  assert.equal(cssDeclarationDryRunFixture.review_context_kind, "css_declaration");
  assert.equal(cssDeclarationDryRunFixture.source_mutation_enabled, false);
  assert.ok(
    cssDeclarationDryRunFixture.required_context_fields.includes(
      "css_source_edit_safety",
    ),
  );
  assert.ok(
    cssDeclarationDryRunFixture.accepted_source_edit_safety.includes(
      "safe-static-utility-source-edit",
    ),
  );
  assert.ok(
    cssDeclarationDryRunFixture.review_receipt_fields.includes(
      "css_declaration_dry_run_preview",
    ),
  );
  assert.ok(
    cssDeclarationDryRunFixture.review_receipt_fields.includes(
      "css_declaration_dry_run_diagnostics",
    ),
  );
  assert.ok(
    cssDeclarationDryRunFixture.review_receipt_fields.includes(
      "css_declaration_dry_run_preview_diagnostics",
    ),
  );
  assert.equal(cssDeclarationDryRunFixture.max_source_path_bytes, 4096);
  assert.equal(cssDeclarationDryRunFixture.max_source_span_bytes, 16384);
  assert.equal(cssDeclarationDryRunFixture.max_source_digest_bytes, 128);
  assert.equal(cssDeclarationDryRunFixture.max_declaration_bytes, 4096);
  assert.equal(cssDeclarationDryRunFixture.max_diagnostic_count, 8);
  assert.equal(cssDeclarationDryRunFixture.max_diagnostic_bytes, 160);
  assert.match(fixtureMirrorScript, /--write/);
  assert.match(fixtureMirrorScript, /--check/);
  assert.match(fixtureMirrorScript, /visual-generator-catalog\.generated\.json/);
  assert.match(fixtureMirrorScript, /visual-generator-control-catalog\.generated\.json/);
  assert.match(fixtureMirrorScript, /visual-generator-recipe-catalog\.generated\.json/);
  assert.match(fixtureMirrorScript, /source-apply-contract\.generated\.json/);
  assert.match(fixtureMirrorScript, /group-context-contract\.generated\.json/);
  assert.match(fixtureMirrorScript, /css-declaration-hint-catalog\.generated\.json/);
  assert.match(fixtureMirrorScript, /css-declaration-dry-run-contract\.generated\.json/);
  assert.match(fixtureMirrorScript, /grouped-class-dry-run-receipt-fixtures\.generated\.json/);
  assert.deepEqual(receiptFixturesGenerated, receiptFixtures);
  assert.equal(
    receiptFixtures.schema,
    "dx.style.grouped-class-dry-run-receipt-fixtures",
  );
  assert.equal(receiptFixtures.active_source.source_digest_algorithm, "fnv1a64");
  assert.equal(
    receiptFixtures.receipts.matching.source_digest,
    receiptFixtures.active_source.source_digest,
  );
  assert.equal(
    receiptFixtures.receipts.matching.source_file.path,
    receiptFixtures.active_source.path,
  );
  assert.equal(
    receiptFixtures.receipts.matching.patch_preview.edits[0].span.start.byte_offset,
    receiptFixtures.active_source.span.start.byte_offset,
  );
  assert.notEqual(
    receiptFixtures.receipts.path_mismatch.source_file.path,
    receiptFixtures.active_source.path,
  );
  assert.notEqual(
    receiptFixtures.receipts.span_mismatch.patch_preview.edits[0].span.end.byte_offset,
    receiptFixtures.active_source.span.end.byte_offset,
  );
  assert.notEqual(
    receiptFixtures.receipts.digest_mismatch.source_digest,
    receiptFixtures.active_source.source_digest,
  );
});

test("DX Style plan keeps the Zed panel source-backed and read-only first", () => {
  const plan = readStyle("PLAN.md");

  assert.match(plan, /### Phase 1: DX Style Source Truth/);
  assert.match(plan, /Emit a grouped-class two-way read model for editor consumers/);
  assert.match(plan, /Mark invertibility honestly/);
  assert.match(plan, /### Phase 2: Zed Read-Only Style Panel/);
  assert.match(plan, /Add a right-panel surface in Zed for DX Style/);
  assert.match(plan, /Detect current file\/cursor style context/);
  assert.match(plan, /Show class expansions, group metadata, generated CSS, and warnings/);
  assert.match(plan, /Do not apply writes yet unless a trusted receipt, active path\/span\/digest match/);
  assert.match(plan, /No fake two-way editing/);
  assert.match(plan, /No generated CSS as the only source of truth/);
  assert.match(plan, /No disconnected demo-only panel controls/);
  assert.match(plan, /No unsafe rewrites of dynamic `className=\{\.\.\.\}` expressions/);
  assert.match(plan, /No hiding non-invertible cases/);
  assert.match(plan, /source-owned dry-run receipt fixtures/);
  assert.match(plan, /source-owned editor write-bridge preflight contract/);
  assert.match(plan, /checked-in fixture/);
});

test("Zed handoff docs register the DX Style panel/read-model guard", () => {
  const dx = read("DX.md");

  assert.match(dx, /node --test script\/dx-style-panel-source\.test\.ts/);
  assert.match(
    dx,
    /DX Style plan\/read-model, Web Preview generator split modules, trusted source-apply session/,
  );
});

test("DX Style visual generator mirror helper reports Zed fallback freshness", () => {
  const output = execFileSync(
    process.execPath,
    [join(dxStyleRoot(), "scripts/sync_zed_visual_generator_fixtures.mjs"), "--check"],
    {
      cwd: process.cwd(),
      encoding: "utf8",
      env: {
        ...process.env,
        DX_ZED_ROOT: process.cwd(),
      },
    },
  );

  assert.match(output, /ok fixtures\/visual-generator-catalog\.json/);
  assert.match(output, /ok fixtures\/visual-generator-control-catalog\.json/);
  assert.match(output, /ok fixtures\/visual-generator-recipe-catalog\.json/);
  assert.match(output, /ok fixtures\/grouped-class-source-apply-contract\.json/);
  assert.match(output, /ok fixtures\/grouped-class-web-preview-context\.json/);
  assert.match(output, /ok fixtures\/grouped-class-reverse-css-delta-contract\.json/);
  assert.match(output, /ok fixtures\/visual-generator-css-declaration-hint-catalog\.json/);
  assert.match(output, /ok fixtures\/css-declaration-dry-run-contract\.json/);
  assert.match(output, /ok fixtures\/grouped-class-dry-run-receipt-fixtures\.json/);
});

test("Zed Style rail keeps GPUI as the shell and Web Preview as the generator host", () => {
  const snapshot = read("crates/agent_ui/src/dx_style_panel.rs");
  const rail = read("crates/agent_ui/src/dx_launch_workspace/style_panel.rs");
  const readiness = read("crates/agent_ui/src/dx_style_panel/readiness.rs");
  const readinessExpectedFiles = read(
    "crates/agent_ui/src/dx_style_panel/readiness/expected_files.rs",
  );

  assert.match(snapshot, /visual_generator_catalog\.rs/);
  assert.match(snapshot, /grouped_class_read_model\.rs/);
  assert.match(snapshot, /grouped_class_source_apply\.rs/);
  assert.match(readinessExpectedFiles, /css_declaration_dry_run\.rs/);
  assert.match(snapshot, /grouped_class_reverse_css_delta\.rs/);
  assert.match(snapshot, /mod active_context/);
  assert.match(snapshot, /mod group_context/);
  assert.match(snapshot, /Generator host: Web Preview owns visual controls/);
  assert.match(snapshot, /trusted dry-run receipts, source identity, and the editor write bridge/);
  assert.match(snapshot, /web_preview_bridge_ready/);
  assert.match(snapshot, /MAX_WEB_PREVIEW_HOST_BYTES/);
  assert.match(snapshot, /read_text_limited_to/);
  assert.match(snapshot, /file_contains_all_markers_limited/);
  assert.match(snapshot, /contains_subslice/);
  assert.match(snapshot, /dx_style_generator_surface_path/);
  assert.match(snapshot, /dx_style_generator_script_path/);
  assert.match(snapshot, /dx_style_css_declaration_dry_run_script_path/);
  assert.match(snapshot, /dx_style_source_apply_session_script_path/);
  assert.match(snapshot, /dx_style_source_apply_path/);
  assert.match(snapshot, /OpenGeneratorPreviewForContext/);
  assert.match(snapshot, /dx-style-source-apply/);
  assert.match(snapshot, /DX_STYLE_GENERATOR_SURFACE_SCHEMA/);
  assert.match(snapshot, /dx_style_generator_url_with_context_and_source_apply_session/);
  assert.match(snapshot, /sourceApplySessionToken/);
  assert.match(snapshot, /DX_STYLE_SOURCE_APPLY_SESSION_KIND/);
  assert.match(snapshot, /GROUPED_CLASS_SOURCE_APPLY_CONTRACT_VERSION/);
  assert.match(snapshot, /GROUPED_CLASS_SOURCE_APPLY_SCOPE/);
  assert.match(snapshot, /cssDeclarationDryRunPreview/);
  assert.match(snapshot, /css_declaration_dry_run_contract_missing/);
  assert.match(snapshot, /source_apply_review_receipt/);
  assert.match(snapshot, /source_apply_contract_ready/);
  assert.match(snapshot, /Source Apply/);
  assert.match(snapshot, /contract review-only/);
  assert.match(snapshot, /Reverse CSS Delta/);
  assert.match(snapshot, /web preview review contract/);
  assert.match(rail, /Web Preview Host/);
  assert.match(rail, /Web Preview ready/);
  assert.match(snapshot, /Visual CSS generators render in Web Preview, not hand-built GPUI controls/);
  assert.match(readiness, /mod expected_files/);
  assert.match(readinessExpectedFiles, /Grouped class editor read model/);
  assert.match(readinessExpectedFiles, /Grouped class cursor context/);
  assert.match(readinessExpectedFiles, /Grouped class dry-run receipt/);
  assert.match(readinessExpectedFiles, /Grouped class editor write bridge/);
  assert.match(readinessExpectedFiles, /Grouped class source apply contract/);
  assert.match(readinessExpectedFiles, /Grouped class Web Preview context/);
  assert.match(readinessExpectedFiles, /Grouped class reverse CSS delta/);
  assert.match(readinessExpectedFiles, /Visual generator catalog/);
  assert.match(readinessExpectedFiles, /Visual generator recipe catalog/);
  assert.match(readinessExpectedFiles, /Visual generator control catalog/);
  assert.match(readinessExpectedFiles, /Visual generator CSS hint catalog/);
  assert.match(readinessExpectedFiles, /CSS declaration dry-run contract/);
  assert.match(readinessExpectedFiles, /Visual generator CSS hint fixture/);
  assert.match(readinessExpectedFiles, /CSS declaration dry-run fixture/);
  assert.match(readinessExpectedFiles, /Grouped class reverse CSS delta fixture/);
});

test("Web Preview owns the DX Style generator surface action", () => {
  const actions = read("crates/zed_actions/src/lib.rs");
  const webPreview = read("crates/web_preview/src/web_preview.rs");
  const webPreviewView = read("crates/web_preview/src/web_preview_view.rs");
  const surface = read("crates/web_preview/src/dx_style_generator_surface.rs");
  const surfaceCatalog = read(
    "crates/web_preview/src/dx_style_generator_surface/catalog.rs",
  );
  const surfaceFixture = read(
    "crates/web_preview/src/dx_style_generator_surface/fixture.rs",
  );
  const surfaceStyle = read(
    "crates/web_preview/src/dx_style_generator_surface/style.rs",
  );
  const surfaceScript = read(
    "crates/web_preview/src/dx_style_generator_surface/script.rs",
  );
  const surfaceSourceApplySessionScript = read(
    "crates/web_preview/src/dx_style_generator_surface/source_apply_session_script.rs",
  );
  const surfaceRecipes = read(
    "crates/web_preview/src/dx_style_generator_surface/recipes.rs",
  );
  const surfaceControls = read(
    "crates/web_preview/src/dx_style_generator_surface/controls.rs",
  );
  const sourceApply = read("crates/web_preview/src/dx_style_source_apply.rs");
  const surfaceSourceApplyContract = read(
    "crates/web_preview/src/dx_style_generator_surface/source_apply_contract.rs",
  );
  const surfaceCssDeclarationDryRunContract = read(
    "crates/web_preview/src/dx_style_generator_surface/css_declaration_dry_run_contract.rs",
  );
  const surfaceCssDeclarationDryRunScript = read(
    "crates/web_preview/src/dx_style_generator_surface/css_declaration_dry_run_script.rs",
  );
  const surfaceGroupContextContract = read(
    "crates/web_preview/src/dx_style_generator_surface/group_context_contract.rs",
  );
  const surfaceReverseCssDeltaContract = read(
    "crates/web_preview/src/dx_style_generator_surface/reverse_css_delta_contract.rs",
  );
  const surfaceGeneratedRecipes = JSON.parse(
    read(
      "crates/web_preview/src/dx_style_generator_surface/visual-generator-recipe-catalog.generated.json",
    ),
  );
  const surfaceGeneratedCatalog = JSON.parse(
    read(
      "crates/web_preview/src/dx_style_generator_surface/visual-generator-catalog.generated.json",
    ),
  );
  const surfaceGeneratedControls = JSON.parse(
    read(
      "crates/web_preview/src/dx_style_generator_surface/visual-generator-control-catalog.generated.json",
    ),
  );
  const surfaceGeneratedSourceApplyContract = JSON.parse(
    read(
      "crates/web_preview/src/dx_style_generator_surface/source-apply-contract.generated.json",
    ),
  );
  const surfaceGeneratedCssDeclarationDryRunContract = JSON.parse(
    read(
      "crates/web_preview/src/dx_style_generator_surface/css-declaration-dry-run-contract.generated.json",
    ),
  );
  const surfaceGeneratedGroupContextContract = JSON.parse(
    read(
      "crates/web_preview/src/dx_style_generator_surface/group-context-contract.generated.json",
    ),
  );
  const surfaceGeneratedReverseCssDeltaContract = JSON.parse(
    read(
      "crates/web_preview/src/dx_style_generator_surface/reverse-css-delta-contract.generated.json",
    ),
  );
  const styleRecipeCatalog = readStyle(
    "src/core/engine/visual_generator_recipe_catalog.rs",
  );
  const styleCatalogFixture = JSON.parse(
    readStyle("fixtures/visual-generator-catalog.json"),
  );
  const styleRecipeFixture = JSON.parse(
    readStyle("fixtures/visual-generator-recipe-catalog.json"),
  );
  const styleControlFixture = JSON.parse(
    readStyle("fixtures/visual-generator-control-catalog.json"),
  );
  const styleSourceApplyFixture = JSON.parse(
    readStyle("fixtures/grouped-class-source-apply-contract.json"),
  );
  const styleCssDeclarationDryRunFixture = JSON.parse(
    readStyle("fixtures/css-declaration-dry-run-contract.json"),
  );
  const styleGroupContextFixture = JSON.parse(
    readStyle("fixtures/grouped-class-web-preview-context.json"),
  );
  const styleReverseCssDeltaFixture = JSON.parse(
    readStyle("fixtures/grouped-class-reverse-css-delta-contract.json"),
  );
  const rail = read("crates/agent_ui/src/dx_launch_workspace/style_panel.rs");
  const styleRecipes = collectRecipeTriples(styleRecipeCatalog, "generator_id");
  const fixtureRecipes = styleRecipeFixture.entries.map((entry) => ({
    id: entry.generator_id,
    classTemplate: entry.class_template,
    cssTemplate: entry.css_template,
  }));
  const generatedRecipes = surfaceGeneratedRecipes.entries.map((entry) => ({
    id: entry.generator_id,
    classTemplate: entry.class_template,
    cssTemplate: entry.css_template,
  }));
  const webPreviewCargo = read("crates/web_preview/Cargo.toml");
  const sourceApplyArm = webPreviewView.slice(
    webPreviewView.indexOf('"dx-style-source-apply" =>'),
    webPreviewView.indexOf('let status = receipt', webPreviewView.indexOf('"dx-style-source-apply" =>')),
  );
  const sourceApplySessionTokenFn = webPreviewView.slice(
    webPreviewView.indexOf("fn next_dx_style_source_apply_session_token"),
    webPreviewView.indexOf("#[allow(dead_code)]", webPreviewView.indexOf("fn next_dx_style_source_apply_session_token")),
  );

  assert.match(actions, /pub mod dx_style/);
  assert.match(actions, /TogglePanel/);
  assert.match(actions, /OpenGeneratorPreview/);
  assert.match(actions, /OpenGeneratorPreviewForContext/);
  assert.match(actions, /source_context_json/);
  assert.match(webPreview, /mod dx_style_generator_surface/);
  assert.match(webPreview, /mod dx_style_source_apply/);
  assert.match(webPreviewView, /zed_actions::dx_style::OpenGeneratorPreview/);
  assert.match(webPreviewView, /OpenGeneratorPreviewForContext/);
  assert.match(webPreviewView, /open_dx_style_generator_in_side_pane/);
  assert.match(
    webPreviewView,
    /dx_style_generator_url_with_context_and_source_apply_session/,
  );
  assert.match(webPreviewView, /dx_style_source_apply_session_refusal/);
  assert.match(webPreviewView, /dx_style_source_apply_session_token/);
  assert.match(webPreviewView, /next_dx_style_source_apply_session_token/);
  assert.match(webPreviewCargo, /uuid\.workspace = true/);
  assert.match(webPreviewView, /use uuid::Uuid/);
  assert.match(sourceApplySessionTokenFn, /Uuid::new_v4\(\)/);
  assert.doesNotMatch(sourceApplySessionTokenFn, /current_epoch_millis/);
  assert.match(webPreviewView, /DX_STYLE_GENERATOR_DISPLAY_URL: &str = "zed:\/\/dx-style\/generator"/);
  assert.match(webPreviewView, /DX_STYLE_GENERATOR_DATA_URL_PREFIX: &str = "data:text\/html;charset=utf-8,"/);
  assert.match(webPreviewView, /fn display_url_for_loaded_url/);
  assert.match(webPreviewView, /fn is_dx_style_generator_data_url/);
  assert.match(webPreviewView, /DX%20Style%20Generators/);
  assert.match(webPreviewView, /fn is_dx_style_generator_display_url/);
  assert.match(webPreviewView, /editor\.set_text\(display_url, window, cx\)/);
  assert.match(
    webPreviewView,
    /display_url_for_loaded_url\(url\.as_str\(\), source_apply_session_active\)/,
  );
  assert.match(webPreviewView, /\.pointer\("\/source_apply_session\/kind"\)/);
  assert.match(webPreviewView, /\.pointer\("\/request\/source_apply_session\/kind"\)/);
  assert.match(webPreviewView, /\.pointer\("\/source_apply_session\/token"\)/);
  assert.match(webPreviewView, /\.pointer\("\/request\/source_apply_session\/token"\)/);
  assert.ok(
    webPreviewView.indexOf("let session_token = self.next_dx_style_source_apply_session_token()") <
      webPreviewView.indexOf("let url = dx_style_generator_url_with_context_and_source_apply_session"),
  );
  assert.ok(
    webPreviewView.indexOf("let url = dx_style_generator_url_with_context_and_source_apply_session") <
      webPreviewView.indexOf(
        "self.load_requested_url_with_source_apply_session(",
        webPreviewView.indexOf("let url = dx_style_generator_url_with_context_and_source_apply_session"),
      ),
  );
  assert.match(
    webPreviewView,
    /self\.load_requested_url_with_source_apply_session\(\s*&url,\s*Some\(session_token\),\s*session_source_identity,\s*window,\s*cx,\s*\)/,
  );
  assert.match(
    webPreviewView,
    /self\.dx_style_source_apply_session_token =\s*dx_style_source_apply_session_token\.map\(SharedString::from\)/,
  );
  assert.match(webPreviewView, /self\.dx_style_source_apply_session_token = None/);
  assert.match(
    webPreviewView,
    /DX Style source apply session token does not match the active trusted Web Preview session/,
  );
  assert.match(webPreviewView, /latest_dx_style_source_apply_receipt/);
  assert.match(webPreviewView, /latest_dx_style_source_apply_receipt_summary/);
  assert.match(webPreviewView, /"reverse_css_delta_contract": receipt\.get\("reverse_css_delta_contract"\)\.cloned\(\)/);
  assert.match(webPreviewView, /"reverse_css_delta_preview": receipt\.get\("reverse_css_delta_preview"\)\.cloned\(\)/);
  assert.match(webPreviewView, /"dry_run_review": receipt\.get\("dry_run_review"\)\.cloned\(\)/);
  assert.match(webPreviewView, /"review_status": receipt\.get\("review_status"\)\.and_then\(Value::as_str\)/);
  assert.match(webPreviewView, /"mutation_ready": receipt\.get\("mutation_ready"\)\.and_then\(Value::as_bool\)/);
  assert.match(webPreviewView, /"context_kind": receipt\.pointer\("\/context\/context_kind"\)\.and_then\(Value::as_str\)/);
  assert.match(webPreviewView, /"css_source_edit_safety": receipt\.pointer\("\/context\/css_source_edit_safety"\)\.and_then\(Value::as_str\)/);
  assert.match(webPreviewView, /"preview_output": receipt\.get\("preview_output"\)\.cloned\(\)/);
  assert.match(webPreviewView, /"css_declaration_dry_run_contract": receipt\.get\("css_declaration_dry_run_contract"\)\.cloned\(\)/);
  assert.match(webPreviewView, /"css_declaration_dry_run_diagnostics": receipt\.get\("css_declaration_dry_run_diagnostics"\)\.cloned\(\)/);
  assert.match(webPreviewView, /"css_declaration_dry_run_preview": receipt\.get\("css_declaration_dry_run_preview"\)\.cloned\(\)/);
  assert.match(webPreviewView, /"css_declaration_dry_run_preview_diagnostics": receipt\.get\("css_declaration_dry_run_preview_diagnostics"\)\.cloned\(\)/);
  assert.match(webPreviewView, /"dx-style-source-apply"/);
  assert.match(webPreviewView, /source_apply_review_receipt/);
  assert.match(webPreviewView, /source_apply_session_refused_receipt/);
  assert.match(webPreviewView, /MAX_DX_STYLE_ACTIVE_EDITOR_REVALIDATION_SOURCE_BYTES: usize = 256 \* 1024/);
  assert.match(webPreviewView, /struct DxStyleSourceApplySessionSourceIdentity/);
  assert.match(webPreviewView, /struct DxStyleSourceApplySessionNativeEditorIdentity/);
  assert.match(webPreviewView, /dx_style_source_apply_session_source_identity: Option<DxStyleSourceApplySessionSourceIdentity>/);
  assert.match(webPreviewView, /fn dx_style_source_apply_session_source_identity_from_context_json/);
  assert.match(
    webPreviewView,
    /dx_style_source_apply_session_source_identity_from_context_json\(\s*source_context_json\.as_deref\(\),\s*\)/,
  );
  assert.match(
    webPreviewView,
    /let session_source_identity =[\s\S]*dx_style_source_apply_session_source_identity_from_context_json[\s\S]*let view = Self::open_or_create\(workspace, window, cx\)/,
  );
  assert.match(webPreviewView, /fn dx_style_session_source_identity_with_native_editor/);
  assert.match(webPreviewView, /workspace\.active_item\(cx\)/);
  assert.match(webPreviewView, /active_item\.item_id\(\)\.as_u64\(\)/);
  assert.match(webPreviewView, /active_item\.act_as::<Editor>\(cx\)/);
  assert.match(webPreviewView, /editor\.active_buffer\(cx\)\?/);
  assert.match(webPreviewView, /active_buffer\.read\(cx\)\.remote_id\(\)\.to_proto\(\)/);
  assert.match(webPreviewView, /"native_editor": self\.native_editor\.as_ref\(\)\.map/);
  assert.match(webPreviewView, /DX_STYLE_SOURCE_APPLY_ACTIVE_CONTEXT_SCHEMA/);
  assert.match(webPreviewView, /DX_STYLE_SOURCE_DIGEST_PREFIX/);
  assert.match(webPreviewView, /fn dx_style_required_bounded_session_source_string/);
  assert.match(webPreviewView, /fn dx_style_optional_bounded_session_source_string/);
  assert.match(webPreviewView, /fn dx_style_is_complete_source_digest/);
  assert.match(webPreviewView, /fn dx_style_source_path_is_under_workspace_root/);
  assert.match(webPreviewView, /source_span_start > source_span_end/);
  assert.match(webPreviewView, /source_span_end > source_len_bytes/);
  assert.match(webPreviewView, /self\.dx_style_source_apply_session_source_identity = None/);
  assert.match(webPreviewView, /fn dx_style_payload_with_active_editor_source_revalidation/);
  assert.match(webPreviewView, /fn dx_style_active_editor_source_revalidation/);
  assert.match(webPreviewView, /session_source_identity_missing/);
  assert.match(webPreviewView, /request_source_length_missing/);
  assert.match(webPreviewView, /session_source_path_mismatch/);
  assert.match(webPreviewView, /session_source_span_mismatch/);
  assert.match(webPreviewView, /session_source_length_mismatch/);
  assert.match(webPreviewView, /session_source_digest_mismatch/);
  assert.match(webPreviewView, /session_native_editor_identity_missing/);
  assert.match(webPreviewView, /native_editor_identity_mismatch/);
  assert.match(webPreviewView, /session_native_editor\.active_buffer_remote_id/);
  assert.match(webPreviewView, /"session_source": session_source_identity\.to_json\(\)/);
  assert.match(webPreviewView, /workspace\.items_of_type::<Editor>\(cx\)/);
  assert.match(webPreviewView, /editor\.active_project_path\(cx\)/);
  assert.match(webPreviewView, /editor\.buffer\(\)\.read\(cx\)\.len\(cx\)\.0/);
  assert.match(webPreviewView, /crate::dx_style_source_apply::active_source_digest\(&source\)/);
  assert.match(webPreviewView, /DxStyleActiveEditorSourceRevalidationEvidence/);
  assert.match(webPreviewView, /request_source_digest/);
  assert.match(webPreviewView, /editor_source_digest/);
  assert.match(webPreviewView, /fn dx_style_source_paths_match/);
  assert.match(webPreviewView, /native_active_editor_source_revalidation/);
  assert.ok(
    webPreviewView.indexOf("self.dx_style_source_apply_session_refusal(&payload)") <
      webPreviewView.indexOf("crate::dx_style_source_apply::source_apply_review_receipt(&payload)"),
  );
  assert.ok(
    sourceApplyArm.indexOf("self.dx_style_payload_with_active_editor_source_revalidation(&payload, cx)") <
      sourceApplyArm.indexOf("crate::dx_style_source_apply::source_apply_review_receipt(&payload)"),
  );
  assert.match(sourceApplyArm, /source_apply_session_refused_receipt/);
  assert.match(sourceApplyArm, /source_apply_review_receipt/);
  assert.match(
    sourceApplyArm,
    /source_apply_session_refused_receipt[\s\S]*false/,
  );
  assert.match(sourceApplyArm, /source_apply_review_receipt[\s\S]*true/);
  assert.match(sourceApplyArm, /if consume_session_token/);
  assert.match(sourceApplyArm, /self\.dx_style_source_apply_session_token = None/);
  assert.match(sourceApplyArm, /self\.dx_style_source_apply_session_source_identity = None/);
  assert.ok(
    sourceApplyArm.indexOf("crate::dx_style_source_apply::source_apply_session_refused_receipt") <
      sourceApplyArm.indexOf("self.dx_style_source_apply_session_token = None"),
  );
  assert.ok(
    sourceApplyArm.indexOf("crate::dx_style_source_apply::source_apply_review_receipt") <
      sourceApplyArm.indexOf("self.dx_style_source_apply_session_token = None"),
  );
  assert.match(webPreviewView, /MAX_DEFERRED_WEB_PREVIEW_IPC_BYTES: usize = 8 \* 1024 \* 1024/);
  assert.match(webPreviewView, /ensure_deferred_ipc_queue_has_byte_capacity/);
  assert.match(webPreviewView, /deferred_ipc_message_bytes\(&self\.deferred_ipc_messages\)/);
  assert.match(webPreviewView, /queued_browser_ipc_message_bytes\(&queue\)/);
  assert.match(webPreviewView, /fn queued_browser_ipc_message_bytes/);
  assert.doesNotMatch(
    webPreviewView,
    /let dx_style_source_apply_session_token = self\.dx_style_source_apply_session_token\.clone\(\)/,
  );
  assert.match(webPreviewView, /dx_style_source_apply_session_token: None/);
  assert.match(webPreviewView, /DX Style source apply review recorded/);
  assert.match(webPreviewView, /DX Style source apply request refused/);
  assert.match(webPreviewView, /open_url_in_side_pane/);
  assert.match(sourceApply, /DX_STYLE_SOURCE_APPLY_RECEIPT_SCHEMA/);
  assert.match(sourceApply, /zed\.web_preview\.dx_style_source_apply_receipt\.v1/);
  assert.match(sourceApply, /DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA/);
  assert.match(sourceApply, /dx\.style\.grouped-class-source-apply-contract/);
  assert.match(sourceApply, /DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_SCHEMA/);
  assert.match(sourceApply, /dx\.style\.grouped-class-reverse-css-delta-contract/);
  assert.match(sourceApply, /DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_SCHEMA/);
  assert.match(sourceApply, /dx\.style\.css-declaration-dry-run-contract/);
  assert.match(sourceApply, /DX_STYLE_APPLY_KIND: &str = "dx-style-source-apply"/);
  assert.match(sourceApply, /DX_STYLE_SOURCE_APPLY_SESSION_KIND/);
  assert.match(
    sourceApply,
    /zed\.web_preview\.dx_style\.source_apply_session/,
  );
  assert.match(sourceApply, /DX_STYLE_ACTIVE_EDITOR_SOURCE_REVALIDATION_SCHEMA/);
  assert.match(
    sourceApply,
    /zed\.web_preview\.dx_style\.active_editor_source_revalidation"/,
  );
  assert.doesNotMatch(
    sourceApply,
    /zed\.web_preview\.dx_style\.active_editor_source_revalidation\.v1/,
  );
  assert.match(sourceApply, /MAX_DX_STYLE_SOURCE_APPLY_SESSION_TOKEN_BYTES: usize = 256/);
  assert.match(sourceApply, /pub\(crate\) fn active_source_digest/);
  assert.match(sourceApply, /source_apply_session_refused_receipt/);
  assert.match(sourceApply, /source_apply_session_refused/);
  assert.match(sourceApply, /not_performed_by_untrusted_session/);
  assert.match(sourceApply, /source-apply contract is missing trusted session kind/);
  assert.match(sourceApply, /source-apply contract is missing trusted session guard/);
  assert.match(sourceApply, /source-apply contract is missing source-apply session receipt field/);
  assert.match(sourceApply, /source-apply contract is missing session-bound source identity guard/);
  assert.match(sourceApply, /max_source_apply_session_token_bytes/);
  assert.match(sourceApply, /ACTIVE_STYLE_CONTEXT_SCHEMA/);
  assert.match(sourceApply, /MAX_SOURCE_PATH_BYTES: usize = 4096/);
  assert.match(sourceApply, /MAX_CLASS_NAME_BYTES: usize = 4096/);
  assert.match(sourceApply, /MAX_CSS_BYTES: usize = 32 \* 1024/);
  assert.match(sourceApply, /MAX_GENERATOR_ID_BYTES: usize = 128/);
  assert.match(sourceApply, /MAX_SOURCE_SPAN_BYTES: u64 = 16 \* 1024/);
  assert.match(sourceApply, /MAX_SOURCE_DIGEST_BYTES: usize = 128/);
  assert.match(sourceApply, /MAX_CONTEXT_KIND_BYTES: usize = 64/);
  assert.match(sourceApply, /MAX_CSS_SOURCE_EDIT_SAFETY_BYTES: usize = 128/);
  assert.match(sourceApply, /SOURCE_DIGEST_PREFIX: &str = "fnv1a64:"/);
  assert.match(sourceApply, /source_apply_review_receipt/);
  assert.match(sourceApply, /"contract_schema": DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA/);
  assert.match(sourceApply, /"ipc_kind": DX_STYLE_APPLY_KIND/);
  assert.match(sourceApply, /reviewed_with_blockers/);
  assert.match(sourceApply, /review_status/);
  assert.match(sourceApply, /mutation_ready/);
  assert.match(sourceApply, /unsupported or missing DX Style source-apply contract schema/);
  assert.match(sourceApply, /source-apply contract IPC kind does not match payload kind/);
  assert.match(sourceApply, /source-apply contract is review-only/);
  assert.match(sourceApply, /source-apply contract is missing active context kind guard/);
  assert.match(sourceApply, /source-apply contract is missing review context kinds/);
  assert.match(sourceApply, /source-apply contract is missing mutation context kind/);
  assert.match(sourceApply, /source-apply contract is missing reverse CSS map guard/);
  assert.match(sourceApply, /source-apply contract is missing declaration-delta guard/);
  assert.match(sourceApply, /source-apply contract is missing CSS declaration dry-run guard/);
  assert.match(sourceApply, /source-apply contract is missing reverse-delta provenance guard/);
  assert.match(sourceApply, /missing DX Style reverse CSS delta contract schema/);
  assert.match(sourceApply, /reverse CSS delta contract is not review-only/);
  assert.match(sourceApply, /reverse CSS delta contract is missing provenance guard/);
  assert.match(sourceApply, /reverse CSS delta contract has no supported properties/);
  assert.match(sourceApply, /reverse CSS delta contract has no required preview provenance fields/);
  assert.match(sourceApply, /reverse CSS delta contract is missing required provenance identity fields/);
  assert.match(sourceApply, /validate_reverse_delta_preview_provenance/);
  assert.match(sourceApply, /validate_required_preview_provenance_field/);
  assert.match(sourceApply, /compare_required_str_or_null/);
  assert.match(sourceApply, /compare_required_u64_or_null/);
  assert.match(sourceApply, /reverse CSS delta contract requires unsupported provenance field/);
  assert.match(sourceApply, /is missing from reverse CSS delta preview/);
  assert.match(sourceApply, /is missing from active group context/);
  assert.match(sourceApply, /does not match active group context/);
  assert.match(sourceApply, /ready reverse CSS delta preview lacks reverse CSS map provenance/);
  assert.match(sourceApply, /validate_contract_u64/);
  assert.match(sourceApply, /string_array_contains/);
  assert.match(sourceApply, /string_array_at/);
  assert.match(sourceApply, /"source-apply contract"/);
  assert.match(sourceApply, /\{contract_name\} \{field\} does not match native limit/);
  assert.match(sourceApply, /"contract":/);
  assert.match(sourceApply, /"context_kind": context_kind/);
  assert.match(sourceApply, /"css_source_edit_safety": css_source_edit_safety/);
  assert.match(sourceApply, /"review_context_kinds": string_array_at\(contract, "\/review_context_kinds"\)/);
  assert.match(sourceApply, /"mutation_context_kinds_when_enabled": string_array_at\(contract, "\/mutation_context_kinds_when_enabled"\)/);
  assert.match(sourceApply, /"max_source_path_bytes"/);
  assert.match(sourceApply, /"max_class_name_bytes"/);
  assert.match(sourceApply, /"max_css_bytes"/);
  assert.match(sourceApply, /"max_generator_id_bytes"/);
  assert.match(sourceApply, /"max_source_span_bytes"/);
  assert.match(sourceApply, /"reverse_css_delta_contract":/);
  assert.match(sourceApply, /"supported_property_count"/);
  assert.match(sourceApply, /"required_provenance_field_count"/);
  assert.match(sourceApply, /"example_target_utility"/);
  assert.match(sourceApply, /"reverse_css_delta_preview":/);
  assert.match(sourceApply, /"provenance_matches_context"/);
  assert.match(sourceApply, /"group_alias"/);
  assert.match(sourceApply, /"group_registry_receipt"/);
  assert.match(sourceApply, /"reverse_css_map_status"/);
  assert.match(sourceApply, /"group_source_state"/);
  assert.match(sourceApply, /"target_utility"/);
  assert.match(sourceApply, /"replacement_source_declaration"/);
  assert.match(sourceApply, /source_mutation/);
  assert.match(sourceApply, /"dry_run_review":/);
  assert.match(sourceApply, /"trusted_receipt_present"/);
  assert.match(sourceApply, /"receipt_summary": apply_gate\.get\("receipt_summary"\)\.cloned\(\)/);
  assert.match(sourceApply, /"receipt_mismatch": apply_gate\.get\("receipt_mismatch"\)\.cloned\(\)/);
  assert.match(sourceApply, /not_performed_by_review_receipt/);
  assert.match(sourceApply, /native source writer capability is review-only/);
  assert.match(sourceApply, /Web Preview did not declare review request capability/);
  assert.match(sourceApply, /Web Preview cannot declare native mutation capability/);
  assert.match(sourceApply, /"mutation_ready": false/);
  assert.match(sourceApply, /web_preview_declared_review_capability/);
  assert.match(sourceApply, /web_preview_declared_mutation_capability/);
  assert.doesNotMatch(sourceApply, /accepted_for_source_writer/);
  assert.match(sourceApply, /editor write bridge is not ready/);
  assert.match(sourceApply, /request source_path does not match context source_path/);
  assert.match(sourceApply, /request source_span does not match context source_span/);
  assert.match(sourceApply, /request source_digest does not match context source_digest/);
  assert.match(sourceApply, /context source_digest is not a complete fnv1a64 digest/);
  assert.match(sourceApply, /request source_digest is not a complete fnv1a64 digest/);
  assert.match(sourceApply, /fn is_source_digest/);
  assert.match(sourceApply, /strip_prefix\(SOURCE_DIGEST_PREFIX\)/);
  assert.match(sourceApply, /digest\.len\(\) == 16/);
  assert.match(sourceApply, /source_len_bytes/);
  assert.match(sourceApply, /context source_span exceeds context source length/);
  assert.match(sourceApply, /request source_span exceeds context source length/);
  assert.match(sourceApply, /style apply gate is ready without a trusted dry-run receipt/);
  assert.match(sourceApply, /style apply gate is ready without an active-source receipt match/);
  assert.match(sourceApply, /style apply gate is ready without a receipt path/);
  assert.match(sourceApply, /source-apply contract is missing active source digest guard/);
  assert.match(sourceApply, /source-apply contract is missing native active editor source revalidation guard/);
  assert.match(sourceApply, /source-apply contract is missing native active editor source revalidation receipt field/);
  assert.match(sourceApply, /source-apply contract is missing cursor-scoped dry-run edit preview guard/);
  assert.match(sourceApply, /source-apply contract is missing dry-run edit review receipt field/);
  assert.match(sourceApply, /source-apply contract is missing source-write readiness receipt field/);
  assert.match(sourceApply, /native active editor source revalidation schema is missing or invalid/);
  assert.match(sourceApply, /native active editor source revalidation did not match active source/);
  assert.match(sourceApply, /native active editor source revalidation path does not match request source_path/);
  assert.match(sourceApply, /native active editor source revalidation digest does not match request source_digest/);
  assert.match(sourceApply, /native active editor source revalidation span does not match request source_span/);
  assert.match(sourceApply, /native active editor source revalidation is missing session-bound source identity/);
  assert.match(sourceApply, /session-bound source identity path does not match request source_path/);
  assert.match(sourceApply, /session-bound source identity digest does not match request source_digest/);
  assert.match(sourceApply, /session-bound source identity length does not match context source_len_bytes/);
  assert.match(sourceApply, /session-bound source identity span does not match request source_span/);
  assert.match(sourceApply, /session-bound source identity is missing native editor identity/);
  assert.match(sourceApply, /native editor identity is missing \{field\}/);
  assert.match(sourceApply, /native editor identity workspace item does not match editor entity/);
  assert.match(sourceApply, /native editor identity buffer_kind is not singleton/);
  assert.match(sourceApply, /native editor identity is missing project_path/);
  assert.match(sourceApply, /"native_active_editor_source_revalidation": native_active_editor_source_revalidation/);
  assert.doesNotMatch(
    sourceApply,
    new RegExp("native active editor source revalidation is not yet " + "performed"),
  );
  assert.match(sourceApply, /"source_digest": request_source_digest/);
  assert.match(sourceApply, /MAX_DRY_RUN_EDIT_PREVIEWS: usize = 3/);
  assert.match(sourceApply, /MAX_DRY_RUN_REPLACEMENT_TEXT_BYTES: usize = 4096/);
  assert.match(sourceApply, /"max_dry_run_edit_previews"/);
  assert.match(sourceApply, /"max_dry_run_replacement_text_bytes"/);
  assert.match(sourceApply, /let dry_run_edit_review_evidence = dry_run_edit_review/);
  assert.match(sourceApply, /"dry_run_edit_review": dry_run_edit_review_evidence/);
  assert.match(sourceApply, /"source_write_readiness": source_write_readiness_evidence/);
  assert.match(sourceApply, /fn source_write_readiness/);
  assert.match(sourceApply, /"mutation_ready": safe_to_mutate/);
  assert.match(sourceApply, /source_mutation_contract_disabled/);
  assert.match(sourceApply, /cursor_scoped_dry_run_edit_review_missing/);
  assert.match(sourceApply, /native_active_editor_source_revalidation_missing/);
  assert.match(sourceApply, /native_review_reasons_present/);
  assert.match(sourceApply, /editor_write_bridge_not_ready/);
  assert.match(sourceApply, /mutation_capable_editor_write_bridge_missing/);
  assert.match(sourceApply, /native_writer_can_mutate_false/);
  assert.match(sourceApply, /runtime_webview_build_proof_missing/);
  assert.match(sourceApply, /source_write_readiness_refused/);
  assert.match(sourceApply, /refused_untrusted_session/);
  assert.match(sourceApply, /fn dry_run_edit_review/);
  assert.match(sourceApply, /fn dry_run_edit_preview_for_source_span/);
  assert.match(sourceApply, /fn review_source_paths_match/);
  assert.match(sourceApply, /fn review_source_paths_equal/);
  assert.match(sourceApply, /dry-run edit review is missing structured edit previews/);
  assert.match(sourceApply, /trusted dry-run receipt has no structured edit preview scoped to the active source span/);
  assert.match(sourceApply, /"structured_edit_preview_count": scoped_previews\.len\(\)/);
  assert.match(sourceApply, /"structured_edit_previews": scoped_previews/);
  assert.match(sourceApply, /"replacement_text": replacement_text/);
  assert.match(surfaceScript, /source_len_bytes: zedStyleContext\?\.source_len_bytes \|\| null/);
  assert.match(sourceApply, /context kind is not listed in the source-apply review contract/);
  assert.match(sourceApply, /CSS declaration context is missing source edit safety/);
  assert.match(sourceApply, /missing DX Style CSS declaration dry-run contract schema/);
  assert.match(sourceApply, /CSS declaration dry-run contract is not review-only/);
  assert.match(sourceApply, /CSS declaration dry-run contract is missing \{field\} receipt field/);
  assert.match(sourceApply, /CSS declaration source edit safety is not accepted for dry-run/);
  assert.match(sourceApply, /CSS declaration dry-run preview is missing proposed declaration/);
  assert.match(sourceApply, /CSS_DECLARATION_DRY_RUN_MAX_DECLARATION_BYTES/);
  assert.match(sourceApply, /MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTICS/);
  assert.match(sourceApply, /MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTIC_BYTES/);
  assert.match(sourceApply, /max_declaration_bytes/);
  assert.match(sourceApply, /max_diagnostic_count/);
  assert.match(sourceApply, /max_diagnostic_bytes/);
  assert.match(sourceApply, /validate_named_contract_u64/);
  assert.match(sourceApply, /\{contract_name\} \{field\} does not match native limit/);
  assert.match(sourceApply, /validate_named_contract_u64\(\s*css_declaration_dry_run_contract,\s*"CSS declaration dry-run contract",\s*"max_declaration_bytes",\s*CSS_DECLARATION_DRY_RUN_MAX_DECLARATION_BYTES as u64/s);
  assert.match(sourceApply, /validate_named_contract_u64\(\s*css_declaration_dry_run_contract,\s*"CSS declaration dry-run contract",\s*"max_diagnostic_count",\s*MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTICS as u64/s);
  assert.match(sourceApply, /validate_named_contract_u64\(\s*css_declaration_dry_run_contract,\s*"CSS declaration dry-run contract",\s*"max_diagnostic_bytes",\s*MAX_CSS_DECLARATION_DRY_RUN_DIAGNOSTIC_BYTES as u64/s);
  assert.match(sourceApply, /validate_named_contract_u64\(\s*css_declaration_dry_run_contract,\s*"CSS declaration dry-run contract",\s*"max_source_path_bytes",\s*MAX_SOURCE_PATH_BYTES as u64/s);
  assert.match(sourceApply, /validate_named_contract_u64\(\s*css_declaration_dry_run_contract,\s*"CSS declaration dry-run contract",\s*"max_source_span_bytes",\s*MAX_SOURCE_SPAN_BYTES/s);
  assert.match(sourceApply, /validate_named_contract_u64\(\s*css_declaration_dry_run_contract,\s*"CSS declaration dry-run contract",\s*"max_source_digest_bytes",\s*MAX_SOURCE_DIGEST_BYTES as u64/s);
  assert.match(sourceApply, /CSS declaration dry-run preview is not ready for review/);
  assert.match(sourceApply, /CSS declaration dry-run proposed declaration exceeds/);
  assert.match(sourceApply, /source-apply contract is missing preview output metadata guard/);
  assert.match(sourceApply, /source-apply contract is missing preview output receipt field/);
  assert.match(sourceApply, /source-apply contract is missing \{field\} receipt field/);
  assert.match(sourceApply, /"css_declaration_dry_run_preview_diagnostics"/);
  assert.match(sourceApply, /max_preview_kind_bytes/);
  assert.match(sourceApply, /max_preview_anatomy_part_bytes/);
  assert.match(sourceApply, /max_preview_anatomy_parts/);
  assert.match(sourceApply, /"preview_output":/);
  assert.match(sourceApply, /"preview_kind": preview_kind/);
  assert.match(sourceApply, /"preview_anatomy": preview_anatomy\.clone\(\)/);
  assert.match(sourceApply, /fn bounded_string_array/);
  assert.match(sourceApply, /"css_declaration_dry_run_contract":/);
  assert.match(sourceApply, /"dry_run_receipt_schema": css_declaration_dry_run_contract\.get\("dry_run_receipt_schema"\)\.and_then\(Value::as_str\)/);
  assert.match(sourceApply, /"max_declaration_bytes": css_declaration_dry_run_contract\.get\("max_declaration_bytes"\)\.and_then\(Value::as_u64\)/);
  assert.match(sourceApply, /"max_diagnostic_count": css_declaration_dry_run_contract\.get\("max_diagnostic_count"\)\.and_then\(Value::as_u64\)/);
  assert.match(sourceApply, /"max_diagnostic_bytes": css_declaration_dry_run_contract\.get\("max_diagnostic_bytes"\)\.and_then\(Value::as_u64\)/);
  assert.match(sourceApply, /"max_source_path_bytes": css_declaration_dry_run_contract\.get\("max_source_path_bytes"\)\.and_then\(Value::as_u64\)/);
  assert.match(sourceApply, /"max_source_span_bytes": css_declaration_dry_run_contract\.get\("max_source_span_bytes"\)\.and_then\(Value::as_u64\)/);
  assert.match(sourceApply, /"max_source_digest_bytes": css_declaration_dry_run_contract\.get\("max_source_digest_bytes"\)\.and_then\(Value::as_u64\)/);
  assert.match(sourceApply, /"review_receipt_fields": string_array_at\(css_declaration_dry_run_contract, "\/review_receipt_fields"\)/);
  assert.match(sourceApply, /"css_declaration_dry_run_diagnostics": css_declaration_dry_run_diagnostics/);
  assert.match(sourceApply, /"css_declaration_dry_run_preview":/);
  assert.match(sourceApply, /"proposed_declaration": css_dry_run_proposed_declaration/);
  assert.match(sourceApply, /"css_declaration_dry_run_preview_diagnostics": css_declaration_dry_run_preview_diagnostics/);
  assert.match(sourceApply, /fn optional_bounded_string_array/);
  assert.match(sourceApply, /CSS declaration dry-run diagnostics are not empty/);
  assert.match(sourceApply, /CSS declaration dry-run preview diagnostics are not empty/);
  assert.match(sourceApply, /CSS declaration dry-run diagnostics require a CSS declaration context/);
  assert.match(sourceApply, /"accepted_source_edit_safety": string_array_at/);
  assert.doesNotMatch(sourceApply, /fs::write|File::create|Command::new|spawn/);
  assert.equal(
    styleSourceApplyFixture.schema,
    "dx.style.grouped-class-source-apply-contract",
  );
  assert.equal(styleSourceApplyFixture.ipc_kind, "dx-style-source-apply");
  assert.equal(
    styleSourceApplyFixture.receipt_schema,
    "zed.web_preview.dx_style_source_apply_receipt.v1",
  );
  assert.equal(styleSourceApplyFixture.active_context_schema, "zed.dx_style.active_context.v1");
  assert.equal(
    styleSourceApplyFixture.source_apply_session_kind,
    "zed.web_preview.dx_style.source_apply_session",
  );
  assert.equal(styleSourceApplyFixture.source_mutation_enabled, false);
  assert.ok(styleSourceApplyFixture.review_receipt_fields.includes("dry_run_review"));
  assert.ok(styleSourceApplyFixture.review_receipt_fields.includes("dry_run_edit_review"));
  assert.ok(
    styleSourceApplyFixture.review_receipt_fields.includes("source_write_readiness"),
  );
  assert.ok(styleSourceApplyFixture.review_receipt_fields.includes("source_apply_session"));
  assert.ok(styleSourceApplyFixture.review_receipt_fields.includes("context_kind"));
  assert.ok(
    styleSourceApplyFixture.review_receipt_fields.includes("css_source_edit_safety"),
  );
  assert.ok(
    styleSourceApplyFixture.required_editor_guards.includes(
      "source-owned preview output metadata",
    ),
  );
  assert.ok(styleSourceApplyFixture.review_receipt_fields.includes("preview_output"));
  assert.ok(
    styleSourceApplyFixture.review_receipt_fields.includes(
      "css_declaration_dry_run_diagnostics",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.review_receipt_fields.includes(
      "css_declaration_dry_run_preview",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.review_receipt_fields.includes(
      "css_declaration_dry_run_preview_diagnostics",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.review_receipt_fields.includes(
      "css_declaration_dry_run_contract",
    ),
  );
  assert.ok(styleSourceApplyFixture.review_receipt_fields.includes("mutation_ready"));
  assert.ok(
    styleSourceApplyFixture.required_editor_guards.includes(
      "trusted Web Preview source-apply session",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.required_editor_guards.includes(
      "active source digest match",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.required_editor_guards.includes(
      "session-bound source identity",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.required_editor_guards.includes(
      "native active editor source revalidation",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.review_receipt_fields.includes(
      "native_active_editor_source_revalidation",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.required_editor_guards.includes(
      "CSS declaration dry-run receipt for CSS contexts",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.required_editor_guards.includes(
      "cursor-scoped dry-run structured edit preview",
    ),
  );
  assert.ok(styleSourceApplyFixture.review_context_kinds.includes("css_declaration"));
  assert.ok(
    styleSourceApplyFixture.mutation_context_kinds_when_enabled.includes("class_token"),
  );
  assert.equal(styleSourceApplyFixture.max_source_digest_bytes, 128);
  assert.equal(styleSourceApplyFixture.max_preview_kind_bytes, 64);
  assert.equal(styleSourceApplyFixture.max_preview_anatomy_part_bytes, 64);
  assert.equal(styleSourceApplyFixture.max_preview_anatomy_parts, 8);
  assert.equal(styleSourceApplyFixture.max_source_path_bytes, 4096);
  assert.equal(styleSourceApplyFixture.max_class_name_bytes, 4096);
  assert.equal(styleSourceApplyFixture.max_css_bytes, 32768);
  assert.equal(styleSourceApplyFixture.max_generator_id_bytes, 128);
  assert.equal(styleSourceApplyFixture.max_source_span_bytes, 16384);
  assert.equal(styleSourceApplyFixture.max_source_apply_session_token_bytes, 256);
  assert.equal(styleSourceApplyFixture.max_dry_run_edit_previews, 3);
  assert.equal(styleSourceApplyFixture.max_dry_run_replacement_text_bytes, 4096);
  assert.ok(
    styleSourceApplyFixture.required_native_handler_capabilities.includes(
      "can_review_request",
    ),
  );
  assert.ok(
    styleSourceApplyFixture.required_native_handler_capabilities.includes(
      "can_mutate_source",
    ),
  );
  assert.match(surface, /DX_STYLE_GENERATOR_SURFACE_SCHEMA/);
  assert.match(surface, /zed\.web_preview\.dx_style_generator_surface\.v1/);
  assert.match(surface, /ACTIVE_STYLE_CONTEXT_SCHEMA/);
  assert.match(surface, /MAX_DX_STYLE_CONTEXT_JSON_BYTES: usize = 256 \* 1024/);
  assert.match(surface, /MAX_DX_STYLE_SOURCE_APPLY_SESSION_TOKEN_BYTES: usize = 256/);
  assert.match(surface, /dx_style_generator_url_with_context_and_source_apply_session/);
  assert.match(surface, /bounded_source_apply_session_token/);
  assert.match(surface, /bounded_source_context_json/);
  assert.match(surface, /blocked_source_context_json/);
  assert.match(surface, /script_safe_json_string_literal/);
  assert.match(surface, /serde_json::from_str::<serde_json::Value>/);
  assert.match(surface, /\.replace\("<\/", "<\\\\\/"\)/);
  assert.match(surface, /context payload too large/);
  assert.match(surface, /invalid context payload/);
  assert.match(surface, /needs_valid_style_context/);
  assert.match(surface, /disabled_until_valid_style_context/);
  assert.match(surface, /data-dx-style-generator-schema/);
  assert.match(surface, /mod catalog/);
  assert.match(surface, /mod controls/);
  assert.match(surface, /mod css_declaration_dry_run_contract/);
  assert.match(surface, /mod css_declaration_dry_run_script/);
  assert.match(surface, /mod fixture/);
  assert.match(surface, /mod recipes/);
  assert.match(surface, /mod reverse_css_delta_contract/);
  assert.match(surface, /mod script/);
  assert.match(surface, /mod source_apply_contract/);
  assert.match(surface, /mod source_apply_session_script/);
  assert.match(surface, /mod group_context_contract/);
  assert.match(surface, /mod style/);
  assert.match(surface, /dx_style_generator_catalog_json/);
  assert.match(surface, /dx_style_generator_controls_json/);
  assert.match(surface, /dx_style_css_declaration_dry_run_contract_json/);
  assert.match(surface, /dx_style_generator_recipes_json/);
  assert.match(surface, /dx_style_source_apply_contract_json/);
  assert.match(surface, /dx_style_group_context_contract_json/);
  assert.match(surface, /dx_style_reverse_css_delta_contract_json/);
  assert.match(surface, /dx_style_generator_css/);
  assert.match(surface, /dx_style_generator_script/);
  assert.match(surface, /__DX_STYLE_GENERATOR_CSS__/);
  assert.match(surface, /__DX_STYLE_GENERATOR_CATALOG_JSON__/);
  assert.match(surface, /__DX_STYLE_GENERATOR_CONTROLS_JSON__/);
  assert.match(surface, /__DX_STYLE_GENERATOR_RECIPES_JSON__/);
  assert.match(surface, /__DX_STYLE_SOURCE_APPLY_CONTRACT_JSON__/);
  assert.match(surface, /__DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_JSON__/);
  assert.match(surface, /__DX_STYLE_GROUP_CONTEXT_CONTRACT_JSON__/);
  assert.match(surface, /__DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_JSON__/);
  assert.match(surface, /__DX_STYLE_SOURCE_APPLY_SESSION_TOKEN__/);
  assert.match(surface, /__DX_STYLE_GENERATOR_SCRIPT__/);
  assert.match(surface, /id="metadataStatus"/);
  assert.match(surface, /id="generatorSearch"/);
  assert.match(surface, /id="copyClassButton"/);
  assert.match(surface, /id="copyCssButton"/);
  assert.match(surface, /id="copyReviewButton"/);
  assert.match(surface, /Filter visual generators/);
  assert.match(surfaceCatalog, /DX_STYLE_VISUAL_GENERATOR_CATALOG_SCHEMA/);
  assert.match(surfaceCatalog, /DX_STYLE_CATALOG_PATH_ENV/);
  assert.match(surfaceCatalog, /DX_STYLE_EMBEDDED_CATALOG_FIXTURE_JSON/);
  assert.match(surfaceCatalog, /include_str!\("visual-generator-catalog\.generated\.json"\)/);
  assert.match(surfaceCatalog, /dx_style_catalog_fixture_path/);
  assert.match(surfaceCatalog, /bounded_json_fixture/);
  assert.match(surfaceCatalog, /catalog_fixture_to_web_preview_json/);
  assert.match(surfaceCatalog, /embedded:dx-style-catalog-fixture/);
  assert.match(surfaceCatalog, /__source/);
  assert.match(surfaceCatalog, /applicable_class_families/);
  assert.match(surfaceCatalog, /preferred_output/);
  assert.match(surfaceCatalog, /source_edit_safety/);
  assert.doesNotMatch(surfaceCatalog, /token_hints_for/);
  assert.match(surfaceFixture, /DX_STYLE_ROOT_ENV/);
  assert.match(surfaceFixture, /DX_STYLE_DEFAULT_ROOT/);
  assert.match(surfaceFixture, /MAX_DX_STYLE_FIXTURE_BYTES/);
  assert.match(surfaceFixture, /dx_style_fixture_path/);
  assert.match(surfaceFixture, /bounded_json_fixture/);
  assert.match(surfaceStyle, /DX_STYLE_GENERATOR_CSS/);
  assert.match(surfaceStyle, /@keyframes dx-style-pulse/);
  assert.match(surfaceStyle, /\.workspace/);
  assert.match(surfaceStyle, /\.catalog-search/);
  assert.match(surfaceStyle, /\.catalog-empty/);
  assert.match(surfaceStyle, /data-preview-kind="layout-items"/);
  assert.match(surfaceStyle, /\.timeline-track/);
  assert.match(surfaceStyle, /\.swatch-row/);
  assert.match(surfaceRecipes, /DX_STYLE_VISUAL_GENERATOR_RECIPE_CATALOG_SCHEMA/);
  assert.match(surfaceRecipes, /DX_STYLE_RECIPE_CATALOG_PATH_ENV/);
  assert.match(surfaceRecipes, /DX_STYLE_EMBEDDED_RECIPE_FIXTURE_JSON/);
  assert.match(surfaceRecipes, /include_str!\("visual-generator-recipe-catalog\.generated\.json"\)/);
  assert.match(surfaceRecipes, /dx_style_recipe_fixture_path/);
  assert.match(surfaceRecipes, /bounded_json_fixture/);
  assert.match(surfaceRecipes, /recipe_fixture_to_web_preview_json/);
  assert.match(surfaceRecipes, /__source/);
  assert.match(surfaceRecipes, /__value_keys/);
  assert.match(surfaceRecipes, /__preview_anatomy_parts/);
  assert.match(surfaceRecipes, /dx_style_generator_recipes_json/);
  assert.match(surfaceRecipes, /classTemplate/);
  assert.match(surfaceRecipes, /cssTemplate/);
  assert.match(surfaceRecipes, /preview_kind/);
  assert.match(surfaceRecipes, /previewKind/);
  assert.match(surfaceRecipes, /preview_anatomy/);
  assert.match(surfaceRecipes, /previewAnatomy/);
  assert.doesNotMatch(surfaceRecipes, /DX_STYLE_FALLBACK_/);
  assert.match(surfaceControls, /DX_STYLE_VISUAL_GENERATOR_CONTROL_CATALOG_SCHEMA/);
  assert.match(surfaceControls, /DX_STYLE_CONTROL_CATALOG_PATH_ENV/);
  assert.match(surfaceControls, /DX_STYLE_EMBEDDED_CONTROL_FIXTURE_JSON/);
  assert.match(surfaceControls, /include_str!\("visual-generator-control-catalog\.generated\.json"\)/);
  assert.match(surfaceControls, /dx_style_control_fixture_path/);
  assert.match(surfaceControls, /bounded_json_fixture/);
  assert.match(surfaceControls, /control_fixture_to_web_preview_json/);
  assert.match(surfaceControls, /__source/);
  assert.match(surfaceControls, /dx_style_generator_controls_json/);
  assert.match(surfaceControls, /controls/);
  assert.match(surfaceSourceApplyContract, /DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA/);
  assert.match(surfaceSourceApplyContract, /DX_STYLE_SOURCE_APPLY_CONTRACT_PATH_ENV/);
  assert.match(surfaceSourceApplyContract, /DX_STYLE_EMBEDDED_SOURCE_APPLY_CONTRACT_JSON/);
  assert.match(
    surfaceSourceApplyContract,
    /include_str!\("source-apply-contract\.generated\.json"\)/,
  );
  assert.match(surfaceSourceApplyContract, /dx_style_source_apply_fixture_path/);
  assert.match(surfaceSourceApplyContract, /bounded_json_fixture/);
  assert.match(surfaceSourceApplyContract, /source_apply_fixture_to_web_preview_json/);
  assert.match(surfaceSourceApplyContract, /source_mutation_enabled/);
  assert.match(surfaceSourceApplyContract, /source_apply_session_kind/);
  assert.match(surfaceSourceApplyContract, /"schema_version": fixture\.get\("schema_version"\)\?\.as_u64\(\)\?/);
  assert.match(surfaceSourceApplyContract, /"scope": fixture\.get\("scope"\)\?\.as_str\(\)\?/);
  assert.match(surfaceSourceApplyContract, /"fixture_path": fixture\.get\("fixture_path"\)\?\.as_str\(\)\?/);
  assert.match(surfaceSourceApplyContract, /review_context_kinds/);
  assert.match(surfaceSourceApplyContract, /mutation_context_kinds_when_enabled/);
  assert.match(surfaceSourceApplyContract, /review_receipt_fields/);
  assert.match(surfaceSourceApplyContract, /max_source_span_bytes/);
  assert.match(surfaceSourceApplyContract, /max_source_digest_bytes/);
  assert.match(surfaceSourceApplyContract, /max_source_apply_session_token_bytes/);
  assert.match(surfaceSourceApplyContract, /max_dry_run_edit_previews/);
  assert.match(surfaceSourceApplyContract, /max_dry_run_replacement_text_bytes/);
  assert.match(surfaceSourceApplyContract, /max_preview_kind_bytes/);
  assert.match(surfaceSourceApplyContract, /max_preview_anatomy_part_bytes/);
  assert.match(surfaceSourceApplyContract, /max_preview_anatomy_parts/);
  assert.match(surfaceSourceApplyContract, /"consumers": string_array\(&fixture, "consumers"\)\?/);
  assert.match(surfaceSourceApplyContract, /"notes": string_array\(&fixture, "notes"\)\?/);
  assert.match(surfaceSourceApplyContract, /fn string_array/);
  assert.match(
    surfaceCssDeclarationDryRunContract,
    /DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_SCHEMA/,
  );
  assert.match(
    surfaceCssDeclarationDryRunContract,
    /DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_PATH_ENV/,
  );
  assert.match(
    surfaceCssDeclarationDryRunContract,
    /include_str!\("css-declaration-dry-run-contract\.generated\.json"\)/,
  );
  assert.match(
    surfaceCssDeclarationDryRunContract,
    /css_declaration_dry_run_fixture_to_web_preview_json/,
  );
  assert.match(surfaceCssDeclarationDryRunContract, /accepted_source_edit_safety/);
  assert.match(surfaceCssDeclarationDryRunContract, /max_declaration_bytes/);
  assert.match(surfaceCssDeclarationDryRunContract, /max_diagnostic_count/);
  assert.match(surfaceCssDeclarationDryRunContract, /max_diagnostic_bytes/);
  assert.match(surfaceGroupContextContract, /DX_STYLE_GROUP_CONTEXT_CONTRACT_SCHEMA/);
  assert.match(surfaceGroupContextContract, /DX_STYLE_GROUP_CONTEXT_CONTRACT_PATH_ENV/);
  assert.match(surfaceGroupContextContract, /DX_STYLE_EMBEDDED_GROUP_CONTEXT_CONTRACT_JSON/);
  assert.match(
    surfaceGroupContextContract,
    /include_str!\("group-context-contract\.generated\.json"\)/,
  );
  assert.match(surfaceGroupContextContract, /dx_style_group_context_fixture_path/);
  assert.match(surfaceGroupContextContract, /group_context_fixture_to_web_preview_json/);
  assert.match(surfaceGroupContextContract, /max_alias_bytes/);
  assert.match(surfaceGroupContextContract, /max_utility_count/);
  assert.match(surfaceGroupContextContract, /candidate_min_utility_count/);
  assert.match(surfaceReverseCssDeltaContract, /DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_SCHEMA/);
  assert.match(surfaceReverseCssDeltaContract, /DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_PATH_ENV/);
  assert.match(surfaceReverseCssDeltaContract, /DX_STYLE_EMBEDDED_REVERSE_CSS_DELTA_CONTRACT_JSON/);
  assert.match(
    surfaceReverseCssDeltaContract,
    /include_str!\("reverse-css-delta-contract\.generated\.json"\)/,
  );
  assert.match(surfaceReverseCssDeltaContract, /dx_style_reverse_css_delta_fixture_path/);
  assert.match(surfaceReverseCssDeltaContract, /reverse_css_delta_fixture_to_web_preview_json/);
  assert.match(surfaceReverseCssDeltaContract, /supported_properties/);
  assert.match(surfaceReverseCssDeltaContract, /required_editor_guards/);
  assert.match(surfaceReverseCssDeltaContract, /required_preview_provenance_fields/);
  assert.match(surfaceReverseCssDeltaContract, /example_preview/);
  assert.equal(surfaceGeneratedRecipes.schema, "dx.style.visual-generator-recipe-catalog");
  assert.equal(surfaceGeneratedRecipes.entry_count, 25);
  assert.deepEqual(surfaceGeneratedRecipes.runtime_value_keys, styleRecipeFixture.runtime_value_keys);
  assert.equal(surfaceGeneratedCatalog.schema, "dx.style.visual-generator-catalog");
  assert.equal(surfaceGeneratedCatalog.generator_count, 25);
  assert.equal(surfaceGeneratedControls.schema, "dx.style.visual-generator-control-catalog");
  assert.equal(surfaceGeneratedControls.entry_count, 25);
  assert.deepEqual(surfaceGeneratedSourceApplyContract, styleSourceApplyFixture);
  assert.deepEqual(surfaceGeneratedCssDeclarationDryRunContract, styleCssDeclarationDryRunFixture);
  assert.deepEqual(surfaceGeneratedGroupContextContract, styleGroupContextFixture);
  assert.deepEqual(surfaceGeneratedReverseCssDeltaContract, styleReverseCssDeltaFixture);
  assert.match(surfaceScript, /DX_STYLE_GENERATOR_SCRIPT/);
  assert.match(surfaceScript, /dx_style_source_apply_session_constants_script/);
  assert.match(surfaceScript, /dx_style_source_apply_session_handler_script/);
  assert.match(surfaceScript, /dx_style_css_declaration_dry_run_constants_script/);
  assert.match(surfaceScript, /dx_style_css_declaration_dry_run_review_script/);
  assert.match(surfaceScript, /__DX_STYLE_SOURCE_APPLY_SESSION_CONSTANTS__/);
  assert.match(surfaceScript, /__DX_STYLE_SOURCE_APPLY_SESSION_HANDLER__/);
  assert.match(surfaceScript, /__DX_STYLE_CSS_DECLARATION_DRY_RUN_CONSTANTS__/);
  assert.match(surfaceScript, /__DX_STYLE_CSS_DECLARATION_DRY_RUN_REVIEW__/);
  assert.match(
    surfaceSourceApplySessionScript,
    /DX_STYLE_SOURCE_APPLY_SESSION_CONSTANTS_SCRIPT/,
  );
  assert.match(
    surfaceSourceApplySessionScript,
    /DX_STYLE_SOURCE_APPLY_SESSION_HANDLER_SCRIPT/,
  );
  assert.match(
    surfaceCssDeclarationDryRunScript,
    /DX_STYLE_CSS_DECLARATION_DRY_RUN_CONSTANTS_SCRIPT/,
  );
  assert.match(
    surfaceCssDeclarationDryRunScript,
    /DX_STYLE_CSS_DECLARATION_DRY_RUN_REVIEW_SCRIPT/,
  );
  assert.match(
    parseableDxStyleGeneratorScript(surfaceScript),
    /function reverseCssDeltaPreview\(output\)/,
  );
  assert.match(surfaceScript, /const catalogPayload = __DX_STYLE_GENERATOR_CATALOG_JSON__/);
  assert.match(surfaceScript, /catalogPayload\.entries/);
  assert.match(surfaceScript, /const controls = __DX_STYLE_GENERATOR_CONTROLS_JSON__/);
  assert.match(surfaceScript, /const recipes = __DX_STYLE_GENERATOR_RECIPES_JSON__/);
  assert.match(surfaceScript, /const sourceApplyContract = __DX_STYLE_SOURCE_APPLY_CONTRACT_JSON__/);
  assert.match(surfaceScript, /const cssDeclarationDryRunContract = __DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_JSON__/);
  assert.match(surfaceScript, /const groupContextContract = __DX_STYLE_GROUP_CONTEXT_CONTRACT_JSON__/);
  assert.match(surfaceScript, /const reverseCssDeltaContract = __DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_JSON__/);
  assert.match(surfaceScript, /const catalogSchema = catalogPayload\.__schema/);
  assert.match(surfaceScript, /const catalogSource = catalogPayload\.__source/);
  assert.match(surfaceScript, /const controlSchema = controls\.__schema/);
  assert.match(surfaceScript, /const controlSource = controls\.__source/);
  assert.match(surfaceScript, /const recipeSchema = recipes\.__schema/);
  assert.match(surfaceScript, /const recipeSource = recipes\.__source/);
  assert.match(surfaceScript, /const recipeValueKeys = Array\.isArray\(recipes\.__value_keys\)/);
  assert.match(surfaceScript, /const recipePreviewAnatomyParts = Array\.isArray\(recipes\.__preview_anatomy_parts\)/);
  assert.match(surfaceScript, /const recipePreviewAnatomyPartSet = new Set\(recipePreviewAnatomyParts\)/);
  assert.match(surfaceScript, /const sourceApplyContractSchema = sourceApplyContract\.__schema/);
  assert.match(surfaceScript, /const sourceApplyContractVersion = sourceApplyContract\.schema_version/);
  assert.match(surfaceScript, /const sourceApplyContractScope = sourceApplyContract\.scope/);
  assert.match(surfaceScript, /const sourceApplyIpcKind = sourceApplyContract\.ipc_kind/);
  assert.match(surfaceScript, /const sourceApplyReceiptSchema = sourceApplyContract\.receipt_schema/);
  assert.match(
    surfaceSourceApplySessionScript,
    /const sourceApplySessionKind = sourceApplyContract\.source_apply_session_kind/,
  );
  assert.match(
    surfaceSourceApplySessionScript,
    /const sourceApplySessionToken = __DX_STYLE_SOURCE_APPLY_SESSION_TOKEN__/,
  );
  assert.match(
    surfaceSourceApplySessionScript,
    /const sourceApplyMaxSessionTokenBytes = Number\(sourceApplyContract\.max_source_apply_session_token_bytes/,
  );
  assert.match(surfaceScript, /const sourceApplyMutationEnabled = sourceApplyContract\.source_mutation_enabled === true/);
  assert.match(surfaceScript, /const sourceApplyRequiredHandlerCapabilities = Array\.isArray\(sourceApplyContract\.required_native_handler_capabilities\)/);
  assert.match(surfaceScript, /const sourceApplyReviewContextKinds = Array\.isArray\(sourceApplyContract\.review_context_kinds\)/);
  assert.match(surfaceScript, /const sourceApplyMutationContextKinds = Array\.isArray\(sourceApplyContract\.mutation_context_kinds_when_enabled\)/);
  assert.match(surfaceCssDeclarationDryRunScript, /const cssDeclarationDryRunSchema = cssDeclarationDryRunContract\.__schema/);
  assert.match(surfaceCssDeclarationDryRunScript, /const cssDeclarationDryRunAcceptedSafety = Array\.isArray\(cssDeclarationDryRunContract\.accepted_source_edit_safety\)/);
  assert.match(surfaceScript, /const groupContextContractSchema = groupContextContract\.__schema/);
  assert.match(surfaceScript, /const groupContextMaxAliasBytes = Number\(groupContextContract\.max_alias_bytes/);
  assert.match(surfaceScript, /const groupContextMaxUtilityCount = Number\(groupContextContract\.max_utility_count/);
  assert.match(surfaceScript, /const groupContextCandidateMin = Number\(groupContextContract\.candidate_min_utility_count/);
  assert.match(surfaceScript, /const reverseCssDeltaContractSchema = reverseCssDeltaContract\.__schema/);
  assert.match(surfaceScript, /const reverseCssDeltaSupportedProperties = Array\.isArray\(reverseCssDeltaContract\.supported_properties\)/);
  assert.match(surfaceScript, /const reverseCssDeltaRequiredGuards = Array\.isArray\(reverseCssDeltaContract\.required_editor_guards\)/);
  assert.match(surfaceScript, /const reverseCssDeltaRequiredProvenanceFields = Array\.isArray\(reverseCssDeltaContract\.required_preview_provenance_fields\)/);
  assert.match(surfaceScript, /const reverseCssDeltaSupportedProvenanceFields = new Set/);
  assert.match(surfaceScript, /const sourceApplyByteLimits = \{/);
  assert.match(surfaceScript, /sourcePath: contractByteLimit\("max_source_path_bytes"\)/);
  assert.match(surfaceScript, /className: contractByteLimit\("max_class_name_bytes"\)/);
  assert.match(surfaceScript, /css: contractByteLimit\("max_css_bytes"\)/);
  assert.match(surfaceScript, /generator: contractByteLimit\("max_generator_id_bytes"\)/);
  assert.match(surfaceScript, /sourceSpan: contractByteLimit\("max_source_span_bytes"\)/);
  assert.match(surfaceScript, /sourceDigest: contractByteLimit\("max_source_digest_bytes"\)/);
  assert.match(surfaceScript, /dryRunEditPreviews: contractByteLimit\("max_dry_run_edit_previews"\)/);
  assert.match(surfaceScript, /dryRunReplacementText: contractByteLimit\("max_dry_run_replacement_text_bytes"\)/);
  assert.match(surfaceScript, /previewKind: contractByteLimit\("max_preview_kind_bytes"\)/);
  assert.match(surfaceScript, /previewAnatomyPart: contractByteLimit\("max_preview_anatomy_part_bytes"\)/);
  assert.match(surfaceScript, /previewAnatomyParts: contractByteLimit\("max_preview_anatomy_parts"\)/);
  assert.match(surfaceScript, /const expectedContextSchema = sourceApplyContract\.active_context_schema/);
  assert.match(surfaceScript, /const metadataDiagnostics = generatorMetadataDiagnostics\(\)/);
  assert.match(surfaceScript, /catalogQuery: ""/);
  assert.match(surfaceScript, /document\.getElementById\("metadataStatus"\)/);
  assert.match(surfaceScript, /document\.getElementById\("generatorSearch"\)/);
  assert.match(surfaceScript, /document\.getElementById\("copyClassButton"\)/);
  assert.match(surfaceScript, /document\.getElementById\("copyCssButton"\)/);
  assert.match(surfaceScript, /document\.getElementById\("copyReviewButton"\)/);
  assert.match(surfaceScript, /function generatorMetadataDiagnostics\(\)/);
  assert.match(surfaceScript, /function catalogGeneratorIds\(\)/);
  assert.match(surfaceScript, /function catalogSearchText\(entry\)/);
  assert.match(surfaceScript, /function filteredCatalog\(\)/);
  assert.match(surfaceScript, /function catalogEntryForGenerator\(generatorId\)/);
  assert.match(surfaceScript, /function catalogMetadataForGenerator\(generatorId\)/);
  assert.match(surfaceScript, /state\.catalogQuery\.trim\(\)\.toLowerCase\(\)/);
  assert.match(surfaceScript, /terms\.every\(\(term\) => haystack\.includes\(term\)\)/);
  assert.match(surfaceScript, /function metadataKeys\(source\)/);
  assert.match(surfaceScript, /function templatePlaceholderKeys\(template\)/);
  assert.match(surfaceScript, /unsupportedRecipePlaceholders/);
  assert.match(surfaceScript, /metadataDiagnostics\.status === "aligned"/);
  assert.match(surfaceScript, /metadata_missing_controls/);
  assert.match(surfaceScript, /metadata_missing_recipes/);
  assert.match(surfaceScript, /metadata_extra_controls/);
  assert.match(surfaceScript, /metadata_extra_recipes/);
  assert.match(surfaceScript, /recipe_value_keys/);
  assert.match(surfaceScript, /recipe_preview_anatomy_parts/);
  assert.match(surfaceScript, /metadata_unsupported_placeholders/);
  assert.match(surfaceScript, /metadata_missing_preview_anatomy/);
  assert.match(surfaceScript, /metadata_unsupported_preview_anatomy/);
  assert.match(surfaceScript, /Metadata aligned/);
  assert.match(surfaceScript, /Metadata drift/);
  assert.match(surfaceScript, /metadataStatusEl\.className = metadataAligned \? "pill ready" : "pill blocked"/);
  assert.match(surfaceScript, /DX Style metadata is out of sync/);
  assert.match(surfaceScript, /Zed Style context schema is unsupported/);
  assert.match(surfaceScript, /function generatorOutput/);
  assert.match(surfaceScript, /function activeRecipe\(id\)/);
  assert.match(surfaceScript, /previewKind: recipe\.previewKind \|\| "hero-card"/);
  assert.match(surfaceScript, /previewAnatomy: previewAnatomy\(recipe\)/);
  assert.match(surfaceScript, /function previewAnatomy\(recipe\)/);
  assert.match(surfaceScript, /function fallbackPreviewAnatomy\(kind\)/);
  assert.match(surfaceScript, /function previewMarkupForOutput\(output\)/);
  assert.match(surfaceScript, /function previewPartMarkup\(part\)/);
  assert.match(surfaceScript, /case "layout-items"/);
  assert.match(surfaceScript, /case "timeline-track"/);
  assert.match(surfaceScript, /case "swatch-row"/);
  assert.match(surfaceScript, /case "preview-title"/);
  assert.match(surfaceScript, /sampleEl\.dataset\.previewKind/);
  assert.match(surfaceScript, /sampleEl\.innerHTML = previewMarkupForOutput\(output\)/);
  assert.match(surfaceScript, /preview_kind: \$\{output\.previewKind/);
  assert.match(surfaceScript, /preview_anatomy: \$\{\(output\.previewAnatomy \|\| \[\]\)\.join/);
  assert.match(surfaceScript, /function activeControlDefinitions/);
  assert.match(surfaceScript, /function controlFromDefinition/);
  assert.match(surfaceScript, /recipeValues/);
  assert.match(surfaceScript, /applyRecipeTemplate/);
  assert.match(surfaceScript, /values\[key\] === undefined \? `\{\$\{key\}\}`/);
  assert.match(surfaceScript, /function contractByteLimit\(field\)/);
  assert.match(surfaceScript, /function utf8ByteLength\(value\)/);
  assert.match(surfaceScript, /new TextEncoder\(\)/);
  assert.match(surfaceScript, /function sourceSpanByteLength\(context\)/);
  assert.match(surfaceScript, /function sourceLengthReady\(context\)/);
  assert.match(surfaceScript, /function sourceSpanReady\(context\)/);
  assert.match(surfaceScript, /source_span_start_exceeds_end/);
  assert.match(surfaceScript, /source_span_exceeds_source_length/);
  assert.match(surfaceScript, /function exceedsContractLimit\(value, limit\)/);
  assert.match(surfaceScript, /function sourceApplyPayloadDiagnostics\(context, output\)/);
  assert.match(surfaceScript, /generator_id_exceeds_contract_limit/);
  assert.match(surfaceScript, /source_path_exceeds_contract_limit/);
  assert.match(surfaceScript, /class_name_exceeds_contract_limit/);
  assert.match(surfaceScript, /css_exceeds_contract_limit/);
  assert.match(surfaceScript, /source_span_exceeds_contract_limit/);
  assert.match(surfaceScript, /\^\$\{sourceDigestPrefix\}\[0-9a-fA-F\]\{16\}\$/);
  assert.match(surfaceScript, /Dry-run edits max/);
  assert.match(surfaceScript, /Replacement max/);
  assert.match(surfaceScript, /preview_kind_exceeds_contract_limit/);
  assert.match(surfaceScript, /preview_anatomy_part_exceeds_contract_limit/);
  assert.match(surfaceScript, /preview_anatomy_parts_exceeds_contract_limit/);
  assert.match(surfaceScript, /function reverseCssDeltaContractDiagnostics\(\)/);
  assert.match(surfaceScript, /reverse_css_delta_contract_missing_provenance_guard/);
  assert.match(surfaceScript, /reverse_css_delta_contract_missing_required_provenance_fields/);
  assert.match(surfaceScript, /reverse_css_delta_contract_unsupported_provenance_field/);
  assert.match(surfaceScript, /function tokenFromReverseDeltaValue\(value, mapping\)/);
  assert.match(surfaceScript, /reverseCssDeltaSupportedProperties\.filter/);
  assert.match(surfaceScript, /let declarationHadUnsupportedValue = false/);
  assert.match(surfaceScript, /display_keyword/);
  assert.match(surfaceScript, /margin_token_suffix/);
  assert.match(surfaceScript, /arbitrary_bracket_value/);
  assert.match(surfaceScript, /drop_shadow_function/);
  assert.match(surfaceScript, /backdrop_blur_function/);
  assert.match(surfaceScript, /align_items_keyword/);
  assert.match(surfaceScript, /justify_content_keyword/);
  assert.match(surfaceScript, /align_content_keyword/);
  assert.match(surfaceScript, /grid_track_repeat_count/);
  assert.match(surfaceScript, /transition_property_value/);
  assert.match(surfaceScript, /transition_timing_function_value/);
  assert.match(surfaceScript, /function targetUtilityFromReverseDelta\(mapping, token\)/);
  assert.match(surfaceScript, /function displayReverseDeltaToken\(value\)/);
  assert.match(surfaceScript, /function alignItemsReverseDeltaToken\(value\)/);
  assert.match(surfaceScript, /function justifyContentReverseDeltaToken\(value\)/);
  assert.match(surfaceScript, /function alignContentReverseDeltaToken\(value\)/);
  assert.match(surfaceScript, /function gridTrackRepeatCountToken\(value\)/);
  assert.match(surfaceScript, /function transitionPropertyReverseDeltaToken\(value\)/);
  assert.match(surfaceScript, /function transitionTimingFunctionReverseDeltaToken\(value\)/);
  assert.match(surfaceScript, /function arbitraryReverseDeltaToken\(value\)/);
  assert.match(surfaceScript, /function isDisplayUtility\(utility\)/);
  assert.match(surfaceScript, /function isMarginUtility\(utility, utilityPrefix\)/);
  assert.match(surfaceScript, /function isBaseGapUtility\(utility\)/);
  assert.match(surfaceScript, /function isOutlineColorUtility\(utility\)/);
  assert.match(surfaceScript, /function isTransitionPropertyUtility\(utility\)/);
  assert.match(surfaceScript, /function isFallbackReverseDeltaMapping\(mapping\)/);
  assert.match(surfaceScript, /property === "transition-property"/);
  assert.match(surfaceScript, /let fallbackStrategyPreview = null/);
  assert.match(surfaceScript, /if \(fallbackStrategyPreview\) return fallbackStrategyPreview/);
  assert.match(surfaceScript, /function isShadowEffectUtility\(utility\)/);
  assert.match(surfaceScript, /function isTextShadowEffectUtility\(utility\)/);
  assert.match(surfaceScript, /function isDropShadowEffectUtility\(utility\)/);
  assert.match(surfaceScript, /function reverseCssDeltaContextField\(field\)/);
  assert.match(surfaceScript, /function reverseCssDeltaPreviewProvenanceDiagnostics\(preview, group\)/);
  assert.match(surfaceScript, /reverse_css_delta_preview_missing:/);
  assert.match(surfaceScript, /reverse_css_delta_context_missing:/);
  assert.match(surfaceScript, /reverse_css_delta_preview_mismatch:/);
  assert.match(surfaceScript, /reverse_css_delta_preview_missing_reverse_map_provenance/);
  assert.match(surfaceSourceApplySessionScript, /function sourceApplyHandler\(\)/);
  assert.match(surfaceSourceApplySessionScript, /function sourceApplyReviewHandler\(\)/);
  assert.match(surfaceSourceApplySessionScript, /window\.__DX_STYLE_SOURCE_APPLY__/);
  assert.match(surfaceSourceApplySessionScript, /installSourceApplyHandler\(\)/);
  assert.match(surfaceSourceApplySessionScript, /function installSourceApplyHandler\(\)/);
  assert.match(surfaceSourceApplySessionScript, /kind: sourceApplyIpcKind/);
  assert.match(surfaceSourceApplySessionScript, /source_apply_session: \{/);
  assert.match(surfaceSourceApplySessionScript, /kind: sourceApplySessionKind/);
  assert.match(surfaceSourceApplySessionScript, /token: sourceApplySessionToken/);
  assert.match(surfaceSourceApplySessionScript, /handler_capability/);
  assert.match(surfaceSourceApplySessionScript, /can_review_request: true/);
  assert.match(surfaceSourceApplySessionScript, /can_mutate_source: false/);
  assert.match(surfaceSourceApplySessionScript, /function sourceApplyHandlerState\(\)/);
  assert.match(surfaceSourceApplySessionScript, /review_only/);
  assert.match(surfaceScript, /function contextSchemaSupported\(context\)/);
  assert.match(surfaceScript, /context\?\.schema === expectedContextSchema/);
  assert.match(surfaceScript, /function contextKindSupported\(context, supportedKinds\)/);
  assert.match(surfaceScript, /supportedKinds\.includes\(context\?\.context_kind \|\| ""\)/);
  assert.match(surfaceScript, /function sourceApplyBlocker\(applyGate, metadataAligned, context, output\)/);
  assert.match(surfaceScript, /metadata_drift/);
  assert.match(surfaceScript, /generator_recipe_missing/);
  assert.match(surfaceScript, /no_zed_style_context/);
  assert.match(surfaceScript, /unsupported_context_schema/);
  assert.match(surfaceScript, /css_declaration_requires_css_dry_run_receipt/);
  assert.match(surfaceScript, /unsupported_mutation_context_kind/);
  assert.match(surfaceScript, /unsupported_review_context_kind/);
  assert.match(surfaceScript, /missing_source_path/);
  assert.match(surfaceScript, /missing_source_span/);
  assert.match(surfaceScript, /missing_source_length/);
  assert.match(surfaceScript, /invalid_source_span/);
  assert.match(surfaceScript, /source_apply_contract_missing/);
  assert.match(surfaceScript, /reverse_css_delta_contract_missing/);
  assert.match(surfaceScript, /source_apply_contract_review_only/);
  assert.match(surfaceScript, /native_apply_writer_missing/);
  assert.match(surfaceScript, /native_apply_handler_missing/);
  assert.match(surfaceScript, /function sourceApplyReady\(applyGate, metadataAligned, context, output\)/);
  assert.match(surfaceScript, /sourceApplyBlocker\(applyGate, metadataAligned, context, output\) === "ready"/);
  assert.match(surfaceScript, /function sourceApplyReviewBlocker\(metadataAligned, context, output\)/);
  assert.match(surfaceScript, /function sourceApplyReviewReady\(metadataAligned, context, output\)/);
  assert.match(surfaceScript, /sourceApplyReviewBlocker\(metadataAligned, context, output\) === "ready"/);
  assert.match(surfaceCssDeclarationDryRunScript, /function cssDeclarationDryRunDiagnostics\(context\)/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_dry_run_contract_missing/);
  assert.match(surfaceCssDeclarationDryRunScript, /function cssDeclarationDryRunSourceLimitDiagnostics\(\)/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_dry_run_missing_source_path_byte_limit/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_dry_run_source_path_byte_limit_mismatch/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_dry_run_missing_source_span_byte_limit/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_dry_run_source_span_byte_limit_mismatch/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_dry_run_missing_source_digest_byte_limit/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_dry_run_source_digest_byte_limit_mismatch/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_source_edit_safety_not_accepted_for_dry_run/);
  assert.match(surfaceCssDeclarationDryRunScript, /const cssDeclarationDryRunMaxDeclarationBytes = Number\(cssDeclarationDryRunContract\.max_declaration_bytes/);
  assert.match(surfaceCssDeclarationDryRunScript, /const cssDeclarationDryRunMaxSourcePathBytes = Number\(cssDeclarationDryRunContract\.max_source_path_bytes/);
  assert.match(surfaceCssDeclarationDryRunScript, /const cssDeclarationDryRunMaxSourceSpanBytes = Number\(cssDeclarationDryRunContract\.max_source_span_bytes/);
  assert.match(surfaceCssDeclarationDryRunScript, /const cssDeclarationDryRunMaxSourceDigestBytes = Number\(cssDeclarationDryRunContract\.max_source_digest_bytes/);
  assert.match(surfaceCssDeclarationDryRunScript, /function cssDeclarationDryRunPreviewDiagnostics\(preview\)/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_dry_run_preview_not_ready/);
  assert.match(surfaceCssDeclarationDryRunScript, /css_declaration_dry_run_proposed_declaration_exceeds_contract_limit/);
  assert.match(surfaceScript, /cssDeclarationDryRunPreview\(output, context\)/);
  assert.match(surfaceScript, /if \(cssDeclarationPreviewDiagnostics\.length\) return cssDeclarationPreviewDiagnostics\[0\]/);
  assert.match(surfaceCssDeclarationDryRunScript, /function cssDeclarationDryRunPreview\(output, context = zedStyleContext\)/);
  assert.match(surfaceCssDeclarationDryRunScript, /ready_for_review/);
  assert.match(surfaceCssDeclarationDryRunScript, /fallback_generated_declaration/);
  assert.match(surfaceScript, /escapeHtml\(applyGate\?\.reason/);
  assert.match(surfaceScript, /escapeHtml\(label\)/);
  assert.match(surfaceCssDeclarationDryRunScript, /function generatedCssDeclarations\(css\)/);
  assert.match(surfaceScript, /function reverseCssDeltaPreviewProvenance\(group\)/);
  assert.match(surfaceScript, /group_registry_receipt: group\?\.registry_receipt/);
  assert.match(surfaceScript, /reverse_css_map_status: group\?\.reverse_css_map_status/);
  assert.match(surfaceScript, /function reverseCssDeltaPreview\(output\)/);
  assert.match(surfaceScript, /tokenFromReverseDeltaValue\(declaration\.value, mapping\)/);
  assert.match(surfaceScript, /function replacementUtilitiesForDelta/);
  assert.match(surfaceScript, /function isBorderColorUtility/);
  assert.match(surfaceScript, /function isBackgroundImageUtility\(utility\)/);
  assert.match(surfaceScript, /property === "background-image"/);
  assert.match(surfaceScript, /no_group_utilities/);
  assert.match(surfaceScript, /unsupported_declaration/);
  assert.match(surfaceScript, /contextSchemaSupported\(context\)/);
  assert.match(surfaceScript, /context\.source_path/);
  assert.match(surfaceScript, /Number\.isInteger\(context\.source_span\?\.start_byte\)/);
  assert.match(surfaceScript, /Number\.isInteger\(context\.source_span\?\.end_byte\)/);
  assert.match(surfaceScript, /applyGate\?\.editor_write_bridge\?\.can_apply/);
  assert.match(surfaceScript, /source_apply_session_kind_missing/);
  assert.match(surfaceScript, /source_apply_session_missing/);
  assert.match(surfaceScript, /source_apply_session_token_exceeds_contract_limit/);
  assert.match(surfaceScript, /function sourceApplyRequest\(output\)/);
  assert.match(surfaceScript, /const cssDeclarationPreview = cssDeclarationDryRunPreview\(output, zedStyleContext\)/);
  assert.match(surfaceScript, /source_apply_session: \{/);
  assert.match(surfaceScript, /token: sourceApplySessionToken/);
  assert.match(surfaceScript, /source_len_bytes: zedStyleContext\?\.source_len_bytes \|\| null/);
  assert.match(surfaceScript, /css_declaration_dry_run_contract: cssDeclarationDryRunContract/);
  assert.match(surfaceScript, /css_declaration_dry_run_diagnostics: cssDeclarationDryRunContextDiagnostics\(zedStyleContext\)/);
  assert.match(surfaceScript, /css_declaration_dry_run_preview: cssDeclarationPreview/);
  assert.match(surfaceScript, /css_declaration_dry_run_preview_diagnostics: cssDeclarationDryRunContextPreviewDiagnostics\(cssDeclarationPreview\)/);
  assert.match(surfaceScript, /function handleReviewApplyClick\(\)/);
  assert.match(surfaceScript, /function handleApplyClick\(\)/);
  assert.match(surfaceScript, /function reviewPacket\(output\)/);
  assert.match(surfaceScript, /const cssDeclarationPreview = cssDeclarationDryRunPreview\(output, zedStyleContext\)/);
  assert.match(surfaceScript, /function dryRunReviewPacket\(applyGate\)/);
  assert.match(surfaceScript, /dry_run_review: dryRunReviewPacket\(applyGate\)/);
  assert.match(surfaceScript, /source_apply_session: sourceApplySessionReviewPacket\(\)/);
  assert.match(surfaceScript, /editor_write_bridge: editorWriteBridgeReviewPacket\(applyGate\)/);
  assert.match(surfaceScript, /source_write_readiness: sourceWriteReadinessPacket\(applyGate, output\)/);
  assert.match(surfaceScript, /function sourceApplySessionReviewPacket\(\)/);
  assert.match(surfaceScript, /token_present: typeof sourceApplySessionToken === "string"/);
  assert.match(surfaceScript, /token_byte_length: tokenByteLength/);
  assert.match(surfaceScript, /within_contract_limit:/);
  assert.match(surfaceScript, /function editorWriteBridgeReviewPacket\(applyGate\)/);
  assert.match(surfaceScript, /present: true/);
  assert.match(surfaceScript, /can_mutate_source: bridge\.can_mutate_source === true/);
  assert.match(surfaceScript, /preflight_schema_version: Number\.isInteger\(bridge\.preflight_schema_version\)/);
  assert.match(surfaceScript, /preflight_scope: bridge\.preflight_scope \|\| null/);
  assert.match(surfaceScript, /summary: bridge\.summary \|\| null/);
  assert.match(surfaceScript, /native_handler_state: sourceApplyHandlerState\(\)/);
  assert.match(surfaceScript, /required_receipt_count: requiredReceipts\.length/);
  assert.match(surfaceScript, /required_guard_count: requiredGuards\.length/);
  assert.match(surfaceScript, /required_editor_guard_count: requiredGuards\.length/);
  assert.match(surfaceScript, /required_native_handler_count: requiredHandlers\.length/);
  assert.match(surfaceScript, /required_handler_capability_count: requiredCapabilities\.length/);
  assert.match(surfaceScript, /required_native_handler_capability_count: requiredCapabilities\.length/);
  assert.match(surfaceScript, /runtime_validation_required: bridge\.runtime_validation_required !== false/);
  assert.match(surfaceScript, /function sourceWriteReadinessPacket\(applyGate, output\)/);
  assert.match(surfaceScript, /schema: "zed\.web_preview\.dx_style\.source_write_readiness\.v1"/);
  assert.match(surfaceScript, /safe_to_mutate: safeToMutate/);
  assert.match(surfaceScript, /mutation_ready: safeToMutate/);
  assert.match(surfaceScript, /source_mutation_contract_disabled/);
  assert.match(surfaceScript, /cursor_scoped_dry_run_edit_review_missing/);
  assert.match(surfaceScript, /native_active_editor_source_revalidation_missing/);
  assert.match(surfaceScript, /editor_write_bridge_not_ready/);
  assert.match(surfaceScript, /mutation_capable_editor_write_bridge_missing/);
  assert.match(surfaceScript, /native_writer_can_mutate_false/);
  assert.match(surfaceScript, /runtime_webview_build_proof_missing/);
  assert.match(surfaceScript, /session_token_present: session\.token_present/);
  assert.doesNotMatch(surfaceScript, /source_write_readiness:[\s\S]{0,1200}token: sourceApplySessionToken/);
  assert.match(surfaceScript, /function dryRunStructuredEditPreviews\(applyGate\)/);
  assert.match(surfaceScript, /structured_edit_preview_count: structuredEditPreviews\.length/);
  assert.match(surfaceScript, /structured_edit_previews: structuredEditPreviews/);
  assert.match(surfaceScript, /replacement_text: typeof edit\?\.replacement_text === "string"/);
  assert.match(surfaceScript, /css_declaration_dry_run_contract:/);
  assert.match(surfaceScript, /max_declaration_bytes: cssDeclarationDryRunMaxDeclarationBytes/);
  assert.match(surfaceScript, /max_source_path_bytes: cssDeclarationDryRunMaxSourcePathBytes/);
  assert.match(surfaceScript, /max_source_span_bytes: cssDeclarationDryRunMaxSourceSpanBytes/);
  assert.match(surfaceScript, /max_source_digest_bytes: cssDeclarationDryRunMaxSourceDigestBytes/);
  assert.match(surfaceScript, /css_declaration_dry_run_diagnostics: cssDeclarationDiagnostics/);
  assert.match(surfaceScript, /css_declaration_dry_run_preview: cssDeclarationPreview/);
  assert.match(surfaceScript, /css_declaration_dry_run_preview_diagnostics: cssDeclarationPreviewDiagnostics/);
  assert.match(surfaceScript, /trusted_receipt_present: applyGate\.trusted_dry_run_receipt_present === true/);
  assert.match(surfaceScript, /zed\.web_preview\.dx_style_generator_review_packet\.v1/);
  assert.match(surfaceScript, /function copyTextToClipboard\(value\)/);
  assert.match(surfaceScript, /navigator\.clipboard\?\.writeText/);
  assert.match(surfaceScript, /document\.execCommand\("copy"\)/);
  assert.match(surfaceScript, /function handleCopy\(kind\)/);
  assert.match(surfaceScript, /copyClassButtonEl\.addEventListener\("click", \(\) => handleCopy\("class"\)\)/);
  assert.match(surfaceScript, /copyCssButtonEl\.addEventListener\("click", \(\) => handleCopy\("css"\)\)/);
  assert.match(surfaceScript, /copyReviewButtonEl\.addEventListener\("click", \(\) => handleCopy\("review"\)\)/);
  assert.match(surfaceScript, /reviewApplyButtonEl\.addEventListener\("click", handleReviewApplyClick\)/);
  assert.match(surfaceScript, /Source apply review request sent to the native handler/);
  assert.match(surfaceScript, /Source apply refused: native Web Preview apply handler is unavailable/);
  assert.match(surfaceScript, /applyButtonEl\.addEventListener\("click", handleApplyClick\)/);
  assert.match(surfaceScript, /web_preview_apply_handler/);
  assert.match(surfaceScript, /source_apply_ready/);
  assert.match(surfaceScript, /source_apply_blocker/);
  assert.match(surfaceScript, /source_apply_review_ready/);
  assert.match(surfaceScript, /source_apply_review_blocker/);
  assert.match(surfaceScript, /source_path: zedStyleContext\?\.source_path \|\| null/);
  assert.match(surfaceScript, /source_span: zedStyleContext\?\.source_span \|\| null/);
  assert.match(surfaceScript, /source_digest: zedStyleContext\?\.source_digest \|\| null/);
  assert.match(surfaceScript, /context_kind: zedStyleContext\?\.context_kind \|\| null/);
  assert.match(surfaceScript, /css_source_edit_safety: zedStyleContext\?\.css_source_edit_safety \|\| null/);
  assert.match(surfaceScript, /reverse_css_delta_contract: reverseCssDeltaContract/);
  assert.match(surfaceScript, /reverse_css_delta_preview: reverseCssDeltaPreview\(output\)/);
  assert.match(surface, /__DX_STYLE_CONTEXT_JSON_STRING__/);
  assert.match(surfaceScript, /context_status/);
  assert.match(surfaceScript, /context_schema/);
  assert.match(surfaceScript, /context_schema_supported/);
  assert.match(surfaceScript, /context_kind/);
  assert.match(surfaceScript, /css_source_edit_safety/);
  assert.match(surfaceScript, /source_path: \$\{zedStyleContext\.source_path\}/);
  assert.match(surfaceScript, /source_len_bytes: \$\{zedStyleContext\.source_len_bytes\}/);
  assert.match(surfaceScript, /active_token/);
  assert.match(surfaceScript, /css_property/);
  assert.match(surfaceScript, /css_generator/);
  assert.match(surfaceScript, /source_span_bytes/);
  assert.match(surfaceScript, /source_digest/);
  assert.match(surfaceScript, /generatorForToken/);
  assert.match(surfaceScript, /const contextGeneratorHint = generatorForContext/);
  assert.match(surfaceScript, /const contextGeneratorSource = contextGeneratorHint\?\.source/);
  assert.match(surfaceScript, /function generatorForContext\(context\)/);
  assert.match(surfaceScript, /context\?\.css_generator/);
  assert.match(surfaceScript, /catalogHasGenerator\(context\.css_generator\)/);
  assert.match(surfaceScript, /source: "css_declaration"/);
  assert.match(surfaceScript, /Array\.isArray\(context\?\.group_context\?\.utilities\)/);
  assert.match(surfaceScript, /source: "group_utilities"/);
  assert.match(surfaceScript, /Array\.isArray\(context\?\.attribute_tokens\)/);
  assert.match(surfaceScript, /source: "active_token"/);
  assert.match(surfaceScript, /source: "attribute_tokens"/);
  assert.match(surfaceScript, /tokenMatchesHint/);
  assert.match(surfaceScript, /hint\.includes\("\*"\)/);
  assert.doesNotMatch(surfaceScript, /token\.startsWith\("grid"\)/);
  assert.match(surfaceScript, /orderedCatalog/);
  assert.match(surfaceScript, /const visibleCatalog = filteredCatalog\(\)/);
  assert.match(surfaceScript, /No generators match this filter/);
  assert.match(surfaceScript, /generatorSearchEl\.addEventListener\("input"/);
  assert.doesNotMatch(surfaceScript, /category ===/);
  assert.doesNotMatch(surfaceScript, /case "radial-gradient"/);
  assert.doesNotMatch(surfaceScript, /case "grid-layout-editor"/);
  assert.equal(
    surfaceGeneratedRecipes.entries.some(
      (entry) => entry.generator_id === "radial-gradient",
    ),
    true,
  );
  assert.equal(
    surfaceGeneratedRecipes.entries.some(
      (entry) => entry.generator_id === "grid-layout-editor",
    ),
    true,
  );
  assert.equal(styleRecipes.length, 25);
  assert.deepEqual(surfaceGeneratedCatalog, styleCatalogFixture);
  assert.deepEqual(fixtureRecipes, styleRecipes);
  assert.deepEqual(generatedRecipes, fixtureRecipes);
  assert.deepEqual(surfaceGeneratedControls, styleControlFixture);
  assert.match(surfaceScript, /attribute_tokens:/);
  assert.match(surfaceScript, /suggested_generator_source/);
  assert.match(surfaceScript, /suggested_generator/);
  assert.match(surfaceScript, /generator_category/);
  assert.match(surfaceScript, /generator_preferred_output/);
  assert.match(surfaceScript, /generator_source_edit_safety/);
  assert.match(surfaceScript, /catalog_schema/);
  assert.match(surfaceScript, /catalog_source/);
  assert.match(surfaceScript, /control_schema/);
  assert.match(surfaceScript, /control_source/);
  assert.match(surfaceScript, /recipe_schema/);
  assert.match(surfaceScript, /recipe_source/);
  assert.match(surface, /id="applyButton"/);
  assert.match(surface, /id="reviewApplyButton"/);
  assert.match(surface, /id="patchReview"/);
  assert.match(surfaceScript, /Apply gated/);
  assert.match(surfaceScript, /Review gated/);
  assert.match(surfaceScript, /renderPatchReview/);
  assert.match(surfaceScript, /renderGeneratorSafetyReview/);
  assert.match(surfaceScript, /Generator source safety/);
  assert.match(surfaceScript, /renderBridgeReview/);
  assert.match(surfaceScript, /renderGroupContextReview/);
  assert.match(surfaceScript, /renderSourceApplyContractReview/);
  assert.match(surfaceScript, /renderCssDeclarationDryRunContractReview/);
  assert.match(surfaceScript, /CSS declaration dry-run contract/);
  assert.match(surfaceScript, /Proposed declaration/);
  assert.match(surfaceScript, /Required CSS context fields/);
  assert.match(surfaceScript, /Accepted source-edit safety/);
  assert.match(surfaceScript, /renderReverseCssDeltaContractReview/);
  assert.match(surfaceScript, /escapeHtml/);
  assert.match(surfaceScript, /apply_gate_reason/);
  assert.match(surfaceScript, /receipt_match/);
  assert.match(surfaceScript, /receipt_mismatch_checked/);
  assert.match(surfaceScript, /receipt_mismatch_reasons/);
  assert.match(surfaceScript, /receipt_closest_candidate/);
  assert.match(surfaceScript, /editor_write_bridge/);
  assert.match(surfaceScript, /editor_write_bridge_summary/);
  assert.match(surfaceScript, /editor_write_bridge_schema/);
  assert.match(surfaceScript, /editor_write_bridge_fixture/);
  assert.match(surfaceScript, /editor_write_bridge_guards/);
  assert.match(surfaceScript, /editor_write_bridge_native_handlers/);
  assert.match(surfaceScript, /editor_write_bridge_handler_capabilities/);
  assert.match(surfaceScript, /editor_write_bridge_reason/);
  assert.match(surfaceScript, /source_apply_contract_schema/);
  assert.match(surfaceScript, /source_apply_contract_source/);
  assert.match(surfaceScript, /source_apply_contract_version/);
  assert.match(surfaceScript, /source_apply_contract_scope/);
  assert.match(surfaceScript, /source_apply_ipc_kind/);
  assert.match(surfaceScript, /source_apply_receipt_schema/);
  assert.match(surfaceScript, /source_apply_context_schema/);
  assert.match(surfaceScript, /group_context_contract_schema/);
  assert.match(surfaceScript, /group_context_contract_source/);
  assert.match(surfaceScript, /group_context_max_alias_bytes/);
  assert.match(surfaceScript, /group_context_max_utility_count/);
  assert.match(surfaceScript, /group_context_candidate_min_utility_count/);
  assert.match(surfaceScript, /reverse_css_delta_contract_schema/);
  assert.match(surfaceScript, /reverse_css_delta_contract_source/);
  assert.match(surfaceScript, /reverse_css_delta_supported_properties/);
  assert.match(surfaceScript, /reverse_css_delta_required_guards/);
  assert.match(surfaceScript, /reverse_css_delta_required_provenance_fields/);
  assert.match(surfaceScript, /reverse_css_delta_contract_diagnostics/);
  assert.match(surfaceScript, /reverse_css_delta_contract_diagnostic/);
  assert.match(surfaceScript, /reverse_css_delta_preview_provenance_diagnostics/);
  assert.match(surfaceScript, /reverse_css_delta_preview_provenance_diagnostic/);
  assert.match(surfaceScript, /reverse_css_delta_example_target/);
  assert.match(surfaceScript, /reverse_css_delta_live_status/);
  assert.match(surfaceScript, /reverse_css_delta_live_target/);
  assert.match(surfaceScript, /reverse_css_delta_live_source/);
  assert.match(surfaceScript, /reverse_css_delta_live_group_alias/);
  assert.match(surfaceScript, /reverse_css_delta_live_group_registry_receipt/);
  assert.match(surfaceScript, /reverse_css_delta_live_reverse_map_status/);
  assert.match(surfaceScript, /group_alias/);
  assert.match(surfaceScript, /group_expansion_status/);
  assert.match(surfaceScript, /group_registry_receipt/);
  assert.match(surfaceScript, /reverse_css_map_receipt/);
  assert.match(surfaceScript, /reverse_css_map_status/);
  assert.match(surfaceScript, /Reverse CSS map/);
  assert.match(surfaceScript, /Reverse CSS status/);
  assert.match(surfaceScript, /Reverse CSS delta contract/);
  assert.match(surfaceScript, /Supported declaration deltas/);
  assert.match(surfaceScript, /entry\.value_strategy \|\| "design_token_suffix"/);
  assert.match(surfaceScript, /Required preview provenance/);
  assert.match(surfaceScript, /Contract diagnostics/);
  assert.match(surfaceScript, /Preview provenance diagnostics/);
  assert.match(surfaceScript, /group_candidate_token_count/);
  assert.match(surfaceScript, /group_source_state/);
  assert.match(surfaceScript, /source_apply_mutation_enabled/);
  assert.match(surfaceScript, /source_apply_required_handler_capabilities/);
  assert.match(surfaceScript, /source_apply_review_context_kinds/);
  assert.match(surfaceScript, /source_apply_mutation_context_kinds/);
  assert.match(surfaceScript, /source_apply_review_receipt_fields/);
  assert.match(surfaceScript, /css_declaration_dry_run_contract_schema/);
  assert.match(surfaceScript, /css_declaration_dry_run_contract_source/);
  assert.match(surfaceScript, /css_declaration_dry_run_required_context_fields/);
  assert.match(surfaceScript, /css_declaration_dry_run_accepted_safety/);
  assert.match(surfaceCssDeclarationDryRunScript, /const cssDeclarationDryRunReviewReceiptFields = Array\.isArray\(cssDeclarationDryRunContract\.review_receipt_fields\)/);
  assert.match(surfaceScript, /dry_run_receipt_schema: cssDeclarationDryRunContract\.dry_run_receipt_schema \|\| null/);
  assert.match(surfaceScript, /review_receipt_fields: cssDeclarationDryRunReviewReceiptFields/);
  assert.match(surfaceScript, /CSS review receipt fields/);
  assert.match(surfaceScript, /css_declaration_dry_run_review_receipt_fields/);
  assert.match(surfaceScript, /css_declaration_dry_run_max_declaration_bytes/);
  assert.match(surfaceCssDeclarationDryRunScript, /const cssDeclarationDryRunMaxDiagnosticCount = Number\(cssDeclarationDryRunContract\.max_diagnostic_count/);
  assert.match(surfaceCssDeclarationDryRunScript, /const cssDeclarationDryRunMaxDiagnosticBytes = Number\(cssDeclarationDryRunContract\.max_diagnostic_bytes/);
  assert.match(surfaceCssDeclarationDryRunScript, /function cssDeclarationDryRunDiagnosticLimitDiagnostics\(diagnostics, prefix\)/);
  assert.match(surfaceScript, /css_declaration_dry_run_max_diagnostic_count/);
  assert.match(surfaceScript, /css_declaration_dry_run_max_diagnostic_bytes/);
  assert.match(surfaceScript, /Diagnostic max/);
  assert.match(surfaceScript, /css_declaration_dry_run_max_source_path_bytes/);
  assert.match(surfaceScript, /css_declaration_dry_run_max_source_span_bytes/);
  assert.match(surfaceScript, /css_declaration_dry_run_max_source_digest_bytes/);
  assert.match(surfaceScript, /Source limits/);
  assert.match(surfaceScript, /css_declaration_dry_run_diagnostics/);
  assert.match(surfaceScript, /css_declaration_dry_run_preview_diagnostics/);
  assert.match(surfaceScript, /css_declaration_dry_run_preview_status/);
  assert.match(surfaceScript, /css_declaration_dry_run_preview_declaration/);
  assert.match(surfaceScript, /CSS declaration source review is gated by the DX Style dry-run contract/);
  assert.match(surfaceScript, /const sourceApplyReviewReceiptFields = Array\.isArray\(sourceApplyContract\.review_receipt_fields\)/);
  assert.match(surfaceScript, /Review context kinds/);
  assert.match(surfaceScript, /Mutation context kinds/);
  assert.match(surfaceScript, /Write readiness/);
  assert.match(surfaceScript, /Write blocker/);
  assert.match(surfaceScript, /Write gaps/);
  assert.match(surfaceScript, /Review receipt fields/);
  assert.match(surfaceScript, /const sourceDigestPrefix = "fnv1a64:"/);
  assert.match(surfaceScript, /function sourceDigestReady\(context\)/);
  assert.match(surfaceScript, /missing_or_invalid_source_digest/);
  assert.match(surfaceScript, /source_digest_exceeds_contract_limit/);
  assert.match(surfaceScript, /source_apply_max_source_path_bytes/);
  assert.match(surfaceScript, /source_apply_max_class_name_bytes/);
  assert.match(surfaceScript, /source_apply_max_css_bytes/);
  assert.match(surfaceScript, /source_apply_max_generator_id_bytes/);
  assert.match(surfaceScript, /source_apply_max_source_span_bytes/);
  assert.match(surfaceScript, /source_apply_max_source_digest_bytes/);
  assert.match(surfaceScript, /source_apply_payload_diagnostics/);
  assert.match(surfaceScript, /source_apply_payload_diagnostic/);
  assert.match(surfaceScript, /closest_candidate/);
  assert.match(surfaceScript, /dry_run_receipt/);
  assert.match(surfaceScript, /receipt_intent/);
  assert.match(surfaceScript, /receipt_edits/);
  assert.match(surfaceScript, /receipt_edit_previews/);
  assert.match(surfaceScript, /summary\.edits/);
  assert.match(surface, /Source apply is gated by trusted spans/);
  assert.match(surfaceScript, /disabled_until_trusted_grouped_class_source_span_and_dry_run_receipt/);
  assert.match(webPreviewCargo, /zed_actions\.workspace = true/);
  assert.match(rail, /OpenGeneratorPreview/);
  assert.match(rail, /window\.dispatch_action\(OpenGeneratorPreview\.boxed_clone\(\), cx\)/);
  assert.match(rail, /disabled\(!snapshot\.web_preview_bridge_ready\)/);
});

test("DX Style has a real right-dock GPUI shell", () => {
  const root = read("crates/agent_ui/src/dx_style_panel.rs");
  const applyGate = read("crates/agent_ui/src/dx_style_panel/apply_gate.rs");
  const cssCursorContext = read(
    "crates/agent_ui/src/dx_style_panel/css_cursor_context.rs",
  );
  const cssHintCatalog = read("crates/agent_ui/src/dx_style_panel/css_hint_catalog.rs");
  const cssHintGenerated = JSON.parse(
    read("crates/agent_ui/src/dx_style_panel/css-declaration-hint-catalog.generated.json"),
  );
  const cssHintStyleFixture = JSON.parse(
    readStyle("fixtures/visual-generator-css-declaration-hint-catalog.json"),
  );
  const cursorContext = read("crates/agent_ui/src/dx_style_panel/cursor_context.rs");
  const cursorContextTokens = read(
    "crates/agent_ui/src/dx_style_panel/cursor_context_tokens.rs",
  );
  const groupContext = read("crates/agent_ui/src/dx_style_panel/group_context.rs");
  const groupRegistry = read("crates/agent_ui/src/dx_style_panel/group_registry.rs");
  const receiptRoots = read("crates/agent_ui/src/dx_style_panel/receipt_roots.rs");
  const reverseCssMap = read("crates/agent_ui/src/dx_style_panel/reverse_css_map.rs");
  const editorWriteBridge = read(
    "crates/agent_ui/src/dx_style_panel/editor_write_bridge.rs",
  );
  const receiptMatch = read("crates/agent_ui/src/dx_style_panel/receipt_match.rs");
  const receiptReview = read("crates/agent_ui/src/dx_style_panel/receipt_review.rs");
  const sourceDigest = read("crates/agent_ui/src/dx_style_panel/source_digest.rs");
  const activeContext = read("crates/agent_ui/src/dx_style_panel/active_context.rs");
  const panel = read("crates/agent_ui/src/dx_style_panel/panel.rs");
  const panelMetric = read("crates/agent_ui/src/dx_style_panel/panel_metric.rs");
  const panelView = read("crates/agent_ui/src/dx_style_panel/panel_view.rs");
  const surfaceScript = read("crates/web_preview/src/dx_style_generator_surface/script.rs");
  const init = read("crates/agent_ui/src/agent_ui.rs");

  assert.match(root, /pub\(crate\) mod panel/);
  assert.match(root, /mod apply_gate/);
  assert.match(root, /mod active_context/);
  assert.match(root, /mod css_cursor_context/);
  assert.match(root, /mod css_hint_catalog/);
  assert.match(root, /mod cursor_context/);
  assert.match(root, /mod cursor_context_tokens/);
  assert.match(root, /mod group_context/);
  assert.match(root, /mod group_registry/);
  assert.match(root, /mod editor_write_bridge/);
  assert.match(root, /mod panel_view/);
  assert.match(root, /mod panel_metric/);
  assert.match(root, /mod receipt_roots/);
  assert.match(root, /mod receipt_match/);
  assert.match(root, /mod receipt_review/);
  assert.match(root, /mod reverse_css_map/);
  assert.match(root, /mod source_digest/);
  assert.match(root, /grouped_class_reverse_css_map\.rs/);
  assert.match(root, /grouped_class_reverse_css_delta\.rs/);
  assert.match(root, /GROUPED_CLASS_REVERSE_CSS_MAP_SCHEMA/);
  assert.match(root, /GROUPED_CLASS_REVERSE_CSS_DELTA_SCHEMA/);
  assert.match(root, /Reverse CSS Map/);
  assert.match(root, /Reverse CSS Delta/);
  assert.match(root, /review-only contract/);
  assert.match(init, /dx_style_panel::panel::init\(cx\)/);
  assert.match(panel, /impl Panel for DxStylePanel/);
  assert.match(panel, /DockPosition::Right/);
  assert.match(panel, /position_is_valid/);
  assert.match(panel, /TogglePanel\.boxed_clone\(\)/);
  assert.match(panel, /workspace\.add_panel\(panel, window, cx\)/);
  assert.match(panel, /workspace\.toggle_panel_focus::<DxStylePanel>/);
  assert.match(panel, /dx_style_panel_snapshot\(\)/);
  assert.match(panel, /WeakEntity<Workspace>/);
  assert.match(panel, /active_style_context/);
  assert.match(activeContext, /active_item\(cx\)/);
  assert.match(activeContext, /project_path\(cx\)/);
  assert.match(activeContext, /PathStyle::local\(\)/);
  assert.match(activeContext, /absolute_path\(&project_path, cx\)/);
  assert.match(activeContext, /get_workspace_root\(&project_path, cx\)/);
  assert.match(activeContext, /workspace_root: Option<String>/);
  assert.match(activeContext, /"workspace_root": self\.workspace_root/);
  assert.match(activeContext, /editor\.buffer\(\)\.read\(cx\)\.len\(cx\)\.0/);
  assert.ok(
    activeContext.indexOf("let source_len = editor.buffer().read(cx).len(cx).0") <
      activeContext.indexOf("let source = editor.text(cx)"),
  );
  assert.match(cursorContext, /is_style_bearing_path/);
  assert.match(activeContext, /newest::<MultiBufferOffset>/);
  assert.match(cursorContext, /cursor_style_token/);
  assert.match(activeContext, /ACTIVE_STYLE_CONTEXT_SCHEMA/);
  assert.match(activeContext, /web_preview_context_json/);
  assert.match(activeContext, /source_path: Option<String>/);
  assert.match(activeContext, /context_kind: Option<String>/);
  assert.match(activeContext, /css_property: Option<String>/);
  assert.match(activeContext, /css_generator: Option<String>/);
  assert.match(activeContext, /css_source_edit_safety: Option<String>/);
  assert.match(activeContext, /attribute_tokens: Vec<String>/);
  assert.match(activeContext, /group_context: ActiveGroupContext/);
  assert.match(activeContext, /span_start: Option<usize>/);
  assert.match(activeContext, /span_end: Option<usize>/);
  assert.match(activeContext, /with_source_path/);
  assert.match(activeContext, /"source_path": self\.source_path/);
  assert.match(activeContext, /"context_kind": self\.context_kind/);
  assert.match(activeContext, /"css_property": self\.css_property/);
  assert.match(activeContext, /"css_generator": self\.css_generator/);
  assert.match(activeContext, /"css_source_edit_safety": self\.css_source_edit_safety/);
  assert.match(activeContext, /"attribute_tokens": self\.attribute_tokens/);
  assert.match(activeContext, /"group_context": self\.group_context\.to_json\(\)/);
  assert.match(activeContext, /source_span_json/);
  assert.match(activeContext, /"source_span": self\.source_span_json\(\)/);
  assert.match(activeContext, /"start_byte": self\.span_start\?/);
  assert.match(activeContext, /"end_byte": self\.span_end\?/);
  assert.match(activeContext, /span_byte_range/);
  assert.match(activeContext, /can_open_generator/);
  assert.match(activeContext, /"workspace unavailable" \| "no active file" \| "non-style file"/);
  assert.match(activeContext, /source_digest::active_source_digest/);
  assert.match(activeContext, /source_len_bytes: Option<usize>/);
  assert.match(activeContext, /fn with_source_len/);
  assert.match(activeContext, /"source_len_bytes": self\.source_len_bytes/);
  assert.match(activeContext, /\.with_source_len\(source_len\)/);
  assert.match(activeContext, /source\.len\(\)/);
  assert.doesNotMatch(
    activeContext,
    /let source_digest = active_source_digest\(&source\);\s*match cursor_style_token/s,
  );
  assert.match(sourceDigest, /active_source_digest/);
  assert.match(sourceDigest, /DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_ALGORITHM/);
  assert.match(sourceDigest, /DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_PREFIX/);
  assert.match(cssCursorContext, /pub\(super\) fn css_style_hint/);
  assert.match(cssCursorContext, /pub\(super\) fn is_css_style_sheet_path/);
  assert.match(cssCursorContext, /css_declaration_generator_hint/);
  assert.doesNotMatch(cssCursorContext, /linear-gradient|box-shadow|clip-path|grid-cols|transform-3d/);
  assert.match(cssHintCatalog, /DX_STYLE_CSS_HINT_CATALOG_SCHEMA/);
  assert.match(cssHintCatalog, /css-declaration-hint-catalog\.generated\.json/);
  assert.match(cssHintCatalog, /OnceLock<Vec<CssHintEntry>>/);
  assert.match(cssHintCatalog, /entry_count/);
  assert.match(cssHintCatalog, /source_edit_safety/);
  assert.match(cssHintCatalog, /css_declaration_generator_hint/);
  assert.match(cssHintCatalog, /property_matches/);
  assert.match(cssHintCatalog, /value_matches/);
  assert.deepEqual(cssHintGenerated, cssHintStyleFixture);
  assert.match(activeContext, /css_style_hint/);
  assert.match(activeContext, /CSS declaration generator hint is read-only/);
  assert.match(activeContext, /"css_declaration"/);
  assert.match(cursorContext, /attribute_tokens: Vec<String>/);
  assert.match(cursorContext, /tokens_in_value/);
  assert.match(cursorContextTokens, /CURSOR_ATTRIBUTE_TOKEN_LIMIT: usize = 32/);
  assert.match(cursorContextTokens, /CURSOR_ATTRIBUTE_TOKEN_MAX_BYTES: usize = 256/);
  assert.match(cursorContextTokens, /pub\(super\) fn tokens_in_value/);
  assert.match(cursorContextTokens, /bracket_depth/);
  assert.match(cursorContextTokens, /paren_depth/);
  assert.match(groupContext, /GROUP_CONTEXT_SCHEMA/);
  assert.match(groupContext, /zed\.dx_style\.group_context\.v1/);
  assert.match(groupContext, /GROUP_CONTEXT_MAX_ALIAS_BYTES: usize = 128/);
  assert.match(groupContext, /GROUP_CONTEXT_MAX_UTILITY_COUNT: usize = 32/);
  assert.match(groupContext, /GROUP_CONTEXT_MAX_UTILITY_BYTES: usize = 256/);
  assert.match(groupContext, /GROUP_CONTEXT_CANDIDATE_MIN_UTILITY_COUNT: usize = 4/);
  assert.match(groupContext, /from_tokens/);
  assert.match(groupContext, /registry_group_entry/);
  assert.match(groupContext, /source_path: Option<&str>/);
  assert.match(groupContext, /registry_group_entry\(alias, source_path, workspace_root\)/);
  assert.match(groupContext, /alias_reference_expanded/);
  assert.match(groupContext, /registry_receipt_expansion_available/);
  assert.match(groupContext, /registry_receipt/);
  assert.match(groupContext, /reverse_css_map_receipt/);
  assert.match(groupContext, /reverse_css_map_status/);
  assert.match(groupContext, /reverse_css_map_summary/);
  assert.match(groupContext, /group_call_context/);
  assert.match(groupContext, /parse_group_call/);
  assert.match(groupContext, /looks_like_atomic_utility/);
  assert.match(groupContext, /utility\.contains\('-'\)/);
  assert.match(groupContext, /needs_project_group_contract/);
  assert.match(groupContext, /candidate_requires_project_repetition_scan/);
  assert.match(groupContext, /source-owned grouping analysis/);
  assert.match(groupRegistry, /DX_STYLE_GROUP_REGISTRY_RECEIPT_SCHEMA/);
  assert.match(groupRegistry, /dx\.style\.grouped-class-registry-receipt/);
  assert.match(groupRegistry, /DX_STYLE_REVERSE_CSS_MAP_RECEIPT_FILE/);
  assert.match(groupRegistry, /grouped-class-reverse-css-map-latest\.json/);
  assert.doesNotMatch(groupRegistry, /DX_STYLE_PROJECT_REGISTRY_RECEIPT_ROOT/);
  assert.doesNotMatch(groupRegistry, /DX_STYLE_HUB_REGISTRY_RECEIPT_ROOT/);
  assert.match(groupRegistry, /registry_receipt_roots/);
  assert.match(groupRegistry, /active_style_receipt_roots/);
  assert.match(groupRegistry, /source_path: Option<&str>/);
  assert.match(groupRegistry, /workspace_root: Option<&str>/);
  assert.doesNotMatch(groupRegistry, /Path::ancestors/);
  assert.match(receiptRoots, /PROJECT_RECEIPT_ANCESTOR_LIMIT: usize = 8/);
  assert.match(receiptRoots, /active_style_receipt_roots/);
  assert.match(receiptRoots, /path\.is_absolute\(\)/);
  assert.match(receiptRoots, /workspace_root/);
  assert.match(receiptRoots, /source_path\.starts_with\(root\)/);
  assert.match(receiptRoots, /Path::ancestors/);
  assert.match(receiptRoots, /if !ancestor\.starts_with\(workspace_root\)/);
  assert.match(receiptRoots, /if ancestor == workspace_root/);
  assert.match(receiptRoots, /\.dx"\)\.join\("receipts"\)\.join\("style"\)/);
  assert.match(receiptRoots, /receipt_root_key/);
  assert.match(receiptRoots, /replace/);
  assert.match(receiptRoots, /to_ascii_lowercase/);
  assert.match(groupRegistry, /cache_key/);
  assert.match(groupRegistry, /GROUP_REGISTRY_CACHE_TTL/);
  assert.match(groupRegistry, /MAX_GROUP_REGISTRY_RECEIPT_BYTES: u64 = 128 \* 1024/);
  assert.match(groupRegistry, /GROUP_REGISTRY_RECEIPT_SCAN_LIMIT: usize = 64/);
  assert.match(groupRegistry, /find_map\(trusted_registry_entries_from_path\)/);
  assert.match(groupRegistry, /Option<Vec<RegistryGroupEntry>>/);
  assert.match(groupRegistry, /reverse_css_map_receipt_for/);
  assert.match(groupRegistry, /registry_entries_verified/);
  assert.match(groupRegistry, /source_owned/);
  assert.doesNotMatch(groupRegistry, /flat_map\(trusted_registry_entries_from_path\)/);
  assert.doesNotMatch(groupRegistry, /Command::new|spawn|powershell|cmd \/c/);
  assert.match(reverseCssMap, /DX_STYLE_REVERSE_CSS_MAP_SCHEMA/);
  assert.match(reverseCssMap, /dx\.style\.grouped-class-reverse-css-map/);
  assert.match(reverseCssMap, /MAX_REVERSE_CSS_MAP_RECEIPT_BYTES: u64 = 128 \* 1024/);
  assert.match(reverseCssMap, /source_mutation_enabled/);
  assert.match(reverseCssMap, /editor_write_bridge_required/);
  assert.match(reverseCssMap, /reverse_status/);
  assert.doesNotMatch(reverseCssMap, /Command::new|spawn|powershell|cmd \/c/);
  assert.match(activeContext, /with_attribute_tokens[\s\S]*source_digest: String/);
  assert.match(activeContext, /ActiveGroupContext::from_tokens[\s\S]*Some\(source_path\)[\s\S]*workspace_root/);
  assert.match(activeContext, /StyleApplyGateInput/);
  assert.match(activeContext, /style_apply_gate/);
  assert.match(activeContext, /apply_gate\.to_json/);
  assert.match(activeContext, /source_digest/);
  assert.match(receiptReview, /GROUPED_CLASS_DRY_RUN_RECEIPT_SCHEMA/);
  assert.match(applyGate, /style_apply_gate/);
  assert.match(applyGate, /style_editor_write_bridge_snapshot/);
  assert.match(applyGate, /active_style_receipt_roots/);
  assert.match(applyGate, /trusted_dry_run_receipts\(input\.source_path, input\.workspace_root\)/);
  assert.match(applyGate, /input\.workspace_root/);
  assert.match(applyGate, /trusted_dry_run_receipts\(\s*source_path: &str,\s*workspace_root: Option<&str>,/s);
  assert.doesNotMatch(applyGate, /DX_STYLE_PROJECT_RECEIPT_ROOT|DX_STYLE_HUB_RECEIPT_ROOT/);
  assert.match(applyGate, /latest_matching_trusted_dry_run_receipt/);
  assert.match(applyGate, /paths\.sort_by/);
  assert.match(applyGate, /receipt_modified/);
  assert.match(applyGate, /needs_matching_active_source_receipt/);
  assert.match(applyGate, /receipt_match/);
  assert.match(applyGate, /receipt_mismatch_summary/);
  assert.match(receiptMatch, /StyleReceiptMismatchSummary/);
  assert.match(receiptMatch, /StyleReceiptCandidateSummary/);
  assert.match(receiptMatch, /receipt_matches_active_source/);
  assert.match(receiptMatch, /receipt_mismatch_summary/);
  assert.match(receiptMatch, /closest_receipt_candidate/);
  assert.match(receiptMatch, /receipt_match_score/);
  assert.match(receiptMatch, /source path mismatch/);
  assert.match(receiptMatch, /cursor token span mismatch/);
  assert.match(receiptMatch, /source digest mismatch/);
  assert.match(applyGate, /StyleDryRunReceiptSummary/);
  assert.match(receiptReview, /trusted_receipt/);
  assert.match(receiptReview, /receipt_source_digest/);
  assert.match(receiptReview, /source_digest_algorithm/);
  assert.match(receiptReview, /DX_STYLE_GROUPED_CLASS_SOURCE_DIGEST_ALGORITHM/);
  assert.match(receiptReview, /source_digest/);
  assert.match(receiptReview, /receipt_summary/);
  assert.match(receiptReview, /patch_preview\/intent/);
  assert.match(receiptReview, /patch_preview\/edits/);
  assert.match(receiptReview, /receipt_edit_summaries/);
  assert.match(receiptReview, /StyleDryRunEditPreview/);
  assert.match(receiptReview, /edit_previews/);
  assert.match(receiptReview, /receipt_edit_previews/);
  assert.match(receiptReview, /start_byte/);
  assert.match(receiptReview, /end_byte/);
  assert.match(receiptReview, /MAX_DRY_RUN_REPLACEMENT_TEXT_BYTES: usize = 4096/);
  assert.match(receiptReview, /replacement_text/);
  assert.match(receiptReview, /replacement_text\.is_empty\(\)/);
  assert.match(receiptReview, /replacement_text\.len\(\) > MAX_DRY_RUN_REPLACEMENT_TEXT_BYTES/);
  assert.match(receiptReview, /DRY_RUN_EDIT_SUMMARY_LIMIT/);
  assert.match(receiptReview, /source_digest_verified/);
  assert.match(receiptReview, /source_span_trusted/);
  assert.match(receiptReview, /dry_run_preview_ready/);
  assert.match(editorWriteBridge, /StyleEditorWriteBridgeSnapshot/);
  assert.match(editorWriteBridge, /state: "not_enabled"/);
  assert.match(editorWriteBridge, /preflight_schema_version/);
  assert.match(editorWriteBridge, /preflight_scope/);
  assert.match(editorWriteBridge, /can_mutate_source/);
  assert.match(editorWriteBridge, /summary: format!/);
  assert.match(editorWriteBridge, /preflight_fixture_path/);
  assert.match(editorWriteBridge, /read_preflight_fixture/);
  assert.match(editorWriteBridge, /MAX_EDITOR_WRITE_BRIDGE_PREFLIGHT_BYTES/);
  assert.match(editorWriteBridge, /PREFLIGHT_LIST_LIMIT/);
  assert.match(editorWriteBridge, /string_list/);
  assert.match(editorWriteBridge, /fallback_preflight/);
  assert.match(editorWriteBridge, /required_editor_guards/);
  assert.match(editorWriteBridge, /required_native_handlers/);
  assert.match(editorWriteBridge, /required_native_handler_capabilities/);
  assert.match(editorWriteBridge, /same-session native editor identity/);
  assert.match(editorWriteBridge, /cursor-scoped dry-run structured edit preview/);
  assert.match(editorWriteBridge, /authorized runtime validation/);
  assert.match(editorWriteBridge, /zed\.web_preview\.dx_style_source_apply_receipt\.v1/);
  assert.deepEqual(
    rustStringVec(editorWriteBridge, "required_receipts"),
    expectedEditorWriteBridgeReceipts,
  );
  assert.deepEqual(
    rustStringVec(editorWriteBridge, "required_editor_guards"),
    expectedEditorWriteBridgeGuards,
  );
  assert.deepEqual(
    rustStringVec(editorWriteBridge, "required_native_handlers"),
    expectedEditorWriteBridgeHandlers,
  );
  assert.deepEqual(
    rustStringVec(editorWriteBridge, "required_native_handler_capabilities"),
    expectedEditorWriteBridgeCapabilities,
  );
  assert.match(editorWriteBridge, /window\.__DX_STYLE_SOURCE_APPLY__/);
  assert.match(editorWriteBridge, /can_mutate_source/);
  assert.match(editorWriteBridge, /can_apply: false/);
  assert.match(editorWriteBridge, /grouped-class-editor-write-bridge-preflight/);
  assert.match(editorWriteBridge, /runtime validation/);
  assert.doesNotMatch(editorWriteBridge, /bounded edit preview review/);
  assert.doesNotMatch(applyGate, /Command::new|spawn|powershell|cmd \/c/);
  assert.doesNotMatch(editorWriteBridge, /Command::new|spawn|powershell|cmd \/c/);
  assert.match(activeContext, /disabled_until_trusted_grouped_class_source_span_and_dry_run_receipt/);
  assert.match(panel, /panel_view::render_panel/);
  assert.match(panelMetric, /pub\(super\) fn metric/);
  assert.match(panelMetric, /max_w\(px\(190\.0\)\)/);
  assert.match(panelView, /OpenGeneratorPreviewForContext/);
  assert.match(panelView, /source_context_json/);
  assert.match(panelView, /STYLE_PANEL_ROW_LIMIT: usize = 13/);
  assert.match(panelView, /take\(STYLE_PANEL_ROW_LIMIT\)/);
  assert.match(panelView, /can_open_generator/);
  assert.match(panelView, /active_context\.can_open_generator\(\)/);
  assert.match(panelView, /Receipt/);
  assert.match(panelView, /Review/);
  assert.match(panelView, /Match/);
  assert.match(panelView, /Bridge/);
  assert.match(panelView, /CSS/);
  assert.match(panelView, /CSS safety/);
  assert.match(panelView, /Class list/);
  assert.match(panelView, /Kind/);
  assert.match(panelView, /Group/);
  assert.match(panelView, /Path/);
  assert.match(panelView, /Span bytes/);
  assert.match(panelView, /span_byte_range/);
  assert.match(panelView, /editor_write_bridge\.summary/);
  assert.match(panelView, /Mismatch/);
  assert.match(panelView, /receipt\.edit_count/);
  assert.match(panelView, /receipt\.edits\.first\(\)/);
  assert.match(surfaceScript, /Structured edit previews/);
  assert.match(surfaceScript, /summary\.edit_previews/);
  assert.match(panelView, /disabled\(!can_open_generator\)/);
  assert.doesNotMatch(panelView, /disabled\(!snapshot\.web_preview_bridge_ready\)/);
  assert.doesNotMatch(panel, /web_preview::|WebPreviewView/);
  assert.doesNotMatch(panelView, /web_preview::|WebPreviewView/);
  assert.ok(
    lineCount("crates/web_preview/src/dx_style_generator_surface/source_apply_session_script.rs") <
      90,
  );
  assert.ok(
    lineCount("crates/web_preview/src/dx_style_generator_surface/css_declaration_dry_run_script.rs") <
      220,
  );
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/panel.rs") < 230);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/apply_gate.rs") < 260);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/css_cursor_context.rs") < 90);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/css_hint_catalog.rs") < 120);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/cursor_context.rs") < 260);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/cursor_context_tokens.rs") < 100);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/group_context.rs") < 210);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/group_registry.rs") < 220);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/receipt_roots.rs") < 90);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/reverse_css_map.rs") < 120);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/source_digest.rs") < 50);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/receipt_match.rs") < 180);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/receipt_review.rs") < 260);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/active_context.rs") < 360);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/panel_metric.rs") < 60);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/panel_view.rs") < 200);
});

test("Zed Style rail surfaces source-only DX Style readiness", () => {
  const panel = read("crates/agent_ui/src/dx_style_panel.rs");
  const readiness = read("crates/agent_ui/src/dx_style_panel/readiness.rs");
  const readinessExpectedFiles = read(
    "crates/agent_ui/src/dx_style_panel/readiness/expected_files.rs",
  );
  const rail = read("crates/agent_ui/src/dx_launch_workspace/style_panel.rs");
  const panelView = read("crates/agent_ui/src/dx_style_panel/panel_view.rs");

  assert.match(panel, /^mod readiness;$/m);
  assert.match(panel, /DxStyleReadinessSnapshot/);
  assert.match(panel, /dx_style_readiness_snapshot\(&root, root_exists\)/);
  assert.match(panel, /pub readiness: DxStyleReadinessSnapshot/);

  assert.match(readiness, /const DX_STYLE_HUB_RECEIPT_ROOT: &str = r"G:\\Dx\\.dx\\receipts\\style";/);
  assert.match(readiness, /mod expected_files/);
  assert.match(readinessExpectedFiles, /EXPECTED_STYLE_FILES/);
  assert.match(readinessExpectedFiles, /GROUPED_CLASS_CONTRACT_SCHEMA/);
  assert.match(readinessExpectedFiles, /GROUPED_CLASS_CURSOR_CONTEXT_SCHEMA/);
  assert.match(readinessExpectedFiles, /GROUPED_CLASS_DRY_RUN_RECEIPT_SCHEMA/);
  assert.match(readinessExpectedFiles, /GROUPED_CLASS_EDITOR_WRITE_BRIDGE_SCHEMA/);
  assert.match(readinessExpectedFiles, /GROUPED_CLASS_SOURCE_APPLY_CONTRACT_SCHEMA/);
  assert.match(readinessExpectedFiles, /GROUPED_CLASS_WEB_PREVIEW_CONTEXT_SCHEMA/);
  assert.match(readinessExpectedFiles, /GROUPED_CLASS_REGISTRY_RECEIPT_SCHEMA/);
  assert.match(readinessExpectedFiles, /GROUPED_CLASS_REVERSE_CSS_MAP_SCHEMA/);
  assert.match(readinessExpectedFiles, /GROUPED_CLASS_REVERSE_CSS_DELTA_SCHEMA/);
  assert.match(readinessExpectedFiles, /VISUAL_GENERATOR_RECIPE_CATALOG_SCHEMA/);
  assert.match(readinessExpectedFiles, /VISUAL_GENERATOR_CONTROL_CATALOG_SCHEMA/);
  assert.match(readinessExpectedFiles, /VISUAL_GENERATOR_CSS_HINT_CATALOG_SCHEMA/);
  assert.match(readinessExpectedFiles, /CSS_DECLARATION_DRY_RUN_CONTRACT_SCHEMA/);
  assert.match(readinessExpectedFiles, /visual-generator-catalog\.json/);
  assert.match(readinessExpectedFiles, /grouped-class-editor-write-bridge-preflight\.json/);
  assert.match(readinessExpectedFiles, /grouped-class-source-apply-contract\.json/);
  assert.match(readinessExpectedFiles, /grouped-class-web-preview-context\.json/);
  assert.match(readinessExpectedFiles, /grouped-class-registry-receipt\.json/);
  assert.match(readinessExpectedFiles, /grouped-class-reverse-css-map\.json/);
  assert.match(readinessExpectedFiles, /grouped-class-reverse-css-delta-contract\.json/);
  assert.match(readinessExpectedFiles, /visual-generator-recipe-catalog\.json/);
  assert.match(readinessExpectedFiles, /visual-generator-control-catalog\.json/);
  assert.match(readinessExpectedFiles, /visual-generator-css-declaration-hint-catalog\.json/);
  assert.match(readinessExpectedFiles, /css-declaration-dry-run-contract\.json/);
  assert.match(readinessExpectedFiles, /TAILWIND_POSTCSS_BROWSER_COMPAT_SCHEMA/);
  assert.match(readinessExpectedFiles, /TAILWIND_V43_CSS_DIRECTIVE_LEDGER_SCHEMA/);
  assert.match(readinessExpectedFiles, /REGISTRY_SNAPSHOT_PATH/);
  assert.match(readiness, /"not-run"/);
  assert.match(readiness, /Generate governed DX Style receipts before enabling mutation controls/);
  assert.match(readiness, /receipt_root_row/);
  assert.doesNotMatch(readiness, /std::process|Command::new|spawn|powershell|cmd \/c/);
  assert.doesNotMatch(readinessExpectedFiles, /std::process|Command::new|spawn|powershell|cmd \/c/);

  assert.match(rail, /metric_row\("Readiness", snapshot\.readiness\.status\.clone\(\)\)/);
  assert.match(rail, /No dx style build\/check receipt has been read by Zed/);
  assert.match(rail, /bounded_items\(\s*&snapshot\.readiness\.missing_rows/s);
  assert.match(
    rail,
    /Button::new\(\s*"dx-style-open-generator-preview",\s*"Open Web Preview Generators",\s*\)/s,
  );
  assert.match(panelView, /"Open Web Preview Generators"/);
  assert.doesNotMatch(rail, /IconButton::new/);
  assert.ok(lineCount("crates/agent_ui/src/dx_style_panel/readiness.rs") < 380);
  assert.ok(
    lineCount("crates/agent_ui/src/dx_style_panel/readiness/expected_files.rs") < 280,
  );
  assert.ok(lineCount("crates/agent_ui/src/dx_launch_workspace/style_panel.rs") < 230);
});
