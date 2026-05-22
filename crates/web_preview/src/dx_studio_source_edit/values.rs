use serde_json::Value;

pub(super) fn string_at(value: &Value, pointers: &[&str]) -> Option<String> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

pub(super) fn push_string_at(value: &Value, pointers: &[&str], output: &mut Vec<String>) {
    for pointer in pointers {
        if let Some(value) = value.pointer(pointer).and_then(Value::as_str)
            && !value.trim().is_empty()
        {
            output.push(value.to_string());
        }
    }
}

pub(super) fn bool_at(value: &Value, pointers: &[&str]) -> Option<bool> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_bool))
}

pub(super) fn string_array_at(value: &Value, pointers: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    for pointer in pointers {
        if let Some(candidate) = value.pointer(pointer) {
            push_string_values(candidate, &mut values);
        }
    }
    unique_strings(values)
}

fn push_string_values(value: &Value, values: &mut Vec<String>) {
    if let Some(value) = value.as_str() {
        if !value.trim().is_empty() {
            values.push(value.to_string());
        }
    } else if let Some(array) = value.as_array() {
        for item in array {
            push_string_values(item, values);
        }
    }
}

pub(super) fn line_indent_before(contents: &str, index: usize) -> String {
    let line_start = contents[..index]
        .rfind('\n')
        .map(|offset| offset + 1)
        .unwrap_or(0);
    contents[line_start..index]
        .chars()
        .take_while(|character| {
            character.is_whitespace() && *character != '\n' && *character != '\r'
        })
        .collect()
}

pub(super) fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

pub(super) fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}
