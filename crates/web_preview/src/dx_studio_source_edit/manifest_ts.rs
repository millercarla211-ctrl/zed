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
    insert_bool(
        &mut contract,
        "allowGeneratedEdits",
        extract_bool_from_contract(contents, "allowGeneratedEdits")
            .or_else(|| extract_bool_from_contract(contents, "allow_generated_edits")),
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
    let Some((open, close)) = assigned_delimiter_range(contents, name, '[', ']') else {
        return Vec::new();
    };

    object_literals(&contents[open + 1..close])
}

fn object_after_marker(contents: &str, marker: &str) -> Option<String> {
    let (open, close) = assigned_delimiter_range(contents, marker, '{', '}')?;
    Some(contents[open..=close].to_string())
}

fn assigned_delimiter_range(
    contents: &str,
    name: &str,
    open_delimiter: char,
    close_delimiter: char,
) -> Option<(usize, usize)> {
    for start in identifier_positions(contents, name) {
        let after_name = start + name.len();
        let Some(assignment) = find_assignment_after_identifier(contents, after_name) else {
            continue;
        };
        let Some(open) = find_unquoted_char(contents, assignment + 1, open_delimiter) else {
            continue;
        };
        let Some(close) = matching_delimiter(contents, open, open_delimiter, close_delimiter)
        else {
            continue;
        };
        return Some((open, close));
    }
    None
}

fn identifier_positions(contents: &str, name: &str) -> Vec<usize> {
    contents
        .match_indices(name)
        .filter_map(|(start, _)| {
            if identifier_is_non_value_declaration(contents, start) {
                return None;
            }
            let before = start
                .checked_sub(1)
                .and_then(|index| contents.as_bytes().get(index))
                .copied();
            let after = contents.as_bytes().get(start + name.len()).copied();
            (!before.is_some_and(is_identifier_byte) && !after.is_some_and(is_identifier_byte))
                .then_some(start)
        })
        .collect()
}

fn identifier_is_non_value_declaration(contents: &str, start: usize) -> bool {
    let line_start = contents[..start]
        .rfind('\n')
        .map(|offset| offset + 1)
        .unwrap_or(0);
    let prefix = contents[line_start..start].trim_start();
    prefix.starts_with("//")
        || prefix.starts_with("type ")
        || prefix.starts_with("export type ")
        || prefix.starts_with("interface ")
        || prefix.starts_with("export interface ")
}

fn find_assignment_after_identifier(contents: &str, from: usize) -> Option<usize> {
    let bytes = contents.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    for (offset, character) in contents[from..].char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }

        if matches!(character, '"' | '\'' | '`') {
            quote = Some(character);
            continue;
        }
        if character == ';' {
            return None;
        }
        if character == '=' {
            let absolute = from + offset;
            let previous = absolute
                .checked_sub(1)
                .and_then(|index| bytes.get(index))
                .copied();
            let next = bytes.get(absolute + 1).copied();
            if previous != Some(b'=')
                && !matches!(next, Some(b'=' | b'>'))
                && previous != Some(b'>')
                && previous != Some(b'<')
            {
                return Some(absolute);
            }
        }
    }
    None
}

fn find_unquoted_char(contents: &str, from: usize, target: char) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (offset, character) in contents[from..].char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }

        if matches!(character, '"' | '\'' | '`') {
            quote = Some(character);
            continue;
        }
        if character == target {
            return Some(from + offset);
        }
        if character == ';' {
            return None;
        }
    }
    None
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_exported_contract_arrays_after_assignment() {
        let source = r#"
export const launchStudioEditContract: DxStudioEditContract = {
  schema: "zed.web_preview.dx_studio.launch_edit_contract.v1",
  route: "/launch",
  sourceOwned: true,
  allowGeneratedEdits: false,
};

export const launchStudioEditOperations: LaunchStudioEditOperation[] = [
  {
    operation: "update_text_content",
    selector: "[data-dx-edit-id='launch.hero']",
    sourceFile: "components/launch/Hero.tsx",
    writesFiles: true,
  },
];

export const launchStudioEditableSurfaces: LaunchStudioEditableSurface[] = [
  {
    id: "launch.hero",
    selector: "[data-dx-edit-id='launch.hero']",
    sourceFile: "components/launch/Hero.tsx",
    operations: ["update_text_content"],
  },
];
"#;

        let contract = edit_contract_from_typescript(source).expect("typed contract");

        assert_eq!(
            contract
                .pointer("/operations/0/operation")
                .and_then(Value::as_str),
            Some("update_text_content")
        );
        assert_eq!(
            contract
                .pointer("/surfaces/0/sourceFile")
                .and_then(Value::as_str),
            Some("components/launch/Hero.tsx")
        );
        assert_eq!(
            contract
                .pointer("/allowGeneratedEdits")
                .and_then(Value::as_bool),
            Some(false)
        );
    }
}
