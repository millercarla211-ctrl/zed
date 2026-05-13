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
    encode_dx_serializer_conversation,
    encode_dx_serializer_packed_catalog,
    encode_dx_serializer_packed_conversation_with_registry_ref,
    encode_dx_serializer_registry_ref,
    encode_dx_serializer_tool_catalog,
};

pub fn agent_syntax() {
    let tools = decode_dx_serializer_tool_catalog(
        "@tool read_file(path!:s,encoding:s=utf8)->(text!:s,mime:s)|Read UTF-8 text from a workspace file\n? path: Workspace-relative file path",
    )
    .unwrap();
    println!("dx-serializer tool syntax:\n{}\n", encode_dx_serializer_tool_catalog(&tools));

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
                id: "toolu_very_long_call_identifier_0001".to_string(),
                tool: "read_file".to_string(),
                args: json!({"path": "src/lib.rs", "encoding": "utf8"}),
            },
            AgentTurn::ToolResult {
                id: "toolu_very_long_call_identifier_0001".to_string(),
                tool: Some("read_file".to_string()),
                status: ToolResultStatus::Ok,
                result: json!({"text": "pub mod llm;"}),
            },
        ],
    };

    let encoded = encode_dx_serializer_conversation(&conversation).unwrap();
    println!("dx-serializer conversation syntax:\n{}\n", encoded);

    let decoded = decode_dx_serializer_conversation(&encoded).unwrap();
    assert_eq!(decoded, conversation);

    let openai = tools.to_openai_tools_json();
    let anthropic = tools.to_anthropic_tools_json();
    let gemini = tools.to_gemini_tools_json();
    let mcp = tools.to_mcp_tools_json();
    let anthropic_export = tools.export_for(ToolProviderTarget::Anthropic);
    let packed = PackedToolCatalog::from_agent_catalog(&tools);
    println!("openai tool wrapper: {}", openai);
    println!("anthropic tool wrapper: {}", anthropic);
    println!("gemini tools wrapper: {}", gemini);
    println!("mcp tool registry: {}", mcp);
    println!("anthropic export_for wrapper: {}", anthropic_export);
    println!("packed catalog:\n{}", encode_dx_serializer_packed_catalog(&packed));
    println!("registry ref:\n{}\n", encode_dx_serializer_registry_ref(&tools));

    let packed_conversation = encode_dx_serializer_packed_conversation_with_registry_ref(
        &conversation,
        &packed,
        &tools.to_dx_serializer_registry_ref(),
    )
    .unwrap();
    println!("packed dx-serializer conversation syntax:\n{}\n", packed_conversation);
    let (_, packed_decoded) =
        decode_dx_serializer_packed_conversation_with_registry_ref(&packed_conversation, &packed)
            .unwrap();
    assert_eq!(packed_decoded, conversation);
}
