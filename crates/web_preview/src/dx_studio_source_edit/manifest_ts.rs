use serde_json::{Map, Value};

use crate::dx_studio;

pub(crate) fn edit_contract_from_typescript(contents: &str) -> Option<Value> {
    if !contents.contains(dx_studio::DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA) {
        return None;
    }

    let operations = const_array_objects(contents, "launchStudioEditOperations")
        .into_iter()
        .map(operation_from_object)
        .collect::<Vec<_>>();
    let surfaces = const_array_objects(contents, "launchStudioEditableSurfaces")
        .into_iter()
        .map(surface_from_object)
        .collect::<Vec<_>>();

    if operations.is_empty() || surfaces.is_empty() {
        return None;
    }

    let mut contract = Map::new();
    contract.insert(
        "schema".to_string(),
        Value::String(dx_studio::DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA.to_string()),
    );
    insert_string(
        &mut contract,
        "route",
        extract_property_from_contract(contents, "route"),
    );
    insert_string(
        &mut contract,
        "sourceManifestFile",
        extract_property_from_contract(contents, "sourceManifestFile"),
    );
    insert_string(
        &mut contract,
        "materializedManifestFile",
        extract_property_from_contract(contents, "materializedManifestFile"),
    );
    insert_string(
        &mut contract,
        "layoutPolicy",
        extract_property_from_contract(contents, "layoutPolicy"),
    );
    insert_string(
        &mut contract,
        "tokenScopeMarker",
        extract_property_from_contract(contents, "tokenScopeMarker"),
    );
    insert_bool(
        &mut contract,
        "sourceOwned",
        extract_bool_from_contract(contents, "sourceOwned"),
    );
    insert_bool(
        &mut contract,
        "noNodeModulesRequired",
        extract_bool_from_contract(contents, "noNodeModulesRequired"),
    );
    insert_bool(
        &mut contract,
        "absolutePositioning",
        extract_bool_from_contract(contents, "absolutePositioning"),
    );
    contract.insert("operations".to_string(), Value::Array(operations));
    contract.insert(
        "editableSurfaces".to_string(),
        Value::Array(surfaces.clone()),
    );
    contract.insert("surfaces".to_string(), Value::Array(surfaces));

    Some(Value::Object(contract))
}

fn operation_from_object(object: String) -> Value {
    let mut value = Map::new();
    copy_string(&object, &mut value, "operation", "operation");
    copy_string(&object, &mut value, "label", "label");
    copy_string(&object, &mut value, "selector", "selector");
    copy_string(&object, &mut value, "sourceFile", "sourceFile");
    copy_string(&object, &mut value, "responsivePolicy", "responsivePolicy");
    copy_string(&object, &mut value, "sourceSnippet", "sourceSnippet");
    copy_string(&object, &mut value, "insertTemplate", "insertTemplate");
    copy_bool(&object, &mut value, "writesFiles", "writesFiles");
    copy_bool(
        &object,
        &mut value,
        "requiresNodeModules",
        "requiresNodeModules",
    );
    copy_bool(
        &object,
        &mut value,
        "requiresServerRestart",
        "requiresServerRestart",
    );
    copy_bool(
        &object,
        &mut value,
        "requiresPackageInstall",
        "requiresPackageInstall",
    );
    Value::Object(value)
}

fn surface_from_object(object: String) -> Value {
    let mut value = Map::new();
    copy_string(&object, &mut value, "id", "id");
    copy_string(&object, &mut value, "selector", "selector");
    copy_string(&object, &mut value, "sourceFile", "sourceFile");
    copy_string(&object, &mut value, "materializedFile", "materializedFile");
    copy_string(&object, &mut value, "layoutPolicy", "layoutPolicy");
    copy_string(&object, &mut value, "receiptPath", "receiptPath");
    copy_string_array(&object, &mut value, "packageIds", "packageIds");
    copy_string_array(&object, &mut value, "operations", "operations");
    copy_string_array(
        &object,
        &mut value,
        "interactionSelectors",
        "interactionSelectors",
    );
    copy_string_array(&object, &mut value, "stateMarkers", "stateMarkers");
    copy_bool(
        &object,
        &mut value,
        "absolutePositioning",
        "absolutePositioning",
    );
    copy_bool(
        &object,
        &mut value,
        "noNodeModulesRequired",
        "noNodeModulesRequired",
    );
    Value::Object(value)
}

fn extract_property_from_contract(contents: &str, key: &str) -> Option<String> {
    let contract = object_after_marker(contents, "launchStudioEditContract")?;
    extract_string_property(&contract, key)
}

fn extract_bool_from_contract(contents: &str, key: &str) -> Option<bool> {
    let contract = object_after_marker(contents, "launchStudioEditContract")?;
    extract_bool_property(&contract, key)
}

fn const_array_objects(contents: &str, name: &str) -> Vec<String> {
    let Some(start) = contents.find(name) else {
        return Vec::new();
    };
    let Some(open) = contents[start..].find('[').map(|offset| start + offset) else {
        return Vec::new();
    };
    let Some(close) = matching_delimiter(contents, open, '[', ']') else {
        return Vec::new();
    };

    object_literals(&contents[open + 1..close])
}

fn object_after_marker(contents: &str, marker: &str) -> Option<String> {
    let start = contents.find(marker)?;
    let open = contents[start..].find('{').map(|offset| start + offset)?;
    let close = matching_delimiter(contents, open, '{', '}')?;
    Some(contents[open..=close].to_string())
}

fn object_literals(contents: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut index = 0;
    while let Some(relative_open) = contents[index..].find('{') {
        let open = index + relative_open;
        let Some(close) = matching_delimiter(contents, open, '{', '}') else {
            break;
        };
        objects.push(contents[open..=close].to_string());
        index = close + 1;
    }
    objects
}

fn matching_delimiter(
    contents: &str,
    open_index: usize,
    open_delimiter: char,
    close_delimiter: char,
) -> Option<usize> {
    let mut depth = 0usize;
    let mut string_quote = None;
    let mut escaped = false;

    for (offset, character) in contents[open_index..].char_indices() {
        if let Some(quote) = string_quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == quote {
                string_quote = None;
            }
            continue;
        }

        if matches!(character, '"' | '\'' | '`') {
            string_quote = Some(character);
        } else if character == open_delimiter {
            depth += 1;
        } else if character == close_delimiter {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(open_index + offset);
            }
        }
    }

    None
}

fn copy_string(source: &str, target: &mut Map<String, Value>, target_key: &str, source_key: &str) {
    insert_string(
        target,
        target_key,
        extract_string_property(source, source_key),
    );
}

fn copy_string_array(
    source: &str,
    target: &mut Map<String, Value>,
    target_key: &str,
    source_key: &str,
) {
    let values = extract_string_array_property(source, source_key);
    if !values.is_empty() {
        target.insert(
            target_key.to_string(),
            Value::Array(values.into_iter().map(Value::String).collect()),
        );
    }
}

fn copy_bool(source: &str, target: &mut Map<String, Value>, target_key: &str, source_key: &str) {
    insert_bool(
        target,
        target_key,
        extract_bool_property(source, source_key),
    );
}

fn insert_string(target: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        target.insert(key.to_string(), Value::String(value));
    }
}

fn insert_bool(target: &mut Map<String, Value>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        target.insert(key.to_string(), Value::Bool(value));
    }
}

fn extract_string_property(source: &str, key: &str) -> Option<String> {
    let marker = format!("{key}:");
    let start = source.find(&marker)? + marker.len();
    extract_quoted_value(&source[start..])
}

fn extract_bool_property(source: &str, key: &str) -> Option<bool> {
    let marker = format!("{key}:");
    let mut value = source[source.find(&marker)? + marker.len()..].trim_start();
    value = value.strip_prefix('!').unwrap_or(value);
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn extract_string_array_property(source: &str, key: &str) -> Vec<String> {
    let marker = format!("{key}:");
    let Some(start) = source.find(&marker).map(|offset| offset + marker.len()) else {
        return Vec::new();
    };
    let Some(open) = source[start..].find('[').map(|offset| start + offset) else {
        return Vec::new();
    };
    let Some(close) = matching_delimiter(source, open, '[', ']') else {
        return Vec::new();
    };

    quoted_values(&source[open + 1..close])
}

fn quoted_values(source: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut index = 0;
    while let Some((relative_quote, _)) = source[index..]
        .char_indices()
        .find(|(_, character)| matches!(character, '"' | '\'' | '`'))
    {
        let start = index + relative_quote;
        if let Some((value, consumed)) = extract_quoted_value_with_len(&source[start..]) {
            values.push(value);
            index = start + consumed;
        } else {
            index = start + 1;
        }
    }
    values
}

fn extract_quoted_value(source: &str) -> Option<String> {
    extract_quoted_value_with_len(source).map(|(value, _)| value)
}

fn extract_quoted_value_with_len(source: &str) -> Option<(String, usize)> {
    let source = source.trim_start();
    let quote = source.chars().next()?;
    if !matches!(quote, '"' | '\'' | '`') {
        return None;
    }
    let value_start = quote.len_utf8();
    let mut escaped = false;
    for (offset, character) in source[value_start..].char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == quote {
            let consumed = value_start + offset + quote.len_utf8();
            return Some((
                source[value_start..value_start + offset].to_string(),
                consumed,
            ));
        }
    }
    None
}
