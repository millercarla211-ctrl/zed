use serde_json::{
    Number,
    Value,
};

use crate::types::ToonError;

pub fn split_top_level(input: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_quotes = false;
    let mut quote_char = '\0';
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if in_quotes {
            if ch == '\\' {
                current.push(ch);
                escaped = true;
                continue;
            }

            if ch == quote_char {
                in_quotes = false;
            }
            current.push(ch);
            continue;
        }

        match ch {
            '"' | '\'' => {
                in_quotes = true;
                quote_char = ch;
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            '<' => {
                angle_depth += 1;
                current.push(ch);
            }
            '>' => {
                angle_depth = angle_depth.saturating_sub(1);
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            _ if ch == delimiter && paren_depth == 0 && angle_depth == 0 && bracket_depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

pub fn split_once_top_level(input: &str, delimiter: char) -> Option<(String, String)> {
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_quotes = false;
    let mut quote_char = '\0';
    let mut escaped = false;

    for (byte_index, ch) in input.char_indices() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if in_quotes {
            if ch == '\\' {
                current.push(ch);
                escaped = true;
                continue;
            }

            if ch == quote_char {
                in_quotes = false;
            }
            current.push(ch);
            continue;
        }

        match ch {
            '"' | '\'' => {
                in_quotes = true;
                quote_char = ch;
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            '<' => {
                angle_depth += 1;
                current.push(ch);
            }
            '>' => {
                angle_depth = angle_depth.saturating_sub(1);
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            _ if ch == delimiter && paren_depth == 0 && angle_depth == 0 && bracket_depth == 0 => {
                let left = input[..byte_index].trim().to_string();
                let right = input[byte_index + ch.len_utf8()..].trim().to_string();
                return Some((left, right));
            }
            _ => current.push(ch),
        }
    }

    None
}

pub fn split_once_top_level_str(input: &str, delimiter: &str) -> Option<(String, String)> {
    if delimiter.is_empty() {
        return None;
    }

    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_quotes = false;
    let mut quote_char = '\0';
    let mut escaped = false;
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut index = 0usize;

    while index < chars.len() {
        let (byte_index, ch) = chars[index];
        if escaped {
            current.push(ch);
            escaped = false;
            index += 1;
            continue;
        }

        if in_quotes {
            if ch == '\\' {
                current.push(ch);
                escaped = true;
                index += 1;
                continue;
            }

            if ch == quote_char {
                in_quotes = false;
            }
            current.push(ch);
            index += 1;
            continue;
        }

        match ch {
            '"' | '\'' => {
                in_quotes = true;
                quote_char = ch;
                current.push(ch);
                index += 1;
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
                index += 1;
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
                index += 1;
            }
            '<' => {
                angle_depth += 1;
                current.push(ch);
                index += 1;
            }
            '>' => {
                angle_depth = angle_depth.saturating_sub(1);
                current.push(ch);
                index += 1;
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
                index += 1;
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
                index += 1;
            }
            _ => {
                let delimiter_matches = paren_depth == 0
                    && angle_depth == 0
                    && bracket_depth == 0
                    && input[byte_index..].starts_with(delimiter);

                if delimiter_matches {
                    let left = input[..byte_index].trim().to_string();
                    let right = input[byte_index + delimiter.len()..].trim().to_string();
                    return Some((left, right));
                }

                current.push(ch);
                index += 1;
            }
        }
    }

    None
}

pub fn parse_scalar_literal(input: &str) -> Result<Value, ToonError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Value::String(String::new()));
    }

    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let inner = &trimmed[1..trimmed.len() - 1];
        let normalized = if trimmed.starts_with('\'') {
            inner.replace('\\', "\\\\").replace('"', "\\\"")
        } else {
            inner.to_string()
        };
        return serde_json::from_str::<Value>(&format!("\"{normalized}\""))
            .map_err(|error| ToonError::InvalidInput(format!("Invalid quoted literal: {error}")));
    }

    match trimmed {
        "true" => Ok(Value::Bool(true)),
        "false" => Ok(Value::Bool(false)),
        "null" => Ok(Value::Null),
        _ => {
            if let Ok(value) = trimmed.parse::<i64>() {
                return Ok(Value::Number(Number::from(value)));
            }
            if let Ok(value) = trimmed.parse::<u64>() {
                return Ok(Value::Number(Number::from(value)));
            }
            if let Ok(value) = trimmed.parse::<f64>() {
                if let Some(number) = Number::from_f64(value) {
                    return Ok(Value::Number(number));
                }
            }

            Ok(Value::String(trimmed.to_string()))
        }
    }
}

pub fn render_scalar_literal(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => {
            if text
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
                && !matches!(text.as_str(), "true" | "false" | "null")
            {
                text.clone()
            } else {
                format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
            }
        }
        other => other.to_string(),
    }
}

pub fn indent_block(block: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    block.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
