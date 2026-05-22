use serde_json::{Map, Value};

use super::super::values::{string_array_at, string_at};

pub(super) fn surface_summary(surface: &Value) -> Value {
    let mut summary = Map::new();
    copy_string(surface, &mut summary, "id", &["/id", "/edit_id", "/editId"]);
    copy_string(surface, &mut summary, "selector", &["/selector"]);
    copy_string(
        surface,
        &mut summary,
        "source_file",
        &["/sourceFile", "/source_file", "/source"],
    );
    copy_string(
        surface,
        &mut summary,
        "materialized_file",
        &["/materializedFile", "/materialized_file"],
    );
    copy_string_array(surface, &mut summary, "operations", &["/operations"]);
    copy_string_array(
        surface,
        &mut summary,
        "package_ids",
        &["/packageIds", "/package_ids", "/packages"],
    );
    copy_bool(
        surface,
        &mut summary,
        "absolute_positioning",
        &["/absolutePositioning", "/absolute_positioning"],
    );
    copy_bool(
        surface,
        &mut summary,
        "no_node_modules_required",
        &["/noNodeModulesRequired", "/no_node_modules_required"],
    );
    Value::Object(summary)
}

pub(super) fn operation_summary(operation: &Value) -> Value {
    let mut summary = Map::new();
    copy_string(
        operation,
        &mut summary,
        "operation",
        &["/operation", "/id", "/name"],
    );
    copy_string(operation, &mut summary, "label", &["/label"]);
    copy_string(operation, &mut summary, "selector", &["/selector"]);
    copy_string(
        operation,
        &mut summary,
        "source_file",
        &["/sourceFile", "/source_file", "/source"],
    );
    copy_string(
        operation,
        &mut summary,
        "responsive_policy",
        &["/responsivePolicy", "/responsive_policy"],
    );
    copy_bool(
        operation,
        &mut summary,
        "writes_files",
        &["/writesFiles", "/writes_files"],
    );
    copy_bool(
        operation,
        &mut summary,
        "requires_server_restart",
        &["/requiresServerRestart", "/requires_server_restart"],
    );
    copy_string(
        operation,
        &mut summary,
        "source_snippet",
        &["/sourceSnippet", "/source_snippet"],
    );
    copy_string(
        operation,
        &mut summary,
        "insert_template",
        &["/insertTemplate", "/insert_template"],
    );
    Value::Object(summary)
}

pub(super) fn edit_contract_summary(contract: &Value, manifest_path: &str) -> Value {
    let mut summary = Map::new();
    summary.insert(
        "manifest".to_string(),
        Value::String(manifest_path.to_string()),
    );
    copy_string(contract, &mut summary, "schema", &["/schema"]);
    copy_string(contract, &mut summary, "route", &["/route"]);
    copy_string(
        contract,
        &mut summary,
        "layout_policy",
        &["/layoutPolicy", "/layout_policy"],
    );
    copy_string(
        contract,
        &mut summary,
        "token_scope_marker",
        &["/tokenScopeMarker", "/token_scope_marker"],
    );
    copy_bool(
        contract,
        &mut summary,
        "source_owned",
        &["/sourceOwned", "/source_owned"],
    );
    copy_bool(
        contract,
        &mut summary,
        "no_node_modules_required",
        &["/noNodeModulesRequired", "/no_node_modules_required"],
    );
    copy_bool(
        contract,
        &mut summary,
        "absolute_positioning",
        &["/absolutePositioning", "/absolute_positioning"],
    );
    Value::Object(summary)
}

fn copy_string(source: &Value, target: &mut Map<String, Value>, key: &str, pointers: &[&str]) {
    if let Some(value) = string_at(source, pointers) {
        target.insert(key.to_string(), Value::String(value));
    }
}

fn copy_string_array(
    source: &Value,
    target: &mut Map<String, Value>,
    key: &str,
    pointers: &[&str],
) {
    let values = string_array_at(source, pointers);
    if !values.is_empty() {
        target.insert(
            key.to_string(),
            Value::Array(values.into_iter().map(Value::String).collect()),
        );
    }
}

fn copy_bool(source: &Value, target: &mut Map<String, Value>, key: &str, pointers: &[&str]) {
    if let Some(value) = pointers
        .iter()
        .find_map(|pointer| source.pointer(pointer).and_then(Value::as_bool))
    {
        target.insert(key.to_string(), Value::Bool(value));
    }
}
