use indexmap::IndexMap;
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
    encode_default,
    types::{
        ToonError,
        ToonResult,
    },
};

use super::util::{
    parse_scalar_literal,
    render_scalar_literal,
    split_once_top_level,
    split_once_top_level_str,
    split_top_level,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SchemaType {
    String,
    Integer,
    Number,
    Boolean,
    Null,
    Any,
    Array(Box<SchemaType>),
    Object(Vec<SchemaField>),
    Enum(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub schema_type: SchemaType,
    pub required: bool,
    pub description: Option<String>,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentToolSpec {
    pub name: String,
    pub description: Option<String>,
    pub input: Vec<SchemaField>,
    pub output: Vec<SchemaField>,
    pub annotations: IndexMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolProviderTarget {
    OpenAi,
    Anthropic,
    Gemini,
    Mcp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgentToolCatalog {
    pub tools: Vec<AgentToolSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DxSerializerRegistryRef {
    pub id: String,
    pub tool_count: usize,
}

impl SchemaType {
    pub fn to_json_schema(&self) -> Value {
        match self {
            SchemaType::String => json!({"type": "string"}),
            SchemaType::Integer => json!({"type": "integer"}),
            SchemaType::Number => json!({"type": "number"}),
            SchemaType::Boolean => json!({"type": "boolean"}),
            SchemaType::Null => json!({"type": "null"}),
            SchemaType::Any => json!({}),
            SchemaType::Array(item) => {
                json!({
                    "type": "array",
                    "items": item.to_json_schema(),
                })
            }
            SchemaType::Object(fields) => fields_to_json_schema(fields),
            SchemaType::Enum(values) => {
                json!({
                    "type": "string",
                    "enum": values,
                })
            }
        }
    }

    fn to_dsl(&self) -> String {
        match self {
            SchemaType::String => "s".to_string(),
            SchemaType::Integer => "i".to_string(),
            SchemaType::Number => "n".to_string(),
            SchemaType::Boolean => "b".to_string(),
            SchemaType::Null => "z".to_string(),
            SchemaType::Any => "*".to_string(),
            SchemaType::Array(item) => format!("a<{}>", item.to_dsl()),
            SchemaType::Object(fields) => format!("o({})", encode_field_signature(fields)),
            SchemaType::Enum(values) => format!("e[{}]", values.join("|")),
        }
    }
}

impl SchemaField {
    fn to_dsl(&self) -> String {
        let marker = if self.required { "!" } else { "?" };
        let mut rendered = format!("{}{}:{}", self.name, marker, self.schema_type.to_dsl());
        if let Some(default) = &self.default {
            rendered.push('=');
            rendered.push_str(&render_scalar_literal(default));
        }
        rendered
    }

    fn apply_description_path(&mut self, path: &[&str], description: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        if path[0] != self.name {
            return false;
        }

        if path.len() == 1 {
            self.description = Some(description.to_string());
            return true;
        }

        match &mut self.schema_type {
            SchemaType::Object(fields) => apply_description_to_fields(fields, &path[1..], description),
            SchemaType::Array(item) => {
                if let SchemaType::Object(fields) = item.as_mut() {
                    apply_description_to_fields(fields, &path[1..], description)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

impl AgentToolSpec {
    pub fn input_schema_json(&self) -> Value {
        fields_to_json_schema(&self.input)
    }

    pub fn output_schema_json(&self) -> Value {
        fields_to_json_schema(&self.output)
    }

    pub fn to_openai_tool_json(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.render_provider_description(),
                "strict": true,
                "parameters": self.input_schema_json(),
            }
        })
    }

    pub fn to_anthropic_tool_json(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.render_provider_description(),
            "input_schema": self.input_schema_json(),
        })
    }

    pub fn to_gemini_function_json(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.render_provider_description(),
            "parameters": self.input_schema_json(),
        })
    }

    pub fn to_mcp_tool_json(&self) -> Value {
        let mut object = Map::new();
        object.insert("name".to_string(), Value::String(self.name.clone()));
        object.insert(
            "description".to_string(),
            Value::String(self.render_provider_description()),
        );
        object.insert("inputSchema".to_string(), self.input_schema_json());
        if !self.output.is_empty() {
            object.insert("outputSchema".to_string(), self.output_schema_json());
        }
        Value::Object(object)
    }

    pub fn export_for(&self, target: ToolProviderTarget) -> Value {
        match target {
            ToolProviderTarget::OpenAi => self.to_openai_tool_json(),
            ToolProviderTarget::Anthropic => self.to_anthropic_tool_json(),
            ToolProviderTarget::Gemini => self.to_gemini_function_json(),
            ToolProviderTarget::Mcp => self.to_mcp_tool_json(),
        }
    }

    pub fn encode_dsl(&self) -> String {
        let mut line = format!(
            "@tool {}({})",
            self.name,
            encode_field_signature(&self.input)
        );
        if !self.output.is_empty() {
            line.push_str("->(");
            line.push_str(&encode_field_signature(&self.output));
            line.push(')');
        }
        if let Some(description) = &self.description {
            if !description.trim().is_empty() {
                line.push('|');
                line.push_str(description.trim());
            }
        }

        let mut doc_lines = Vec::new();
        render_field_doc_lines(&self.input, "", false, &mut doc_lines);
        render_field_doc_lines(&self.output, "return.", true, &mut doc_lines);

        if doc_lines.is_empty() {
            line
        } else {
            format!("{line}\n{}", doc_lines.join("\n"))
        }
    }

    pub fn render_provider_description(&self) -> String {
        let mut sections = Vec::new();
        if let Some(description) = &self.description {
            if !description.trim().is_empty() {
                sections.push(description.trim().to_string());
            }
        }

        let mut parameter_lines = Vec::new();
        collect_field_descriptions(&self.input, "", &mut parameter_lines);
        if !parameter_lines.is_empty() {
            sections.push(format!("Parameters:\n{}", parameter_lines.join("\n")));
        }

        let mut output_lines = Vec::new();
        collect_field_descriptions(&self.output, "", &mut output_lines);
        if !output_lines.is_empty() {
            sections.push(format!("Returns:\n{}", output_lines.join("\n")));
        }

        sections.join("\n\n")
    }
}

impl AgentToolCatalog {
    pub fn encode_dx_serializer(&self) -> String {
        self.encode_dsl()
    }

    pub fn to_dx_serializer_registry_ref(&self) -> DxSerializerRegistryRef {
        let encoded = self.encode_dsl();
        DxSerializerRegistryRef {
            id: stable_registry_id(&encoded),
            tool_count: self.tools.len(),
        }
    }

    pub fn to_openai_tools_json(&self) -> Value {
        Value::Array(
            self.tools
                .iter()
                .map(AgentToolSpec::to_openai_tool_json)
                .collect(),
        )
    }

    pub fn to_anthropic_tools_json(&self) -> Value {
        Value::Array(
            self.tools
                .iter()
                .map(AgentToolSpec::to_anthropic_tool_json)
                .collect(),
        )
    }

    pub fn to_gemini_tools_json(&self) -> Value {
        json!([{
            "functionDeclarations": self
                .tools
                .iter()
                .map(AgentToolSpec::to_gemini_function_json)
                .collect::<Vec<_>>(),
        }])
    }

    pub fn to_mcp_tools_json(&self) -> Value {
        Value::Array(
            self.tools
                .iter()
                .map(AgentToolSpec::to_mcp_tool_json)
                .collect(),
        )
    }

    pub fn export_for(&self, target: ToolProviderTarget) -> Value {
        match target {
            ToolProviderTarget::OpenAi => self.to_openai_tools_json(),
            ToolProviderTarget::Anthropic => self.to_anthropic_tools_json(),
            ToolProviderTarget::Gemini => self.to_gemini_tools_json(),
            ToolProviderTarget::Mcp => self.to_mcp_tools_json(),
        }
    }

    pub fn encode_dsl(&self) -> String {
        self.tools
            .iter()
            .map(AgentToolSpec::encode_dsl)
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn encode_json_manifest(&self) -> ToonResult<String> {
        encode_default(&json!({
            "tools": self
                .tools
                .iter()
                .map(|tool| json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input": tool.input_schema_json(),
                    "output": tool.output_schema_json(),
                }))
                .collect::<Vec<_>>()
        }))
    }
}

impl DxSerializerRegistryRef {
    pub fn encode_dsl(&self) -> String {
        format!("@dxs {} t={}", self.id, self.tool_count)
    }
}

pub fn encode_tool_catalog_dsl(catalog: &AgentToolCatalog) -> String {
    catalog.encode_dsl()
}

pub fn encode_dx_serializer_tool_catalog(catalog: &AgentToolCatalog) -> String {
    catalog.encode_dx_serializer()
}

pub fn decode_tool_catalog_dsl(input: &str) -> ToonResult<AgentToolCatalog> {
    let mut tools = Vec::new();
    let mut current_tool: Option<AgentToolSpec> = None;

    for raw_line in input.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@tool ") {
            if let Some(tool) = current_tool.take() {
                tools.push(tool);
            }
            current_tool = Some(parse_tool_header(rest)?);
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("? ") {
            let tool = current_tool
                .as_mut()
                .ok_or_else(|| ToonError::InvalidInput("Field description found before @tool".to_string()))?;
            apply_doc_line(tool, rest)?;
            continue;
        }

        return Err(ToonError::InvalidInput(format!(
            "Unrecognized tool DSL line: {trimmed}"
        )));
    }

    if let Some(tool) = current_tool.take() {
        tools.push(tool);
    }

    Ok(AgentToolCatalog { tools })
}

pub fn decode_dx_serializer_tool_catalog(input: &str) -> ToonResult<AgentToolCatalog> {
    decode_tool_catalog_dsl(input)
}

pub fn encode_dx_serializer_registry_ref(catalog: &AgentToolCatalog) -> String {
    catalog.to_dx_serializer_registry_ref().encode_dsl()
}

pub fn decode_dx_serializer_registry_ref(input: &str) -> ToonResult<DxSerializerRegistryRef> {
    let trimmed = input.trim();
    let rest = if let Some(rest) = trimmed.strip_prefix("@dxs ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("@dx-serializer use ") {
        rest
    } else {
        return Err(ToonError::InvalidInput(format!(
            "dx-serializer registry refs must start with '@dxs' or '@dx-serializer use': {trimmed}"
        )));
    };

    let mut parts = rest.split_whitespace();
    let id = parts
        .next()
        .ok_or_else(|| ToonError::InvalidInput("dx-serializer registry ref is missing an id".to_string()))?;
    let mut tool_count = None;

    for part in parts {
        if let Some(value) = part.strip_prefix("t=").or_else(|| part.strip_prefix("tools=")) {
            tool_count = Some(value.parse::<usize>().map_err(|_| {
                ToonError::InvalidInput(format!(
                    "dx-serializer registry ref has an invalid tool count: {trimmed}"
                ))
            })?);
        }
    }

    Ok(DxSerializerRegistryRef {
        id: id.to_string(),
        tool_count: tool_count.unwrap_or_default(),
    })
}

fn parse_tool_header(input: &str) -> ToonResult<AgentToolSpec> {
    let (signature_part, description) = if let Some((left, right)) = split_once_top_level(input, '|')
    {
        (left, Some(right))
    } else {
        (input.trim().to_string(), None)
    };

    let (header, output_part) = if let Some((left, right)) =
        split_once_top_level_str(&signature_part, "->")
    {
        (left, Some(right.trim().to_string()))
    } else {
        (signature_part, None)
    };

    let open = header
        .find('(')
        .ok_or_else(|| ToonError::InvalidInput(format!("Tool header missing input signature: {header}")))?;
    let close = header
        .rfind(')')
        .ok_or_else(|| ToonError::InvalidInput(format!("Tool header missing closing ')': {header}")))?;
    if close <= open {
        return Err(ToonError::InvalidInput(format!(
            "Invalid tool input signature: {header}"
        )));
    }

    let name = header[..open].trim().to_string();
    let input_fields = parse_field_list(&header[open + 1..close])?;
    let output_fields = if let Some(output_part) = output_part {
        let trimmed = output_part.trim();
        if trimmed.starts_with('(') && trimmed.ends_with(')') {
            parse_field_list(&trimmed[1..trimmed.len() - 1])?
        } else {
            return Err(ToonError::InvalidInput(format!(
                "Output signature must use (...) syntax: {trimmed}"
            )));
        }
    } else {
        Vec::new()
    };

    Ok(AgentToolSpec {
        name,
        description: description.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
        input: input_fields,
        output: output_fields,
        annotations: IndexMap::new(),
    })
}

fn parse_field_list(input: &str) -> ToonResult<Vec<SchemaField>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    split_top_level(trimmed, ',')
        .into_iter()
        .filter(|item| !item.is_empty())
        .map(|item| parse_field(&item))
        .collect()
}

fn parse_field(input: &str) -> ToonResult<SchemaField> {
    let (left, right) = split_once_top_level(input, ':').ok_or_else(|| {
        ToonError::InvalidInput(format!("Field definition must contain ':': {input}"))
    })?;

    let left = left.trim();
    let (name, required) = if let Some(name) = left.strip_suffix('!') {
        (name.trim().to_string(), true)
    } else if let Some(name) = left.strip_suffix('?') {
        (name.trim().to_string(), false)
    } else {
        (left.to_string(), true)
    };

    let (type_part, default) = if let Some((type_part, default_literal)) = split_once_top_level(&right, '=') {
        (type_part, Some(parse_scalar_literal(&default_literal)?))
    } else {
        (right, None)
    };

    Ok(SchemaField {
        name,
        schema_type: parse_schema_type(&type_part)?,
        required,
        description: None,
        default,
    })
}

fn parse_schema_type(input: &str) -> ToonResult<SchemaType> {
    let trimmed = input.trim();
    match trimmed {
        "s" | "str" | "string" => Ok(SchemaType::String),
        "i" | "int" | "integer" => Ok(SchemaType::Integer),
        "n" | "num" | "number" | "float" => Ok(SchemaType::Number),
        "b" | "bool" | "boolean" => Ok(SchemaType::Boolean),
        "z" | "null" => Ok(SchemaType::Null),
        "*" | "any" => Ok(SchemaType::Any),
        _ if trimmed.starts_with("a<") && trimmed.ends_with('>') => Ok(SchemaType::Array(Box::new(
            parse_schema_type(&trimmed[2..trimmed.len() - 1])?,
        ))),
        _ if trimmed.starts_with("o(") && trimmed.ends_with(')') => {
            Ok(SchemaType::Object(parse_field_list(&trimmed[2..trimmed.len() - 1])?))
        }
        _ if trimmed.starts_with("e[") && trimmed.ends_with(']') => Ok(SchemaType::Enum(
            split_top_level(&trimmed[2..trimmed.len() - 1], '|')
                .into_iter()
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect(),
        )),
        _ => Err(ToonError::InvalidInput(format!(
            "Unsupported schema type: {trimmed}"
        ))),
    }
}

fn encode_field_signature(fields: &[SchemaField]) -> String {
    fields
        .iter()
        .map(SchemaField::to_dsl)
        .collect::<Vec<_>>()
        .join(",")
}

fn fields_to_json_schema(fields: &[SchemaField]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for field in fields {
        let mut schema_value = field.schema_type.to_json_schema();
        if let Value::Object(object) = &mut schema_value {
            if let Some(description) = &field.description {
                object.insert("description".to_string(), Value::String(description.clone()));
            }
            if let Some(default) = &field.default {
                object.insert("default".to_string(), default.clone());
            }
        }
        properties.insert(field.name.clone(), schema_value);
        if field.required {
            required.push(Value::String(field.name.clone()));
        }
    }

    let mut object = Map::new();
    object.insert("type".to_string(), Value::String("object".to_string()));
    object.insert("properties".to_string(), Value::Object(properties));
    object.insert("additionalProperties".to_string(), Value::Bool(false));
    if !required.is_empty() {
        object.insert("required".to_string(), Value::Array(required));
    }
    Value::Object(object)
}

fn stable_registry_id(input: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("dxs_{hash:016x}")
}

fn render_field_doc_lines(
    fields: &[SchemaField],
    prefix: &str,
    include_return_prefix: bool,
    output: &mut Vec<String>,
) {
    for field in fields {
        let path = format!("{prefix}{}", field.name);
        if let Some(description) = &field.description {
            let key = if include_return_prefix {
                path.clone()
            } else {
                path.trim_start_matches('.').to_string()
            };
            output.push(format!("? {}: {}", key, description));
        }

        match &field.schema_type {
            SchemaType::Object(nested) => {
                render_field_doc_lines(nested, &format!("{path}."), include_return_prefix, output);
            }
            SchemaType::Array(item) => {
                if let SchemaType::Object(nested) = item.as_ref() {
                    render_field_doc_lines(
                        nested,
                        &format!("{path}."),
                        include_return_prefix,
                        output,
                    );
                }
            }
            _ => {}
        }
    }
}

fn collect_field_descriptions(fields: &[SchemaField], prefix: &str, output: &mut Vec<String>) {
    for field in fields {
        let path = if prefix.is_empty() {
            field.name.clone()
        } else {
            format!("{prefix}.{}", field.name)
        };

        let field_type = field.schema_type.to_dsl();
        let description = field
            .description
            .clone()
            .unwrap_or_else(|| "No extra description.".to_string());
        output.push(format!("- {path} ({field_type}): {description}"));

        match &field.schema_type {
            SchemaType::Object(nested) => collect_field_descriptions(nested, &path, output),
            SchemaType::Array(item) => {
                if let SchemaType::Object(nested) = item.as_ref() {
                    collect_field_descriptions(nested, &path, output);
                }
            }
            _ => {}
        }
    }
}

fn apply_doc_line(tool: &mut AgentToolSpec, input: &str) -> ToonResult<()> {
    let (path, description) = input.split_once(':').ok_or_else(|| {
        ToonError::InvalidInput(format!("Field documentation must contain ':': {input}"))
    })?;
    let path = path.trim();
    let description = description.trim();

    if let Some(annotation_key) = path.strip_prefix("meta.") {
        tool.annotations
            .insert(annotation_key.trim().to_string(), description.to_string());
        return Ok(());
    }

    if let Some(return_path) = path.strip_prefix("return.") {
        if apply_description_to_fields(
            &mut tool.output,
            &return_path.split('.').collect::<Vec<_>>(),
            description,
        ) {
            return Ok(());
        }
    } else if apply_description_to_fields(
        &mut tool.input,
        &path.split('.').collect::<Vec<_>>(),
        description,
    ) {
        return Ok(());
    }

    Err(ToonError::InvalidInput(format!(
        "Documentation path does not match a schema field: {path}"
    )))
}

fn apply_description_to_fields(fields: &mut [SchemaField], path: &[&str], description: &str) -> bool {
    fields
        .iter_mut()
        .any(|field| field.apply_description_path(path, description))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_catalog_round_trip_preserves_schema_and_docs() {
        let dsl = r#"@tool read_file(path!:s,encoding:s=utf8,from?:i,to?:i)->(text!:s,mime:s)|Read UTF-8 text from a workspace file
? path: Workspace-relative file path
? encoding: Text encoding; defaults to utf8
? return.text: File contents

@tool search_docs(query!:s,limit?:i=5)->(hits!:a<o(title!:s,url!:s,snippet?:s)>)|Search indexed docs
? query: Search query text"#;

        let catalog = decode_tool_catalog_dsl(dsl).unwrap();
        assert_eq!(catalog.tools.len(), 2);
        assert_eq!(catalog.tools[0].name, "read_file");
        assert_eq!(catalog.tools[0].input[0].description.as_deref(), Some("Workspace-relative file path"));
        assert_eq!(catalog.tools[1].output[0].name, "hits");

        let encoded = encode_tool_catalog_dsl(&catalog);
        assert!(encoded.contains("@tool read_file("));
        assert!(encoded.contains("? return.text: File contents"));
    }

    #[test]
    fn tool_schema_converts_to_provider_shapes() {
        let catalog = decode_tool_catalog_dsl(
            "@tool open_url(url!:s)->(ok!:b)|Open a URL\n? url: Absolute URL to open",
        )
        .unwrap();
        let tool = &catalog.tools[0];

        let openai = tool.to_openai_tool_json();
        assert_eq!(openai["type"], Value::String("function".to_string()));
        assert_eq!(openai["function"]["name"], Value::String("open_url".to_string()));
        assert_eq!(
            openai["function"]["parameters"]["properties"]["url"]["type"],
            Value::String("string".to_string())
        );

        let anthropic = tool.to_anthropic_tool_json();
        assert_eq!(anthropic["name"], Value::String("open_url".to_string()));
        assert_eq!(
            anthropic["input_schema"]["properties"]["url"]["type"],
            Value::String("string".to_string())
        );

        let gemini = tool.to_gemini_function_json();
        assert_eq!(gemini["name"], Value::String("open_url".to_string()));
        assert_eq!(
            gemini["parameters"]["properties"]["url"]["type"],
            Value::String("string".to_string())
        );

        let mcp = tool.to_mcp_tool_json();
        assert_eq!(mcp["name"], Value::String("open_url".to_string()));
        assert_eq!(
            mcp["inputSchema"]["properties"]["url"]["type"],
            Value::String("string".to_string())
        );

        let exported = tool.export_for(ToolProviderTarget::Gemini);
        assert_eq!(exported["name"], Value::String("open_url".to_string()));
    }

    #[test]
    fn tool_headers_with_hyphenated_names_still_parse_output_signatures() {
        let catalog = decode_tool_catalog_dsl(
            "@tool google-search(query!:s,top_k?:i=5)->(hits!:a<o(title!:s,url!:s)>)|Search the web",
        )
        .unwrap();

        assert_eq!(catalog.tools[0].name, "google-search");
        assert_eq!(catalog.tools[0].output.len(), 1);
        assert_eq!(catalog.tools[0].output[0].name, "hits");

        let exported = catalog.export_for(ToolProviderTarget::OpenAi);
        assert_eq!(
            exported[0]["function"]["name"],
            Value::String("google-search".to_string())
        );
    }

    #[test]
    fn dx_serializer_registry_refs_are_stable_and_round_trip() {
        let catalog =
            decode_tool_catalog_dsl("@tool read_file(path!:s)->(text!:s)|Read a file").unwrap();

        let left = catalog.to_dx_serializer_registry_ref();
        let right = catalog.to_dx_serializer_registry_ref();
        assert_eq!(left, right);
        assert!(left.id.starts_with("dxs_"));

        let decoded = decode_dx_serializer_registry_ref(&left.encode_dsl()).unwrap();
        assert_eq!(decoded, left);
    }
}
