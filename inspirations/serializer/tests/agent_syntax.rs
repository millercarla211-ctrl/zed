use serde_json::json;
use serializer::{
    AgentConversation,
    AgentMessage,
    AgentRole,
    AgentTurn,
    PackedToolCatalog,
    ToolProviderTarget,
    ToolResultStatus,
    decode_dx_serializer_conversation,
    decode_dx_serializer_packed_conversation_with_registry_ref,
    decode_dx_serializer_tool_catalog,
    decode_packed_conversation_dsl,
    encode_dx_serializer_conversation,
    encode_dx_serializer_packed_catalog,
    encode_dx_serializer_packed_conversation,
    encode_dx_serializer_packed_conversation_with_registry_ref,
    encode_dx_serializer_registry_ref,
    encode_dx_serializer_tool_catalog,
};

#[test]
fn tool_catalog_dsl_supports_nested_types_and_provider_conversion() {
    let dsl = r#"@tool search_docs(query!:s,limit?:i=5)->(hits!:a<o(title!:s,url!:s,snippet?:s)>)|Search indexed docs
? query: Search query text
? return.hits.title: Search hit title"#;

    let catalog = decode_dx_serializer_tool_catalog(dsl).unwrap();
    assert_eq!(catalog.tools.len(), 1);

    let encoded = encode_dx_serializer_tool_catalog(&catalog);
    assert!(encoded.contains("@tool search_docs("));
    assert!(encoded.contains("? return.hits.title: Search hit title"));

    let openai = catalog.to_openai_tools_json();
    assert_eq!(openai[0]["function"]["name"], json!("search_docs"));
    assert_eq!(
        openai[0]["function"]["parameters"]["properties"]["query"]["type"],
        json!("string")
    );

    let gemini = catalog.to_gemini_tools_json();
    assert_eq!(
        gemini[0]["functionDeclarations"][0]["name"],
        json!("search_docs")
    );

    let mcp = catalog.to_mcp_tools_json();
    assert_eq!(mcp[0]["name"], json!("search_docs"));

    let exported = catalog.export_for(ToolProviderTarget::Anthropic);
    assert_eq!(exported[0]["name"], json!("search_docs"));

    let packed = PackedToolCatalog::from_agent_catalog(&catalog);
    let packed_catalog = encode_dx_serializer_packed_catalog(&packed);
    assert!(packed_catalog.contains("@p"));
    let registry_ref = encode_dx_serializer_registry_ref(&catalog);
    assert!(registry_ref.starts_with("@dxs dxs_"));
}

#[test]
fn conversation_dsl_round_trips_tool_history() {
    let conversation = AgentConversation {
        turns: vec![
            AgentTurn::Message(AgentMessage {
                role: AgentRole::Developer,
                content: "Prefer local tools first.".to_string(),
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
                result: json!({"text": "pub mod llm;"}),
            },
            AgentTurn::Message(AgentMessage {
                role: AgentRole::Assistant,
                content: "The file exports the llm module.".to_string(),
            }),
        ],
    };

    let encoded = encode_dx_serializer_conversation(&conversation).unwrap();
    assert!(encoded.contains("C#c1 read_file(path=src/lib.rs)"));
    assert!(encoded.contains("T#c1 ok read_file(text=\"pub mod llm;\")"));
    let decoded = decode_dx_serializer_conversation(&encoded).unwrap();
    assert_eq!(decoded, conversation);
}

#[test]
fn packed_conversation_dsl_round_trips_tool_history() {
    let catalog = decode_dx_serializer_tool_catalog(
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

    let encoded = encode_dx_serializer_packed_conversation(&conversation, &packed).unwrap();
    assert!(encoded.contains(">#c1=src/lib.rs"));
    assert!(encoded.contains("<#c1+(\"pub mod llm;\",text/plain)"));

    let decoded = decode_packed_conversation_dsl(&encoded, &packed).unwrap();
    assert_eq!(decoded, conversation);
}

#[test]
fn packed_conversation_dsl_aliases_long_runtime_call_ids() {
    let catalog =
        decode_dx_serializer_tool_catalog("@tool read_file(path!:s)->(text!:s)|Read a file").unwrap();
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

    let encoded = encode_dx_serializer_packed_conversation_with_registry_ref(
        &conversation,
        &packed,
        &catalog.to_dx_serializer_registry_ref(),
    )
    .unwrap();
    assert!(encoded.starts_with("@dxs dxs_"));
    assert!(encoded.contains("@i a=toolu_very_long_call_identifier_0001"));
    assert!(encoded.contains(">#a=src/lib.rs"));
    assert!(encoded.contains("<#a+=\"pub mod llm;\""));

    let (registry_ref, decoded) =
        decode_dx_serializer_packed_conversation_with_registry_ref(&encoded, &packed).unwrap();
    assert_eq!(registry_ref, Some(catalog.to_dx_serializer_registry_ref()));
    assert_eq!(decoded, conversation);
}
