use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    Value,
    json,
};

use crate::{
    decode_default,
    encode_default,
    types::{
        ToonError,
        ToonResult,
    },
};

use super::util::{
    indent_block,
    parse_scalar_literal,
    render_scalar_literal,
    split_once_top_level,
    split_top_level,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    System,
    Developer,
    User,
    Assistant,
    Reasoning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolResultStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: AgentRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentTurn {
    Message(AgentMessage),
    ToolCall {
        id: String,
        tool: String,
        args: Value,
    },
    ToolResult {
        id: String,
        tool: Option<String>,
        status: ToolResultStatus,
        result: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgentConversation {
    pub turns: Vec<AgentTurn>,
}

impl AgentRole {
    fn marker(self) -> char {
        match self {
            AgentRole::System => 'S',
            AgentRole::Developer => 'D',
            AgentRole::User => 'U',
            AgentRole::Assistant => 'A',
            AgentRole::Reasoning => 'R',
        }
    }

    fn from_marker(marker: char) -> Option<Self> {
        match marker {
            'S' => Some(AgentRole::System),
            'D' => Some(AgentRole::Developer),
            'U' => Some(AgentRole::User),
            'A' => Some(AgentRole::Assistant),
            'R' => Some(AgentRole::Reasoning),
            _ => None,
        }
    }
}

impl AgentConversation {
    pub fn encode_dx_serializer(&self) -> ToonResult<String> {
        encode_conversation_dsl(self)
    }

    pub fn decode_dx_serializer(input: &str) -> ToonResult<Self> {
        decode_conversation_dsl(input)
    }

    pub fn encode_dsl(&self) -> ToonResult<String> {
        encode_conversation_dsl(self)
    }

    pub fn decode_dsl(input: &str) -> ToonResult<Self> {
        decode_conversation_dsl(input)
    }

    pub fn as_prompt_json(&self) -> Value {
        Value::Array(
            self.turns
                .iter()
                .map(|turn| match turn {
                    AgentTurn::Message(message) => json!({
                        "role": format!("{:?}", message.role).to_lowercase(),
                        "content": message.content,
                    }),
                    AgentTurn::ToolCall { id, tool, args } => json!({
                        "type": "tool_call",
                        "id": id,
                        "tool": tool,
                        "args": args,
                    }),
                    AgentTurn::ToolResult {
                        id,
                        tool,
                        status,
                        result,
                    } => json!({
                        "type": "tool_result",
                        "id": id,
                        "tool": tool,
                        "status": match status {
                            ToolResultStatus::Ok => "ok",
                            ToolResultStatus::Error => "error",
                        },
                        "result": result,
                    }),
                })
                .collect(),
        )
    }
}

pub fn encode_conversation_dsl(conversation: &AgentConversation) -> ToonResult<String> {
    let mut chunks = Vec::new();

    for turn in &conversation.turns {
        match turn {
            AgentTurn::Message(message) => {
                if message.content.contains('\n') {
                    chunks.push(format!(
                        "{}>>>\n{}\n<<<",
                        message.role.marker(),
                        message.content
                    ));
                } else {
                    chunks.push(format!("{}> {}", message.role.marker(), message.content));
                }
            }
            AgentTurn::ToolCall { id, tool, args } => {
                if is_effectively_empty(args) {
                    chunks.push(format!("C#{id} {tool}"));
                } else if let Some(inline) = render_inline_object(args) {
                    chunks.push(format!("C#{id} {tool}({inline})"));
                } else {
                    let toon = encode_default(args)?;
                    chunks.push(format!("C#{id} {tool}:\n{}", indent_block(&toon, 2)));
                }
            }
            AgentTurn::ToolResult {
                id,
                tool,
                status,
                result,
            } => {
                let header = if let Some(tool) = tool {
                    format!(
                        "T#{id} {} {tool}",
                        match status {
                            ToolResultStatus::Ok => "ok",
                            ToolResultStatus::Error => "error",
                        }
                    )
                } else {
                    format!(
                        "T#{id} {}",
                        match status {
                            ToolResultStatus::Ok => "ok",
                            ToolResultStatus::Error => "error",
                        }
                    )
                };

                if is_effectively_empty(result) {
                    chunks.push(header);
                } else if tool.is_some() {
                    if let Some(inline) = render_inline_object(result) {
                        chunks.push(format!("{header}({inline})"));
                        continue;
                    }
                    let toon = encode_default(result)?;
                    chunks.push(format!("{header}:\n{}", indent_block(&toon, 2)));
                } else {
                    let toon = encode_default(result)?;
                    chunks.push(format!("{header}:\n{}", indent_block(&toon, 2)));
                }
            }
        }
    }

    Ok(chunks.join("\n\n"))
}

pub fn encode_dx_serializer_conversation(
    conversation: &AgentConversation,
) -> ToonResult<String> {
    encode_conversation_dsl(conversation)
}

pub fn decode_conversation_dsl(input: &str) -> ToonResult<AgentConversation> {
    let lines: Vec<&str> = input.lines().collect();
    let mut turns = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        let raw_line = lines[index].trim_end();
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }

        if let Some((role, content, consumed)) = parse_message_turn(&lines, index)? {
            turns.push(AgentTurn::Message(AgentMessage { role, content }));
            index = consumed;
            continue;
        }

        if trimmed.starts_with("C#") {
            let (turn, consumed) = parse_tool_call_turn(&lines, index)?;
            turns.push(turn);
            index = consumed;
            continue;
        }

        if trimmed.starts_with("T#") {
            let (turn, consumed) = parse_tool_result_turn(&lines, index)?;
            turns.push(turn);
            index = consumed;
            continue;
        }

        return Err(ToonError::InvalidInput(format!(
            "Unrecognized conversation line: {trimmed}"
        )));
    }

    Ok(AgentConversation { turns })
}

pub fn decode_dx_serializer_conversation(input: &str) -> ToonResult<AgentConversation> {
    decode_conversation_dsl(input)
}

fn parse_message_turn(lines: &[&str], index: usize) -> ToonResult<Option<(AgentRole, String, usize)>> {
    let trimmed = lines[index].trim();
    let mut chars = trimmed.chars();
    let Some(marker) = chars.next() else {
        return Ok(None);
    };
    let Some(role) = AgentRole::from_marker(marker) else {
        return Ok(None);
    };

    let rest = chars.as_str();
    if let Some(content) = rest.strip_prefix("> ") {
        return Ok(Some((role, content.to_string(), index + 1)));
    }
    if rest == ">" {
        return Ok(Some((role, String::new(), index + 1)));
    }
    if rest == ">>>" {
        let mut collected = Vec::new();
        let mut cursor = index + 1;
        while cursor < lines.len() {
            if lines[cursor].trim() == "<<<" {
                return Ok(Some((role, collected.join("\n"), cursor + 1)));
            }
            collected.push(lines[cursor].to_string());
            cursor += 1;
        }
        return Err(ToonError::InvalidInput(format!(
            "Missing closing <<< block for {} message",
            marker
        )));
    }

    Ok(None)
}

fn parse_tool_call_turn(lines: &[&str], index: usize) -> ToonResult<(AgentTurn, usize)> {
    let trimmed = lines[index].trim();
    let header = trimmed
        .strip_prefix("C#")
        .ok_or_else(|| ToonError::InvalidInput(format!("Invalid tool call line: {trimmed}")))?;

    let mut parts = header.trim().split_whitespace();
    let id = parts
        .next()
        .ok_or_else(|| ToonError::InvalidInput(format!("Tool call missing id: {trimmed}")))?;
    let remainder = header.trim()[id.len()..].trim();
    if remainder.is_empty() {
        return Err(ToonError::InvalidInput(format!(
            "Tool call missing tool name: {trimmed}"
        )));
    }

    let (tool, inline_args, has_block) = if let Some(stripped) = remainder.strip_suffix(':') {
        (stripped.trim().to_string(), None, true)
    } else if let Some(open_index) = remainder.find('(') {
        if !remainder.ends_with(')') {
            return Err(ToonError::InvalidInput(format!(
                "Inline tool-call arguments must end with ')': {trimmed}"
            )));
        }
        (
            remainder[..open_index].trim().to_string(),
            Some(remainder[open_index + 1..remainder.len() - 1].trim().to_string()),
            false,
        )
    } else {
        (remainder.to_string(), None, false)
    };
    if tool.is_empty() {
        return Err(ToonError::InvalidInput(format!(
            "Tool call missing tool name: {trimmed}"
        )));
    }

    let (args, consumed) = if has_block {
        let (block, consumed) = collect_indented_block(lines, index + 1)?;
        if block.trim().is_empty() {
            (json!({}), consumed)
        } else {
            (decode_default(&block)?, consumed)
        }
    } else if let Some(inline_args) = inline_args {
        (parse_inline_object(&inline_args)?, index + 1)
    } else {
        (json!({}), index + 1)
    };

    Ok((
        AgentTurn::ToolCall {
            id: id.to_string(),
            tool,
            args,
        },
        consumed,
    ))
}

fn parse_tool_result_turn(lines: &[&str], index: usize) -> ToonResult<(AgentTurn, usize)> {
    let trimmed = lines[index].trim();
    let header = trimmed
        .strip_prefix("T#")
        .ok_or_else(|| ToonError::InvalidInput(format!("Invalid tool result line: {trimmed}")))?;

    let header = header.trim();

    let mut parts = header.split_whitespace();
    let id = parts
        .next()
        .ok_or_else(|| ToonError::InvalidInput(format!("Tool result missing id: {trimmed}")))?;
    let status = match parts.next() {
        Some("ok") => ToolResultStatus::Ok,
        Some("error") => ToolResultStatus::Error,
        Some(other) => {
            return Err(ToonError::InvalidInput(format!(
                "Unsupported tool result status '{other}'",
            )))
        }
        None => {
            return Err(ToonError::InvalidInput(format!(
                "Tool result missing status: {trimmed}",
            )))
        }
    };
    let remainder = header[id.len()..].trim_start();
    let remainder = match status {
        ToolResultStatus::Ok => remainder
            .strip_prefix("ok")
            .ok_or_else(|| ToonError::InvalidInput(format!("Tool result missing 'ok' marker: {trimmed}")))?,
        ToolResultStatus::Error => remainder.strip_prefix("error").ok_or_else(|| {
            ToonError::InvalidInput(format!("Tool result missing 'error' marker: {trimmed}"))
        })?,
    }
    .trim();

    let (tool, inline_result, has_block) = if remainder.is_empty() {
        (None, None, false)
    } else if let Some(stripped) = remainder.strip_suffix(':') {
        (Some(stripped.trim().to_string()), None, true)
    } else if let Some(open_index) = remainder.find('(') {
        if !remainder.ends_with(')') {
            return Err(ToonError::InvalidInput(format!(
                "Inline tool-result payload must end with ')': {trimmed}"
            )));
        }
        (
            Some(remainder[..open_index].trim().to_string()),
            Some(remainder[open_index + 1..remainder.len() - 1].trim().to_string()),
            false,
        )
    } else {
        (Some(remainder.to_string()), None, false)
    };
    if tool.as_deref().is_some_and(str::is_empty) {
        return Err(ToonError::InvalidInput(format!(
            "Tool result is missing a tool name after status: {trimmed}"
        )));
    }

    let (result, consumed) = if has_block {
        let (block, consumed) = collect_indented_block(lines, index + 1)?;
        if block.trim().is_empty() {
            (json!({}), consumed)
        } else {
            (decode_default(&block)?, consumed)
        }
    } else if let Some(inline_result) = inline_result {
        (parse_inline_object(&inline_result)?, index + 1)
    } else {
        (json!({}), index + 1)
    };

    Ok((
        AgentTurn::ToolResult {
            id: id.to_string(),
            tool,
            status,
            result,
        },
        consumed,
    ))
}

fn collect_indented_block(lines: &[&str], start: usize) -> ToonResult<(String, usize)> {
    let mut collected = Vec::new();
    let mut cursor = start;

    while cursor < lines.len() {
        let line = lines[cursor];
        if line.trim().is_empty() {
            collected.push(String::new());
            cursor += 1;
            continue;
        }
        if let Some(stripped) = line.strip_prefix("  ") {
            collected.push(stripped.to_string());
            cursor += 1;
            continue;
        }
        break;
    }

    if collected.is_empty() {
        return Err(ToonError::InvalidInput(
            "Indented TOON block expected after tool header".to_string(),
        ));
    }

    while collected.last().is_some_and(String::is_empty) {
        collected.pop();
    }

    Ok((collected.join("\n"), cursor))
}

fn is_effectively_empty(value: &Value) -> bool {
    matches!(value, Value::Object(object) if object.is_empty())
}

fn render_inline_object(value: &Value) -> Option<String> {
    let Value::Object(object) = value else {
        return None;
    };
    if object.is_empty() {
        return None;
    }
    if !object.values().all(is_inline_scalar) {
        return None;
    }

    Some(
        object
            .iter()
            .map(|(key, value)| format!("{key}={}", render_scalar_literal(value)))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn parse_inline_object(input: &str) -> ToonResult<Value> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(json!({}));
    }

    let mut object = serde_json::Map::new();
    for pair in split_top_level(trimmed, ',') {
        let (key, value) = split_once_top_level(&pair, '=').ok_or_else(|| {
            ToonError::InvalidInput(format!(
                "Inline object entries must use key=value syntax: {pair}"
            ))
        })?;
        let key = key.trim();
        if key.is_empty() {
            return Err(ToonError::InvalidInput(
                "Inline object entry is missing a key".to_string(),
            ));
        }
        object.insert(key.to_string(), parse_scalar_literal(&value)?);
    }
    Ok(Value::Object(object))
}

fn is_inline_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn conversation_round_trip_keeps_tool_calls_and_messages() {
        let conversation = AgentConversation {
            turns: vec![
                AgentTurn::Message(AgentMessage {
                    role: AgentRole::System,
                    content: "You are DX.".to_string(),
                }),
                AgentTurn::Message(AgentMessage {
                    role: AgentRole::User,
                    content: "Summarize src/lib.rs".to_string(),
                }),
                AgentTurn::ToolCall {
                    id: "c1".to_string(),
                    tool: "read_file".to_string(),
                    args: json!({"path": "src/lib.rs"}),
                },
                AgentTurn::ToolResult {
                    id: "c1".to_string(),
                    tool: Some("read_file".to_string()),
                    status: ToolResultStatus::Ok,
                    result: json!({"text": "pub mod lib;"}),
                },
                AgentTurn::Message(AgentMessage {
                    role: AgentRole::Assistant,
                    content: "It exports the library modules.".to_string(),
                }),
            ],
        };

        let encoded = encode_conversation_dsl(&conversation).unwrap();
        assert!(encoded.contains("C#c1 read_file(path=src/lib.rs)"));
        assert!(encoded.contains("T#c1 ok read_file(text=\"pub mod lib;\")"));

        let decoded = decode_conversation_dsl(&encoded).unwrap();
        assert_eq!(decoded, conversation);
    }

    #[test]
    fn multiline_messages_use_block_syntax() {
        let conversation = AgentConversation {
            turns: vec![AgentTurn::Message(AgentMessage {
                role: AgentRole::Developer,
                content: "Line 1\nLine 2".to_string(),
            })],
        };

        let encoded = encode_conversation_dsl(&conversation).unwrap();
        assert!(encoded.contains("D>>>"));
        assert!(encoded.contains("<<<"));

        let decoded = decode_conversation_dsl(&encoded).unwrap();
        assert_eq!(decoded, conversation);
    }

    #[test]
    fn flat_scalar_tool_payloads_use_inline_syntax() {
        let conversation = AgentConversation {
            turns: vec![
                AgentTurn::ToolCall {
                    id: "c1".to_string(),
                    tool: "read_file".to_string(),
                    args: json!({"path": "src/lib.rs", "encoding": "utf8"}),
                },
                AgentTurn::ToolResult {
                    id: "c1".to_string(),
                    tool: Some("read_file".to_string()),
                    status: ToolResultStatus::Ok,
                    result: json!({"mime": "text/plain", "ok": true}),
                },
            ],
        };

        let encoded = encode_conversation_dsl(&conversation).unwrap();
        assert!(encoded.contains("C#c1 read_file(path=src/lib.rs,encoding=utf8)"));
        assert!(encoded.contains("T#c1 ok read_file(mime=text/plain,ok=true)"));

        let decoded = decode_conversation_dsl(&encoded).unwrap();
        assert_eq!(decoded, conversation);
    }
}
