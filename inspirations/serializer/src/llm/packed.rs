use std::collections::{
    HashMap,
    HashSet,
};

use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    Map,
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

use super::{
    AgentConversation,
    AgentMessage,
    AgentRole,
    AgentToolCatalog,
    AgentTurn,
    DxSerializerRegistryRef,
    ToolResultStatus,
    decode_dx_serializer_registry_ref,
    util::{
        indent_block,
        parse_scalar_literal,
        render_scalar_literal,
        split_once_top_level,
        split_top_level,
    },
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedFieldAlias {
    pub alias: String,
    pub name: String,
    pub required: bool,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedToolSpec {
    pub alias: String,
    pub name: String,
    pub input: Vec<PackedFieldAlias>,
    pub output: Vec<PackedFieldAlias>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PackedToolCatalog {
    pub tools: Vec<PackedToolSpec>,
}

impl AgentToolCatalog {
    pub fn to_packed_catalog(&self) -> PackedToolCatalog {
        PackedToolCatalog::from_agent_catalog(self)
    }
}

impl PackedToolCatalog {
    pub fn from_agent_catalog(catalog: &AgentToolCatalog) -> Self {
        let mut used_tool_aliases = HashSet::new();
        let tools = catalog
            .tools
            .iter()
            .map(|tool| {
                let tool_alias = next_unique_alias(&tool.name, &mut used_tool_aliases);
                PackedToolSpec {
                    alias: tool_alias,
                    name: tool.name.clone(),
                    input: build_field_aliases(&tool.input),
                    output: build_field_aliases(&tool.output),
                }
            })
            .collect();

        Self { tools }
    }

    pub fn encode_dsl(&self) -> String {
        self.tools
            .iter()
            .map(PackedToolSpec::encode_dsl)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn encode_dx_serializer(&self) -> String {
        self.encode_dsl()
    }

    pub fn tool_by_name(&self, name: &str) -> Option<&PackedToolSpec> {
        self.tools.iter().find(|tool| tool.name == name)
    }

    pub fn tool_by_alias(&self, alias: &str) -> Option<&PackedToolSpec> {
        self.tools.iter().find(|tool| tool.alias == alias)
    }

    pub fn sole_tool(&self) -> Option<&PackedToolSpec> {
        (self.tools.len() == 1).then(|| &self.tools[0])
    }
}

impl PackedToolSpec {
    pub fn encode_dsl(&self) -> String {
        let input = self
            .input
            .iter()
            .map(PackedFieldAlias::encode_dsl)
            .collect::<Vec<_>>()
            .join(",");
        let output = self
            .output
            .iter()
            .map(PackedFieldAlias::encode_dsl)
            .collect::<Vec<_>>()
            .join(",");

        if self.output.is_empty() {
            format!("@p {}={}({input})", self.alias, self.name)
        } else {
            format!("@p {}={}({input})->({output})", self.alias, self.name)
        }
    }

    fn encode_input_payload(
        &self,
        args: &Value,
        literals: &HashMap<String, String>,
    ) -> ToonResult<PackedPayload> {
        self.encode_payload(args, &self.input, literals)
    }

    fn encode_output_payload(
        &self,
        result: &Value,
        literals: &HashMap<String, String>,
    ) -> ToonResult<PackedPayload> {
        self.encode_payload(result, &self.output, literals)
    }

    fn encode_payload(
        &self,
        payload: &Value,
        fields: &[PackedFieldAlias],
        literals: &HashMap<String, String>,
    ) -> ToonResult<PackedPayload> {
        let Value::Object(object) = payload else {
            return Err(ToonError::InvalidInput(format!(
                "Packed tool payloads must be objects for tool '{}'",
                self.name
            )));
        };

        if object.is_empty() {
            return Ok(PackedPayload::Empty);
        }

        let has_unknown_keys = object.keys().any(|key| fields.iter().all(|field| field.name != *key));
        let mut inline_values = Vec::with_capacity(fields.len());
        let mut all_inline = !has_unknown_keys;

        for field in fields {
            match object.get(&field.name) {
                Some(value) => {
                    if is_inline_scalar(value) {
                        if field.default.as_ref() == Some(value) {
                            inline_values.push(None);
                        } else {
                            inline_values.push(Some(render_packed_scalar(value, literals)?));
                        }
                    } else {
                        all_inline = false;
                        inline_values.push(None);
                    }
                }
                None => {
                    if field.required && field.default.is_none() {
                        return Err(ToonError::InvalidInput(format!(
                            "Packed payload for '{}' is missing required field '{}'",
                            self.name, field.name
                        )));
                    }
                    inline_values.push(None);
                }
            }
        }

        if all_inline {
            trim_omittable_tail(&mut inline_values);
            let rendered = inline_values
                .into_iter()
                .map(|value| value.unwrap_or_else(|| "_".to_string()))
                .collect::<Vec<_>>();

            if rendered.is_empty() {
                return Ok(PackedPayload::Empty);
            }

            if rendered.len() == 1 {
                return Ok(PackedPayload::Scalar(rendered[0].clone()));
            }

            return Ok(PackedPayload::Inline(rendered.join(",")));
        }

        let mut aliased = Map::new();
        for (key, value) in object {
            if let Some(field) = fields.iter().find(|field| field.name == *key) {
                aliased.insert(field.alias.clone(), value.clone());
            } else {
                aliased.insert(key.clone(), value.clone());
            }
        }

        let block = encode_default(&Value::Object(aliased))?;
        Ok(PackedPayload::Block(block))
    }

    fn decode_input_inline(
        &self,
        inline: &str,
        literals: &HashMap<String, String>,
    ) -> ToonResult<Value> {
        decode_inline_payload(inline, &self.input, literals)
    }

    fn decode_output_inline(
        &self,
        inline: &str,
        literals: &HashMap<String, String>,
    ) -> ToonResult<Value> {
        decode_inline_payload(inline, &self.output, literals)
    }

    fn decode_input_block(&self, block: &str) -> ToonResult<Value> {
        decode_block_payload(block, &self.input)
    }

    fn decode_output_block(&self, block: &str) -> ToonResult<Value> {
        decode_block_payload(block, &self.output)
    }
}

impl PackedFieldAlias {
    fn encode_dsl(&self) -> String {
        let marker = if self.required { "!" } else { "?" };
        let mut rendered = format!("{}{}:{}", self.alias, marker, self.name);
        if let Some(default) = &self.default {
            rendered.push('=');
            rendered.push_str(&render_scalar_literal(default));
        }
        rendered
    }
}

enum PackedPayload {
    Empty,
    Scalar(String),
    Inline(String),
    Block(String),
}

pub fn encode_packed_tool_catalog_dsl(catalog: &PackedToolCatalog) -> String {
    catalog.encode_dsl()
}

pub fn encode_dx_serializer_packed_catalog(catalog: &PackedToolCatalog) -> String {
    catalog.encode_dx_serializer()
}

pub fn encode_packed_conversation_dsl(
    conversation: &AgentConversation,
    packed: &PackedToolCatalog,
) -> ToonResult<String> {
    let mut chunks = Vec::new();
    let literal_aliases = build_literal_aliases(conversation, packed);
    let call_id_aliases = build_call_id_aliases(conversation);
    let mut call_alias_by_id = HashMap::new();

    if !literal_aliases.is_empty() {
        let mut literal_entries = literal_aliases
            .iter()
            .map(|(value, alias)| (alias.clone(), value.clone()))
            .collect::<Vec<_>>();
        literal_entries.sort_by(|left, right| left.0.cmp(&right.0));
        chunks.extend(literal_entries.into_iter().map(|(alias, value)| {
            format!(
                "@l {}={}",
                alias,
                render_scalar_literal(&Value::String(value))
            )
        }));
    }

    if !call_id_aliases.is_empty() {
        let mut call_id_entries = call_id_aliases
            .iter()
            .map(|(id, alias)| (alias.clone(), id.clone()))
            .collect::<Vec<_>>();
        call_id_entries.sort_by(|left, right| left.0.cmp(&right.0));
        chunks.extend(call_id_entries.into_iter().map(|(alias, id)| {
            format!(
                "@i {}={}",
                alias,
                render_scalar_literal(&Value::String(id))
            )
        }));
    }

    for turn in &conversation.turns {
        match turn {
            AgentTurn::Message(message) => {
                if message.content.contains('\n') {
                    chunks.push(format!(
                        "{}>>>\n{}\n<<<",
                        role_marker(message.role),
                        message.content
                    ));
                } else {
                    chunks.push(format!("{}> {}", role_marker(message.role), message.content));
                }
            }
            AgentTurn::ToolCall { id, tool, args } => {
                let packed_id = call_id_aliases
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.clone());
                if let Some(spec) = packed.tool_by_name(tool) {
                    call_alias_by_id.insert(id.clone(), spec.alias.clone());
                    let omit_alias = packed.sole_tool().is_some_and(|sole| sole.name == spec.name);
                    match spec.encode_input_payload(args, &literal_aliases)? {
                        PackedPayload::Empty => {
                            if omit_alias {
                                chunks.push(format!(">#{packed_id}"));
                            } else {
                                chunks.push(format!(">#{packed_id} {}", spec.alias));
                            }
                        }
                        PackedPayload::Scalar(value) => {
                            if omit_alias {
                                chunks.push(format!(">#{packed_id}={value}"));
                            } else {
                                chunks.push(format!(">#{packed_id} {}={value}", spec.alias));
                            }
                        }
                        PackedPayload::Inline(values) => {
                            if omit_alias {
                                chunks.push(format!(">#{packed_id}({values})"));
                            } else {
                                chunks.push(format!(">#{packed_id} {}({values})", spec.alias));
                            }
                        }
                        PackedPayload::Block(block) => {
                            let header = if omit_alias {
                                format!(">#{packed_id}")
                            } else {
                                format!(">#{packed_id} {}", spec.alias)
                            };
                            chunks.push(format!("{header}:\n{}", indent_block(&block, 2)));
                        }
                    }
                } else {
                    return Err(ToonError::InvalidInput(format!(
                        "No packed alias registered for tool '{tool}'"
                    )));
                }
            }
            AgentTurn::ToolResult {
                id,
                tool,
                status,
                result,
            } => {
                let symbol = match status {
                    ToolResultStatus::Ok => '+',
                    ToolResultStatus::Error => '!',
                };
                let packed_id = call_id_aliases
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.clone());

                match tool {
                    Some(tool_name) => {
                        let spec = packed.tool_by_name(tool_name).ok_or_else(|| {
                            ToonError::InvalidInput(format!(
                                "No packed alias registered for tool '{tool_name}'"
                            ))
                        })?;
                        let omit_alias = call_alias_by_id.get(id) == Some(&spec.alias);

                        match spec.encode_output_payload(result, &literal_aliases)? {
                            PackedPayload::Empty => {
                                if omit_alias {
                                    chunks.push(format!("<#{packed_id}{symbol}"));
                                } else {
                                    chunks.push(format!("<#{packed_id}{symbol} {}", spec.alias));
                                }
                            }
                            PackedPayload::Scalar(value) => {
                                if omit_alias {
                                    chunks.push(format!("<#{packed_id}{symbol}={value}"));
                                } else {
                                    chunks.push(format!("<#{packed_id}{symbol} {}={value}", spec.alias));
                                }
                            }
                            PackedPayload::Inline(values) => {
                                if omit_alias {
                                    chunks.push(format!("<#{packed_id}{symbol}({values})"));
                                } else {
                                    chunks.push(format!("<#{packed_id}{symbol} {}({values})", spec.alias));
                                }
                            }
                            PackedPayload::Block(block) => {
                                let header = if omit_alias {
                                    format!("<#{packed_id}{symbol}")
                                } else {
                                    format!("<#{packed_id}{symbol} {}", spec.alias)
                                };
                                chunks.push(format!("{header}:\n{}", indent_block(&block, 2)));
                            }
                        }
                    }
                    None => {
                        let header = format!("<#{packed_id}{symbol}");
                        if is_effectively_empty(result) {
                            chunks.push(header);
                        } else {
                            let block = encode_default(result)?;
                            chunks.push(format!("{header}:\n{}", indent_block(&block, 2)));
                        }
                    }
                }
            }
        }
    }

    Ok(chunks.join("\n\n"))
}

pub fn encode_dx_serializer_packed_conversation(
    conversation: &AgentConversation,
    packed: &PackedToolCatalog,
) -> ToonResult<String> {
    encode_packed_conversation_dsl(conversation, packed)
}

pub fn encode_dx_serializer_packed_conversation_with_registry_ref(
    conversation: &AgentConversation,
    packed: &PackedToolCatalog,
    registry: &DxSerializerRegistryRef,
) -> ToonResult<String> {
    let body = encode_packed_conversation_dsl(conversation, packed)?;
    Ok(format!("{}\n\n{body}", registry.encode_dsl()))
}

pub fn decode_packed_conversation_dsl(
    input: &str,
    packed: &PackedToolCatalog,
) -> ToonResult<AgentConversation> {
    let lines: Vec<&str> = input.lines().collect();
    let mut turns = Vec::new();
    let (literal_table, call_id_table, _, mut index) = parse_header_tables(&lines)?;
    let mut seen_calls = HashMap::new();

    while index < lines.len() {
        let trimmed = lines[index].trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }

        if let Some((role, content, consumed)) = parse_message_turn(&lines, index)? {
            turns.push(AgentTurn::Message(AgentMessage { role, content }));
            index = consumed;
            continue;
        }

        if trimmed.starts_with("C#") || trimmed.starts_with(">#") {
            let (turn, consumed) =
                parse_packed_tool_call_turn(&lines, index, packed, &literal_table, &call_id_table)?;
            if let AgentTurn::ToolCall { id, tool, .. } = &turn {
                seen_calls.insert(id.clone(), tool.clone());
            }
            turns.push(turn);
            index = consumed;
            continue;
        }

        if trimmed.starts_with("T#") || trimmed.starts_with("<#") {
            let (turn, consumed) = parse_packed_tool_result_turn(
                &lines,
                index,
                packed,
                &literal_table,
                &call_id_table,
                &seen_calls,
            )?;
            turns.push(turn);
            index = consumed;
            continue;
        }

        return Err(ToonError::InvalidInput(format!(
            "Unrecognized packed conversation line: {trimmed}"
        )));
    }

    Ok(AgentConversation { turns })
}

pub fn decode_dx_serializer_packed_conversation(
    input: &str,
    packed: &PackedToolCatalog,
) -> ToonResult<AgentConversation> {
    decode_packed_conversation_dsl(input, packed)
}

pub fn decode_dx_serializer_packed_conversation_with_registry_ref(
    input: &str,
    packed: &PackedToolCatalog,
) -> ToonResult<(Option<DxSerializerRegistryRef>, AgentConversation)> {
    let lines: Vec<&str> = input.lines().collect();
    let (_, _, registry_ref, _) = parse_header_tables(&lines)?;
    let conversation = decode_packed_conversation_dsl(input, packed)?;
    Ok((registry_ref, conversation))
}

fn parse_packed_tool_call_turn(
    lines: &[&str],
    index: usize,
    packed: &PackedToolCatalog,
    literals: &HashMap<String, String>,
    call_ids: &HashMap<String, String>,
) -> ToonResult<(AgentTurn, usize)> {
    let trimmed = lines[index].trim();
    let header = if let Some(rest) = trimmed.strip_prefix(">#") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("C#") {
        rest
    } else {
        return Err(ToonError::InvalidInput(format!("Invalid packed tool call line: {trimmed}")));
    };

    let mut parts = header.trim().split_whitespace();
    let id_token = parts
        .next()
        .ok_or_else(|| ToonError::InvalidInput(format!("Packed tool call missing id: {trimmed}")))?;
    let id = resolve_call_id(id_token, call_ids);
    let remainder = header.trim()[id_token.len()..].trim();
    let (alias, inline_args, has_block, alias_omitted) = if remainder.is_empty() {
        (None, None, false, true)
    } else if remainder == ":" {
        (None, None, true, true)
    } else if let Some(stripped) = remainder.strip_prefix('=') {
        (None, Some(stripped.trim().to_string()), false, true)
    } else if remainder.starts_with('(') {
        if !remainder.ends_with(')') {
            return Err(ToonError::InvalidInput(format!(
                "Packed inline tool-call arguments must end with ')': {trimmed}"
            )));
        }
        (
            None,
            Some(remainder[1..remainder.len() - 1].trim().to_string()),
            false,
            true,
        )
    } else if let Some(stripped) = remainder.strip_suffix(':') {
        (Some(stripped.trim().to_string()), None, true, false)
    } else if let Some((alias, scalar)) = split_once_top_level(remainder, '=') {
        (Some(alias.trim().to_string()), Some(scalar.trim().to_string()), false, false)
    } else if let Some(open_index) = remainder.find('(') {
        if !remainder.ends_with(')') {
            return Err(ToonError::InvalidInput(format!(
                "Packed inline tool-call arguments must end with ')': {trimmed}"
            )));
        }
        (
            Some(remainder[..open_index].trim().to_string()),
            Some(remainder[open_index + 1..remainder.len() - 1].trim().to_string()),
            false,
            false,
        )
    } else {
        (Some(remainder.to_string()), None, false, false)
    };

    let spec = if alias_omitted {
        packed.sole_tool().ok_or_else(|| {
            ToonError::InvalidInput(format!(
                "Packed tool call omitted the alias but the registry does not have exactly one tool: {trimmed}"
            ))
        })?
    } else {
        let alias = alias.ok_or_else(|| {
            ToonError::InvalidInput(format!("Packed tool call missing tool alias: {trimmed}"))
        })?;
        packed.tool_by_alias(&alias).ok_or_else(|| {
            ToonError::InvalidInput(format!("Unknown packed tool alias '{alias}'"))
        })?
    };

    let (args, consumed) = if has_block {
        let (block, consumed) = collect_indented_block(lines, index + 1)?;
        (spec.decode_input_block(&block)?, consumed)
    } else if let Some(inline) = inline_args {
        (spec.decode_input_inline(&inline, literals)?, index + 1)
    } else {
        (json!({}), index + 1)
    };

    Ok((
        AgentTurn::ToolCall {
            id,
            tool: spec.name.clone(),
            args,
        },
        consumed,
    ))
}

fn parse_packed_tool_result_turn(
    lines: &[&str],
    index: usize,
    packed: &PackedToolCatalog,
    literals: &HashMap<String, String>,
    call_ids: &HashMap<String, String>,
    seen_calls: &HashMap<String, String>,
) -> ToonResult<(AgentTurn, usize)> {
    let trimmed = lines[index].trim();
    let header = if let Some(rest) = trimmed.strip_prefix("<#") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("T#") {
        rest
    } else {
        return Err(ToonError::InvalidInput(format!("Invalid packed tool result line: {trimmed}")));
    };

    let plus_index = header.find('+');
    let bang_index = header.find('!');
    let status_index = match (plus_index, bang_index) {
        (Some(p), Some(b)) => Some(p.min(b)),
        (Some(p), None) => Some(p),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
    .ok_or_else(|| {
        ToonError::InvalidInput(format!(
            "Packed tool result must contain '+' or '!' after the id: {trimmed}"
        ))
    })?;

    let id_token = header[..status_index].trim();
    if id_token.is_empty() {
        return Err(ToonError::InvalidInput(format!(
            "Packed tool result missing id: {trimmed}"
        )));
    }
    let id = resolve_call_id(id_token, call_ids);

    let status = match header.as_bytes()[status_index] as char {
        '+' => ToolResultStatus::Ok,
        '!' => ToolResultStatus::Error,
        _ => unreachable!(),
    };

    let remainder = header[status_index + 1..].trim();
    let (tool, result, consumed) = if remainder.is_empty() {
        (seen_calls.get(&id).cloned(), json!({}), index + 1)
    } else {
        let (alias, inline_result, has_block, alias_omitted) =
            if remainder == ":" {
                (None, None, true, true)
            } else if let Some(stripped) = remainder.strip_prefix('=') {
                (None, Some(stripped.trim().to_string()), false, true)
            } else if remainder.starts_with('(') {
                if !remainder.ends_with(')') {
                    return Err(ToonError::InvalidInput(format!(
                        "Packed inline tool-result payload must end with ')': {trimmed}"
                    )));
                }
                (
                    None,
                    Some(remainder[1..remainder.len() - 1].trim().to_string()),
                    false,
                    true,
                )
            } else if let Some((alias, scalar)) = split_once_top_level(remainder, '=') {
                (Some(alias.trim().to_string()), Some(scalar.trim().to_string()), false, false)
            } else if let Some(stripped) = remainder.strip_suffix(':') {
                (Some(stripped.trim().to_string()), None, true, false)
            } else if let Some(open_index) = remainder.find('(') {
                if !remainder.ends_with(')') {
                    return Err(ToonError::InvalidInput(format!(
                        "Packed inline tool-result payload must end with ')': {trimmed}"
                    )));
                }
                (
                    Some(remainder[..open_index].trim().to_string()),
                    Some(remainder[open_index + 1..remainder.len() - 1].trim().to_string()),
                    false,
                    false,
                )
            } else {
                (Some(remainder.to_string()), None, false, false)
            };

        let inferred_tool_name = seen_calls
            .get(&id)
            .cloned()
            .or_else(|| packed.sole_tool().map(|tool| tool.name.clone()));
        let (tool_name, result, consumed) = if alias_omitted {
            if let Some(tool_name) = inferred_tool_name {
                let spec = packed.tool_by_name(&tool_name).ok_or_else(|| {
                    ToonError::InvalidInput(format!(
                        "No packed alias registered for inferred tool '{tool_name}'"
                    ))
                })?;

                let (result, consumed) = if has_block {
                    let (block, consumed) = collect_indented_block(lines, index + 1)?;
                    (spec.decode_output_block(&block)?, consumed)
                } else if let Some(inline) = inline_result {
                    (spec.decode_output_inline(&inline, literals)?, index + 1)
                } else {
                    (json!({}), index + 1)
                };

                (Some(tool_name), result, consumed)
            } else if has_block {
                let (block, consumed) = collect_indented_block(lines, index + 1)?;
                (None, decode_default(&block)?, consumed)
            } else {
                return Err(ToonError::InvalidInput(format!(
                    "Packed tool result omitted the alias but no earlier call with id '{id}' was found"
                )));
            }
        } else {
            let alias = alias.ok_or_else(|| {
                ToonError::InvalidInput(format!(
                    "Packed tool result is missing an alias: {trimmed}"
                ))
            })?;
            let spec = packed.tool_by_alias(&alias).ok_or_else(|| {
                ToonError::InvalidInput(format!("Unknown packed tool alias '{alias}'"))
            })?;
            let (result, consumed) = if has_block {
                let (block, consumed) = collect_indented_block(lines, index + 1)?;
                (spec.decode_output_block(&block)?, consumed)
            } else if let Some(inline) = inline_result {
                (spec.decode_output_inline(&inline, literals)?, index + 1)
            } else {
                (json!({}), index + 1)
            };

            (Some(spec.name.clone()), result, consumed)
        };

        (tool_name, result, consumed)
    };

    Ok((
        AgentTurn::ToolResult {
            id,
            tool,
            status,
            result,
        },
        consumed,
    ))
}

fn decode_inline_payload(
    inline: &str,
    fields: &[PackedFieldAlias],
    literals: &HashMap<String, String>,
) -> ToonResult<Value> {
    let trimmed = inline.trim();
    if trimmed.is_empty() {
        return Ok(json!({}));
    }

    let parts = split_top_level(trimmed, ',');
    if parts.len() > fields.len() {
        return Err(ToonError::InvalidInput(format!(
            "Packed inline payload has {} values but only {} fields are defined",
            parts.len(),
            fields.len()
        )));
    }
    let mut object = Map::new();

    for (index, field) in fields.iter().enumerate() {
        let Some(raw) = parts.get(index) else {
            if let Some(default) = &field.default {
                object.insert(field.name.clone(), default.clone());
                continue;
            }
            if field.required {
                return Err(ToonError::InvalidInput(format!(
                    "Missing required packed positional value for '{}'",
                    field.name
                )));
            }
            continue;
        };

        let token = raw.trim();
        if token.is_empty() || token == "_" {
            if let Some(default) = &field.default {
                object.insert(field.name.clone(), default.clone());
            } else if field.required {
                return Err(ToonError::InvalidInput(format!(
                    "Missing required packed positional value for '{}'",
                    field.name
                )));
            }
            continue;
        }

        object.insert(field.name.clone(), parse_packed_scalar(token, literals)?);
    }

    Ok(Value::Object(object))
}

fn decode_block_payload(block: &str, fields: &[PackedFieldAlias]) -> ToonResult<Value> {
    let decoded: Value = decode_default(block)?;
    let Value::Object(object) = decoded else {
        return Err(ToonError::InvalidInput(
            "Packed block payload must decode to an object".to_string(),
        ));
    };

    let mut expanded = Map::new();
    for (key, value) in object {
        if let Some(field) = fields.iter().find(|field| field.alias == key) {
            expanded.insert(field.name.clone(), value);
        } else {
            expanded.insert(key, value);
        }
    }

    for field in fields {
        if !expanded.contains_key(&field.name) {
            if let Some(default) = &field.default {
                expanded.insert(field.name.clone(), default.clone());
            } else if field.required {
                return Err(ToonError::InvalidInput(format!(
                    "Packed payload block is missing required field '{}'",
                    field.name
                )));
            }
        }
    }

    Ok(Value::Object(expanded))
}

fn build_literal_aliases(conversation: &AgentConversation, packed: &PackedToolCatalog) -> HashMap<String, String> {
    let mut counts = HashMap::<String, usize>::new();

    for turn in &conversation.turns {
        match turn {
            AgentTurn::ToolCall { tool, args, .. } => {
                if let Some(spec) = packed.tool_by_name(tool) {
                    collect_inline_literal_candidates(args, &spec.input, &mut counts);
                }
            }
            AgentTurn::ToolResult {
                tool: Some(tool),
                result,
                ..
            } => {
                if let Some(spec) = packed.tool_by_name(tool) {
                    collect_inline_literal_candidates(result, &spec.output, &mut counts);
                }
            }
            _ => {}
        }
    }

    let mut selected = Vec::new();
    let mut preview_index = 0usize;
    let mut ordered_counts = counts.into_iter().collect::<Vec<_>>();
    ordered_counts.sort_by(|left, right| left.0.cmp(&right.0));

    for (value, count) in ordered_counts {
        if should_pack_literal(&value, count, preview_index) {
            selected.push((value, count));
            preview_index += 1;
        }
    }

    let mut aliases = HashMap::new();
    for (index, (value, _)) in selected.into_iter().enumerate() {
        aliases.insert(value, encode_symbol(index));
    }

    aliases
}

fn build_call_id_aliases(conversation: &AgentConversation) -> HashMap<String, String> {
    let mut counts = HashMap::<String, usize>::new();
    let mut ordered = Vec::<String>::new();

    for turn in &conversation.turns {
        let id = match turn {
            AgentTurn::ToolCall { id, .. } | AgentTurn::ToolResult { id, .. } => id,
            AgentTurn::Message(_) => continue,
        };

        if !counts.contains_key(id) {
            ordered.push(id.clone());
        }
        *counts.entry(id.clone()).or_insert(0) += 1;
    }

    let reserved = counts.keys().cloned().collect::<HashSet<_>>();
    let mut aliases = HashMap::new();
    let mut alias_cursor = 0usize;

    for id in ordered {
        let count = counts.get(&id).copied().unwrap_or_default();
        let (alias, next_cursor) = next_available_generated_alias(alias_cursor, &reserved);
        if should_pack_call_id(&id, count, &alias) {
            aliases.insert(id, alias);
            alias_cursor = next_cursor;
        }
    }

    aliases
}

fn collect_inline_literal_candidates(
    payload: &Value,
    fields: &[PackedFieldAlias],
    counts: &mut HashMap<String, usize>,
) {
    let Value::Object(object) = payload else {
        return;
    };

    if object.keys().any(|key| fields.iter().all(|field| field.name != *key)) {
        return;
    }

    for field in fields {
        let Some(value) = object.get(&field.name) else {
            if field.required && field.default.is_none() {
                return;
            }
            continue;
        };

        if !is_inline_scalar(value) {
            return;
        }
    }

    for field in fields {
        if let Some(Value::String(text)) = object.get(&field.name) {
            *counts.entry(text.clone()).or_insert(0) += 1;
        }
    }
}

fn should_pack_literal(value: &str, count: usize, alias_index: usize) -> bool {
    if count < 2 {
        return false;
    }

    let rendered = render_scalar_literal(&Value::String(value.to_string()));
    let alias = encode_symbol(alias_index);
    let alias_cost = alias.len() + 1;
    let header_cost = 6 + alias.len() + rendered.len();
    let total_savings = (rendered.len().saturating_sub(alias_cost) * count) as isize - header_cost as isize;
    total_savings > 0
}

fn should_pack_call_id(id: &str, count: usize, alias: &str) -> bool {
    if count == 0 {
        return false;
    }

    if needs_call_id_alias(id) {
        return true;
    }

    if count < 2 {
        return false;
    }

    let rendered = render_scalar_literal(&Value::String(id.to_string()));
    let header_cost = 6 + alias.len() + rendered.len();
    let total_savings = (id.len().saturating_sub(alias.len()) * count) as isize - header_cost as isize;
    total_savings > 0
}

fn needs_call_id_alias(id: &str) -> bool {
    !id.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn encode_symbol(mut index: usize) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let base = ALPHABET.len();
    let mut chars = Vec::new();

    loop {
        chars.push(ALPHABET[index % base] as char);
        if index < base {
            break;
        }
        index = (index / base) - 1;
    }

    chars.iter().rev().collect()
}

fn next_available_generated_alias(mut cursor: usize, reserved: &HashSet<String>) -> (String, usize) {
    loop {
        let alias = encode_symbol(cursor);
        cursor += 1;
        if !reserved.contains(&alias) {
            return (alias, cursor);
        }
    }
}

fn parse_header_tables(
    lines: &[&str],
) -> ToonResult<(
    HashMap<String, String>,
    HashMap<String, String>,
    Option<DxSerializerRegistryRef>,
    usize,
)> {
    let mut literals = HashMap::new();
    let mut call_ids = HashMap::new();
    let mut registry_ref = None;
    let mut index = 0usize;

    while index < lines.len() {
        let trimmed = lines[index].trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("@l ").or_else(|| trimmed.strip_prefix("@lit ")) {
            let (alias, literal) = split_once_top_level(rest, '=').ok_or_else(|| {
                ToonError::InvalidInput(format!(
                    "Packed literal table entries must use @lit alias=value syntax: {trimmed}"
                ))
            })?;
            let value = parse_scalar_literal(&literal)?;
            let Value::String(text) = value else {
                return Err(ToonError::InvalidInput(format!(
                    "Packed literal table entries must decode to strings: {trimmed}"
                )));
            };
            literals.insert(alias.trim().to_string(), text);
            index += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@i ").or_else(|| trimmed.strip_prefix("@cid ")) {
            let (alias, literal) = split_once_top_level(rest, '=').ok_or_else(|| {
                ToonError::InvalidInput(format!(
                    "Packed call-id entries must use @cid alias=value syntax: {trimmed}"
                ))
            })?;
            let value = parse_scalar_literal(&literal)?;
            let Value::String(text) = value else {
                return Err(ToonError::InvalidInput(format!(
                    "Packed call-id entries must decode to strings: {trimmed}"
                )));
            };
            call_ids.insert(alias.trim().to_string(), text);
            index += 1;
            continue;
        }

        if trimmed.starts_with("@dxs ") || trimmed.starts_with("@dx-serializer use ") {
            registry_ref = Some(decode_dx_serializer_registry_ref(trimmed)?);
            index += 1;
            continue;
        }

        break;
    }

    Ok((literals, call_ids, registry_ref, index))
}

fn resolve_call_id(token: &str, call_ids: &HashMap<String, String>) -> String {
    call_ids
        .get(token)
        .cloned()
        .unwrap_or_else(|| token.to_string())
}

fn render_packed_scalar(value: &Value, literals: &HashMap<String, String>) -> ToonResult<String> {
    match value {
        Value::Null => Ok("~".to_string()),
        Value::Bool(true) => Ok("+".to_string()),
        Value::Bool(false) => Ok("-".to_string()),
        Value::Number(number) => Ok(number.to_string()),
        Value::String(text) => {
            if let Some(alias) = literals.get(text) {
                Ok(format!("^{alias}"))
            } else {
                Ok(render_scalar_literal(value))
            }
        }
        other => Err(ToonError::InvalidInput(format!(
            "Packed scalar renderer does not support nested value: {other}"
        ))),
    }
}

fn parse_packed_scalar(token: &str, literals: &HashMap<String, String>) -> ToonResult<Value> {
    match token {
        "~" => Ok(Value::Null),
        "+" => Ok(Value::Bool(true)),
        "-" => Ok(Value::Bool(false)),
        _ if token.starts_with('^') => {
            let alias = &token[1..];
            let value = literals.get(alias).ok_or_else(|| {
                ToonError::InvalidInput(format!(
                    "Unknown packed literal alias '^{}'",
                    alias
                ))
            })?;
            Ok(Value::String(value.clone()))
        }
        _ => parse_scalar_literal(token),
    }
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
            "Indented packed payload expected after tool header".to_string(),
        ));
    }

    while collected.last().is_some_and(String::is_empty) {
        collected.pop();
    }

    Ok((collected.join("\n"), cursor))
}

fn parse_message_turn(
    lines: &[&str],
    index: usize,
) -> ToonResult<Option<(AgentRole, String, usize)>> {
    let trimmed = lines[index].trim();
    let mut chars = trimmed.chars();
    let Some(marker) = chars.next() else {
        return Ok(None);
    };
    let Some(role) = role_from_marker(marker) else {
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

fn role_marker(role: AgentRole) -> char {
    match role {
        AgentRole::System => 'S',
        AgentRole::Developer => 'D',
        AgentRole::User => 'U',
        AgentRole::Assistant => 'A',
        AgentRole::Reasoning => 'R',
    }
}

fn role_from_marker(marker: char) -> Option<AgentRole> {
    match marker {
        'S' => Some(AgentRole::System),
        'D' => Some(AgentRole::Developer),
        'U' => Some(AgentRole::User),
        'A' => Some(AgentRole::Assistant),
        'R' => Some(AgentRole::Reasoning),
        _ => None,
    }
}

fn build_field_aliases(fields: &[super::SchemaField]) -> Vec<PackedFieldAlias> {
    let mut used_aliases = HashSet::new();
    fields
        .iter()
        .map(|field| PackedFieldAlias {
            alias: next_unique_alias(&field.name, &mut used_aliases),
            name: field.name.clone(),
            required: field.required,
            default: field.default.clone(),
        })
        .collect()
}

fn next_unique_alias(name: &str, used: &mut HashSet<String>) -> String {
    let base = base_alias(name);
    if used.insert(base.clone()) {
        return base;
    }

    let mut counter = 2usize;
    loop {
        let candidate = format!("{base}{counter}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        counter += 1;
    }
}

fn base_alias(name: &str) -> String {
    let segments = name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    if segments.len() > 1 {
        let alias = segments
            .iter()
            .filter_map(|segment| segment.chars().next())
            .collect::<String>()
            .to_ascii_lowercase();
        if !alias.is_empty() {
            return alias;
        }
    }

    name.chars()
        .find(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase().to_string())
        .unwrap_or_else(|| "x".to_string())
}

fn trim_omittable_tail(values: &mut Vec<Option<String>>) {
    while values.last().is_some_and(Option::is_none) {
        values.pop();
    }
}

fn is_inline_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn is_effectively_empty(value: &Value) -> bool {
    matches!(value, Value::Object(object) if object.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_catalog_derives_short_aliases() {
        let catalog = AgentToolCatalog {
            tools: vec![
                super::super::AgentToolSpec {
                    name: "read_file".to_string(),
                    description: None,
                    input: vec![
                        super::super::SchemaField {
                            name: "path".to_string(),
                            schema_type: super::super::SchemaType::String,
                            required: true,
                            description: None,
                            default: None,
                        },
                        super::super::SchemaField {
                            name: "encoding".to_string(),
                            schema_type: super::super::SchemaType::String,
                            required: false,
                            description: None,
                            default: Some(json!("utf8")),
                        },
                    ],
                    output: vec![super::super::SchemaField {
                        name: "text".to_string(),
                        schema_type: super::super::SchemaType::String,
                        required: true,
                        description: None,
                        default: None,
                    }],
                    annotations: Default::default(),
                },
            ],
        };

        let packed = catalog.to_packed_catalog();
        assert_eq!(packed.tools[0].alias, "rf");
        assert_eq!(packed.tools[0].input[0].alias, "p");
        assert_eq!(packed.tools[0].input[1].alias, "e");
    }

    #[test]
    fn packed_conversation_round_trip_uses_positional_payloads() {
        let catalog = super::super::decode_tool_catalog_dsl(
            "@tool read_file(path!:s,encoding:s=utf8)->(text!:s,mime:s)|Read a file",
        )
        .unwrap();
        let packed = catalog.to_packed_catalog();

        let conversation = AgentConversation {
            turns: vec![
                AgentTurn::Message(AgentMessage {
                    role: AgentRole::User,
                    content: "Summarize src/lib.rs".to_string(),
                }),
                AgentTurn::ToolCall {
                    id: "c1".to_string(),
                    tool: "read_file".to_string(),
                    args: json!({"path": "src/lib.rs", "encoding": "utf8"}),
                },
                AgentTurn::ToolResult {
                    id: "c1".to_string(),
                    tool: Some("read_file".to_string()),
                    status: ToolResultStatus::Ok,
                    result: json!({"text": "pub mod llm;", "mime": "text/plain"}),
                },
            ],
        };

        let encoded = encode_packed_conversation_dsl(&conversation, &packed).unwrap();
        assert!(encoded.contains(">#c1=src/lib.rs"));
        assert!(encoded.contains("<#c1+(\"pub mod llm;\",text/plain)"));

        let decoded = decode_packed_conversation_dsl(&encoded, &packed).unwrap();
        assert_eq!(decoded, conversation);
    }

    #[test]
    fn packed_conversation_falls_back_to_block_for_unknown_fields_without_losing_them() {
        let catalog = super::super::decode_tool_catalog_dsl(
            "@tool run_task(task!:s)->(ok!:b)|Run a task",
        )
        .unwrap();
        let packed = catalog.to_packed_catalog();

        let conversation = AgentConversation {
            turns: vec![AgentTurn::ToolCall {
                id: "c1".to_string(),
                tool: "run_task".to_string(),
                args: json!({"task": "index workspace", "priority": "high"}),
            }],
        };

        let encoded = encode_packed_conversation_dsl(&conversation, &packed).unwrap();
        assert!(encoded.contains(">#c1:"));
        assert!(encoded.contains("t: index workspace"));
        assert!(encoded.contains("priority: high"));

        let decoded = decode_packed_conversation_dsl(&encoded, &packed).unwrap();
        assert_eq!(decoded, conversation);
    }

    #[test]
    fn packed_conversation_uses_literal_dictionary_for_repeated_long_strings() {
        let catalog = super::super::decode_tool_catalog_dsl(
            "@tool read_file(path!:s)->(text!:s)|Read a file",
        )
        .unwrap();
        let packed = catalog.to_packed_catalog();

        let conversation = AgentConversation {
            turns: vec![
                AgentTurn::ToolCall {
                    id: "c1".to_string(),
                    tool: "read_file".to_string(),
                    args: json!({"path": "src/features/editor/very_long_file_name.rs"}),
                },
                AgentTurn::ToolCall {
                    id: "c2".to_string(),
                    tool: "read_file".to_string(),
                    args: json!({"path": "src/features/editor/very_long_file_name.rs"}),
                },
            ],
        };

        let encoded = encode_packed_conversation_dsl(&conversation, &packed).unwrap();
        assert!(encoded.contains("@l a=src/features/editor/very_long_file_name.rs"));
        assert!(encoded.contains(">#c1=^a"));
        assert!(encoded.contains(">#c2=^a"));

        let decoded = decode_packed_conversation_dsl(&encoded, &packed).unwrap();
        assert_eq!(decoded, conversation);
    }

    #[test]
    fn packed_conversation_uses_literal_dictionary_when_short_strings_repeat_enough() {
        let catalog =
            super::super::decode_tool_catalog_dsl("@tool read_symbol(name!:s)->(ok!:b)|Read symbol")
                .unwrap();
        let packed = catalog.to_packed_catalog();

        let conversation = AgentConversation {
            turns: (0..6)
                .map(|index| AgentTurn::ToolCall {
                    id: format!("c{index}"),
                    tool: "read_symbol".to_string(),
                    args: json!({"name": "rust"}),
                })
                .collect(),
        };

        let encoded = encode_packed_conversation_dsl(&conversation, &packed).unwrap();
        assert!(encoded.contains("@l a=rust"));
        assert!(encoded.contains(">#c0=^a"));

        let decoded = decode_packed_conversation_dsl(&encoded, &packed).unwrap();
        assert_eq!(decoded, conversation);
    }

    #[test]
    fn packed_conversation_aliases_long_call_ids() {
        let catalog = super::super::decode_tool_catalog_dsl(
            "@tool read_file(path!:s)->(text!:s)|Read a file",
        )
        .unwrap();
        let packed = catalog.to_packed_catalog();
        let long_id = "toolu_very_long_call_identifier_0001";

        let conversation = AgentConversation {
            turns: vec![
                AgentTurn::ToolCall {
                    id: long_id.to_string(),
                    tool: "read_file".to_string(),
                    args: json!({"path": "src/lib.rs"}),
                },
                AgentTurn::ToolResult {
                    id: long_id.to_string(),
                    tool: Some("read_file".to_string()),
                    status: ToolResultStatus::Ok,
                    result: json!({"text": "pub mod llm;"}),
                },
            ],
        };

        let encoded = encode_packed_conversation_dsl(&conversation, &packed).unwrap();
        assert!(encoded.contains("@i a=toolu_very_long_call_identifier_0001"));
        assert!(encoded.contains(">#a=src/lib.rs"));
        assert!(encoded.contains("<#a+=\"pub mod llm;\""));

        let decoded = decode_packed_conversation_dsl(&encoded, &packed).unwrap();
        assert_eq!(decoded, conversation);
    }

    #[test]
    fn packed_conversation_call_id_aliases_do_not_collide_with_existing_raw_ids() {
        let catalog =
            super::super::decode_tool_catalog_dsl("@tool read_file(path!:s)->(text!:s)|Read a file")
                .unwrap();
        let packed = catalog.to_packed_catalog();

        let conversation = AgentConversation {
            turns: vec![
                AgentTurn::ToolCall {
                    id: "a".to_string(),
                    tool: "read_file".to_string(),
                    args: json!({"path": "src/short.rs"}),
                },
                AgentTurn::ToolResult {
                    id: "a".to_string(),
                    tool: Some("read_file".to_string()),
                    status: ToolResultStatus::Ok,
                    result: json!({"text": "short"}),
                },
                AgentTurn::ToolCall {
                    id: "toolu_very_long_call_identifier_0001".to_string(),
                    tool: "read_file".to_string(),
                    args: json!({"path": "src/lib.rs"}),
                },
                AgentTurn::ToolResult {
                    id: "toolu_very_long_call_identifier_0001".to_string(),
                    tool: Some("read_file".to_string()),
                    status: ToolResultStatus::Ok,
                    result: json!({"text": "pub mod llm;"}),
                },
            ],
        };

        let encoded = encode_packed_conversation_dsl(&conversation, &packed).unwrap();
        assert!(encoded.contains(">#a=src/short.rs"));
        assert!(encoded.contains("@i b=toolu_very_long_call_identifier_0001"));
        assert!(encoded.contains(">#b=src/lib.rs"));

        let decoded = decode_packed_conversation_dsl(&encoded, &packed).unwrap();
        assert_eq!(decoded, conversation);
    }

    #[test]
    fn packed_conversation_with_registry_ref_round_trips_registry_header() {
        let catalog =
            super::super::decode_tool_catalog_dsl("@tool read_file(path!:s)->(text!:s)|Read a file")
                .unwrap();
        let packed = catalog.to_packed_catalog();
        let registry_ref = catalog.to_dx_serializer_registry_ref();

        let conversation = AgentConversation {
            turns: vec![AgentTurn::ToolCall {
                id: "c1".to_string(),
                tool: "read_file".to_string(),
                args: json!({"path": "src/lib.rs"}),
            }],
        };

        let encoded = encode_dx_serializer_packed_conversation_with_registry_ref(
            &conversation,
            &packed,
            &registry_ref,
        )
        .unwrap();
        assert!(encoded.starts_with("@dxs dxs_"));

        let (decoded_ref, decoded_conversation) =
            decode_dx_serializer_packed_conversation_with_registry_ref(&encoded, &packed).unwrap();
        assert_eq!(decoded_ref, Some(registry_ref));
        assert_eq!(decoded_conversation, conversation);
    }

    #[test]
    fn packed_conversation_single_scalar_outputs_use_equals_form() {
        let catalog =
            super::super::decode_tool_catalog_dsl("@tool read_file(path!:s)->(text!:s)|Read a file")
                .unwrap();
        let packed = catalog.to_packed_catalog();
        let conversation = AgentConversation {
            turns: vec![
                AgentTurn::ToolCall {
                    id: "c1".to_string(),
                    tool: "read_file".to_string(),
                    args: json!({"path": "src/lib.rs"}),
                },
                AgentTurn::ToolResult {
                    id: "c1".to_string(),
                    tool: Some("read_file".to_string()),
                    status: ToolResultStatus::Ok,
                    result: json!({"text": "pub mod llm;"}),
                },
            ],
        };

        let encoded = encode_packed_conversation_dsl(&conversation, &packed).unwrap();
        assert!(encoded.contains(">#c1=src/lib.rs"));
        assert!(encoded.contains("<#c1+=\"pub mod llm;\""));

        let decoded = decode_packed_conversation_dsl(&encoded, &packed).unwrap();
        assert_eq!(decoded, conversation);
    }
}
