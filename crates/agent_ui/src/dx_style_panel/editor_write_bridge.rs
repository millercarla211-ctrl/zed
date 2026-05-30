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
    pub(super) preflight_fixture_path: String,
    pub(super) required_receipts: Vec<String>,
    pub(super) required_editor_guards: Vec<String>,
    pub(super) required_native_handlers: Vec<String>,
    pub(super) required_native_handler_capabilities: Vec<String>,
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
            "preflight_fixture_path": self.preflight_fixture_path,
            "required_receipts": self.required_receipts,
            "required_editor_guards": self.required_editor_guards,
            "required_native_handlers": self.required_native_handlers,
            "required_native_handler_capabilities": self.required_native_handler_capabilities,
            "runtime_validation_required": self.runtime_validation_required,
            "can_apply": self.can_apply,
        })
    }
}

const GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_SCHEMA: &str =
    "dx.style.grouped-class-editor-write-bridge-preflight";
const GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_FIXTURE: &str =
    r"G:\Dx\style\fixtures\grouped-class-editor-write-bridge-preflight.json";
const MAX_EDITOR_WRITE_BRIDGE_PREFLIGHT_BYTES: u64 = 64 * 1024;
const PREFLIGHT_LIST_LIMIT: usize = 16;

pub(super) fn style_editor_write_bridge_snapshot() -> StyleEditorWriteBridgeSnapshot {
    let preflight_path = PathBuf::from(GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_FIXTURE);
    let preflight = read_preflight_fixture(&preflight_path).unwrap_or_else(fallback_preflight);

    StyleEditorWriteBridgeSnapshot {
        state: preflight.state,
        summary: format!(
            "{} receipt(s), {} guard(s), {} native handler(s), {} handler capability(s), runtime validation {}",
            preflight.required_receipts.len(),
            preflight.required_editor_guards.len(),
            preflight.required_native_handlers.len(),
            preflight.required_native_handler_capabilities.len(),
            if preflight.runtime_validation_required {
                "required"
            } else {
                "not required"
            }
        ),
        reason: concat!(
            "dx.style.grouped-class-editor-write-bridge-preflight is source-owned but not enabled. ",
            "Editor source writes require trusted source spans, fresh dry-run receipts, ",
            "explicit user apply, and runtime validation before Apply can mutate files."
        )
        .to_string(),
        preflight_schema: GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_SCHEMA.to_string(),
        preflight_fixture_path: preflight_path.display().to_string(),
        required_receipts: preflight.required_receipts,
        required_editor_guards: preflight.required_editor_guards,
        required_native_handlers: preflight.required_native_handlers,
        required_native_handler_capabilities: preflight.required_native_handler_capabilities,
        runtime_validation_required: preflight.runtime_validation_required,
        can_apply: false,
    }
}

struct EditorWriteBridgePreflight {
    state: String,
    required_receipts: Vec<String>,
    required_editor_guards: Vec<String>,
    required_native_handlers: Vec<String>,
    required_native_handler_capabilities: Vec<String>,
    runtime_validation_required: bool,
}

fn read_preflight_fixture(path: &Path) -> Option<EditorWriteBridgePreflight> {
    let value = serde_json::from_str::<Value>(&read_text_limited(path)?).ok()?;
    if value.get("schema")?.as_str()? != GROUPED_CLASS_EDITOR_WRITE_BRIDGE_PREFLIGHT_SCHEMA {
        return None;
    }
    Some(EditorWriteBridgePreflight {
        state: value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("not_enabled")
            .to_string(),
        required_receipts: string_list(&value, "required_receipts"),
        required_editor_guards: string_list(&value, "required_editor_guards"),
        required_native_handlers: string_list(&value, "required_native_handlers"),
        required_native_handler_capabilities: string_list(
            &value,
            "required_native_handler_capabilities",
        ),
        runtime_validation_required: value
            .get("runtime_validation_required")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    })
}

fn fallback_preflight() -> EditorWriteBridgePreflight {
    EditorWriteBridgePreflight {
        state: "not_enabled".to_string(),
        required_receipts: vec![
            "dx.style.grouped-class-dry-run-receipt".to_string(),
            "dx.style.grouped-class-source-digest".to_string(),
        ],
        required_editor_guards: vec![
            "active source path match".to_string(),
            "active cursor token span match".to_string(),
            "active source digest match".to_string(),
            "explicit user apply action".to_string(),
            "bounded edit preview review".to_string(),
        ],
        required_native_handlers: vec!["window.__DX_STYLE_SOURCE_APPLY__".to_string()],
        required_native_handler_capabilities: vec![
            "can_review_request".to_string(),
            "can_mutate_source".to_string(),
        ],
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
