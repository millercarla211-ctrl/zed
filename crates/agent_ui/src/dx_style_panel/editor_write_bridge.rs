use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

#[derive(Clone)]
pub(super) struct StyleEditorWriteBridgeSnapshot {
    pub(super) state: String,
    pub(super) summary: String,
    pub(super) reason: String,
    pub(super) preflight_schema: String,
    pub(super) preflight_schema_version: u64,
    pub(super) preflight_scope: String,
    pub(super) preflight_fixture_path: String,
    pub(super) can_mutate_source: bool,
    pub(super) required_receipts: Vec<String>,
    pub(super) required_editor_guards: Vec<String>,
    pub(super) required_native_handlers: Vec<String>,
    pub(super) required_native_handler_capabilities: Vec<String>,
    pub(super) required_source_apply_review_receipt_fields: Vec<String>,
    pub(super) required_runtime_proofs: Vec<String>,
    pub(super) runtime_validation_receipt_schema: String,
    pub(super) required_runtime_validation_receipt_fields: Vec<String>,
    pub(super) mutation_write_receipt_schema: String,
    pub(super) required_mutation_write_receipt_fields: Vec<String>,
    pub(super) runtime_validation_required: bool,
    pub(super) can_apply: bool,
}

impl StyleEditorWriteBridgeSnapshot {
    pub(super) fn to_json(&self) -> Value {
        json!({
            "state": self.state,
            "summary": self.summary,
            "reason": self.reason,
            "preflight_schema": self.preflight_schema,
            "preflight_schema_version": self.preflight_schema_version,
            "preflight_scope": self.preflight_scope,
            "preflight_fixture_path": self.preflight_fixture_path,
            "can_mutate_source": self.can_mutate_source,
            "required_receipts": self.required_receipts,
            "required_editor_guards": self.required_editor_guards,
            "required_native_handlers": self.required_native_handlers,
            "required_native_handler_capabilities": self.required_native_handler_capabilities,
            "required_source_apply_review_receipt_fields": self.required_source_apply_review_receipt_fields,
            "required_runtime_proofs": self.required_runtime_proofs,
            "runtime_validation_receipt_schema": self.runtime_validation_receipt_schema,
            "required_runtime_validation_receipt_fields": self.required_runtime_validation_receipt_fields,
            "mutation_write_receipt_schema": self.mutation_write_receipt_schema,
            "required_mutation_write_receipt_fields": self.required_mutation_write_receipt_fields,
            "runtime_validation_required": self.runtime_validation_required,
            "can_apply": self.can_apply,
        })
    }
}

const GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_SCHEMA: &str =
    "dx.style.grouped-class-editor-write-bridge-preflight";
const GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_FIXTURE: &str =
    r"G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json";
const GENERATED_EDITOR_WRITE_BRIDGE_PREFLIGHT_JSON: &str =
    include_str!("editor-write-bridge-preflight.generated.json");
const MAX_EDITOR_WRITE_BRIDGE_PREFLIGHT_BYTES: u64 = 64 * 1024;
const PREFLIGHT_LIST_LIMIT: usize = 32;

pub(super) fn style_editor_write_bridge_snapshot() -> StyleEditorWriteBridgeSnapshot {
    let preflight_path = PathBuf::from(GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_FIXTURE);
    let preflight = read_preflight_fixture(&preflight_path)
        .or_else(generated_preflight)
        .unwrap_or_else(emergency_preflight);

    StyleEditorWriteBridgeSnapshot {
        state: preflight.state,
        summary: format!(
            "{} receipt(s), {} guard(s), {} native handler(s), {} handler capability(s), {} review field(s), {} runtime proof(s), {} runtime receipt field(s), {} mutation write field(s), runtime validation {}",
            preflight.required_receipts.len(),
            preflight.required_editor_guards.len(),
            preflight.required_native_handlers.len(),
            preflight.required_native_handler_capabilities.len(),
            preflight.required_source_apply_review_receipt_fields.len(),
            preflight.required_runtime_proofs.len(),
            preflight.required_runtime_validation_receipt_fields.len(),
            preflight.required_mutation_write_receipt_fields.len(),
            if preflight.runtime_validation_required {
                "required"
            } else {
                "not required"
            }
        ),
        reason: concat!(
            "dx.style.grouped-class-editor-write-bridge-preflight is source-owned but not enabled. ",
            "Editor source writes require trusted source spans, same-session editor identity, ",
            "cursor-scoped dry-run edit review, complete source-apply review receipt fields, ",
            "explicit user apply, and runtime validation before Apply can mutate files."
        )
        .to_string(),
        preflight_schema: GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_SCHEMA.to_string(),
        preflight_schema_version: preflight.schema_version,
        preflight_scope: preflight.scope,
        preflight_fixture_path: preflight_path.display().to_string(),
        can_mutate_source: preflight.can_mutate_source,
        required_receipts: preflight.required_receipts,
        required_editor_guards: preflight.required_editor_guards,
        required_native_handlers: preflight.required_native_handlers,
        required_native_handler_capabilities: preflight.required_native_handler_capabilities,
        required_source_apply_review_receipt_fields: preflight
            .required_source_apply_review_receipt_fields,
        required_runtime_proofs: preflight.required_runtime_proofs,
        runtime_validation_receipt_schema: preflight.runtime_validation_receipt_schema,
        required_runtime_validation_receipt_fields: preflight
            .required_runtime_validation_receipt_fields,
        mutation_write_receipt_schema: preflight.mutation_write_receipt_schema,
        required_mutation_write_receipt_fields: preflight.required_mutation_write_receipt_fields,
        runtime_validation_required: preflight.runtime_validation_required,
        can_apply: false,
    }
}

struct EditorWriteBridgePreflight {
    schema_version: u64,
    scope: String,
    state: String,
    can_mutate_source: bool,
    required_receipts: Vec<String>,
    required_editor_guards: Vec<String>,
    required_native_handlers: Vec<String>,
    required_native_handler_capabilities: Vec<String>,
    required_source_apply_review_receipt_fields: Vec<String>,
    required_runtime_proofs: Vec<String>,
    runtime_validation_receipt_schema: String,
    required_runtime_validation_receipt_fields: Vec<String>,
    mutation_write_receipt_schema: String,
    required_mutation_write_receipt_fields: Vec<String>,
    runtime_validation_required: bool,
}

fn read_preflight_fixture(path: &Path) -> Option<EditorWriteBridgePreflight> {
    let value = serde_json::from_str::<Value>(&read_text_limited(path)?).ok()?;
    preflight_from_value(&value)
}

fn generated_preflight() -> Option<EditorWriteBridgePreflight> {
    let value = serde_json::from_str::<Value>(GENERATED_EDITOR_WRITE_BRIDGE_PREFLIGHT_JSON).ok()?;
    preflight_from_value(&value)
}

fn preflight_from_value(value: &Value) -> Option<EditorWriteBridgePreflight> {
    if value.get("schema")?.as_str()? != GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_SCHEMA {
        return None;
    }
    Some(EditorWriteBridgePreflight {
        schema_version: value
            .get("schema_version")
            .and_then(Value::as_u64)
            .unwrap_or(1),
        scope: value
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("preflight requirements for trusted grouped-class editor source writes")
            .to_string(),
        state: value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("not_enabled")
            .to_string(),
        can_mutate_source: value
            .get("can_mutate_source")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        required_receipts: string_list(&value, "required_receipts"),
        required_editor_guards: string_list(&value, "required_editor_guards"),
        required_native_handlers: string_list(&value, "required_native_handlers"),
        required_native_handler_capabilities: string_list(
            &value,
            "required_native_handler_capabilities",
        ),
        required_source_apply_review_receipt_fields: string_list(
            &value,
            "required_source_apply_review_receipt_fields",
        ),
        required_runtime_proofs: string_list(&value, "required_runtime_proofs"),
        runtime_validation_receipt_schema: value
            .get("runtime_validation_receipt_schema")
            .and_then(Value::as_str)
            .unwrap_or("zed.web_preview.dx_style.runtime_validation_receipt.v1")
            .to_string(),
        required_runtime_validation_receipt_fields: string_list(
            &value,
            "required_runtime_validation_receipt_fields",
        ),
        mutation_write_receipt_schema: value
            .get("mutation_write_receipt_schema")
            .and_then(Value::as_str)
            .unwrap_or("zed.web_preview.dx_style.mutation_write_receipt.v1")
            .to_string(),
        required_mutation_write_receipt_fields: string_list(
            &value,
            "required_mutation_write_receipt_fields",
        ),
        runtime_validation_required: value
            .get("runtime_validation_required")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    })
}

fn emergency_preflight() -> EditorWriteBridgePreflight {
    EditorWriteBridgePreflight {
        schema_version: 1,
        scope: "preflight requirements for trusted grouped-class editor source writes".to_string(),
        state: "not_enabled".to_string(),
        can_mutate_source: false,
        required_receipts: Vec::new(),
        required_editor_guards: vec![
            "generated editor write-bridge preflight mirror parse failed".to_string(),
        ],
        required_native_handlers: Vec::new(),
        required_native_handler_capabilities: Vec::new(),
        required_source_apply_review_receipt_fields: Vec::new(),
        required_runtime_proofs: Vec::new(),
        runtime_validation_receipt_schema: "zed.web_preview.dx_style.runtime_validation_receipt.v1"
            .to_string(),
        required_runtime_validation_receipt_fields: Vec::new(),
        mutation_write_receipt_schema: "zed.web_preview.dx_style.mutation_write_receipt.v1"
            .to_string(),
        required_mutation_write_receipt_fields: Vec::new(),
        runtime_validation_required: true,
    }
}

fn string_list(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|item| !item.is_empty())
                .take(PREFLIGHT_LIST_LIMIT)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn read_text_limited(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_EDITOR_WRITE_BRIDGE_PREFLIGHT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_EDITOR_WRITE_BRIDGE_PREFLIGHT_BYTES {
        return None;
    }
    String::from_utf8(bytes).ok()
}
