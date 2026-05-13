# Production Notes

This serializer crate is now shaped as a Rust library first and a CLI/TUI
second.

That is the intended production posture for:

- Zed-derived editor forks
- Codex-style coding agents
- ZeroClaw-style gateways and local daemons
- any DX shell that wants compact tool catalogs and compact tool-heavy traces

In this repo, that compact tool/transcript layer should be referred to as
`dx-serializer`.

## Stable Integration Surface

Use these public APIs as the primary embed points:

- `encode`, `decode`, `encode_default`, `decode_default`
- `AgentToolCatalog`, `AgentToolSpec`, `SchemaField`, `SchemaType`
- `DxSerializerRegistryRef`
- `ToolProviderTarget`
- `decode_dx_serializer_tool_catalog`, `encode_dx_serializer_tool_catalog`
- `encode_dx_serializer_registry_ref`, `decode_dx_serializer_registry_ref`
- `AgentConversation`, `AgentTurn`, `AgentMessage`, `AgentRole`
- `decode_dx_serializer_conversation`, `encode_dx_serializer_conversation`
- `PackedToolCatalog`, `PackedToolSpec`, `PackedFieldAlias`
- `encode_dx_serializer_packed_catalog`
- `encode_dx_serializer_packed_conversation`, `decode_dx_serializer_packed_conversation`
- `encode_dx_serializer_packed_conversation_with_registry_ref`
- `decode_dx_serializer_packed_conversation_with_registry_ref`

## Recommended Authoring Pattern

1. Store tool registries in `dx-serializer` tool syntax text files.
2. Parse once into `AgentToolCatalog`.
3. Derive a packed registry for repeated tool loops.
4. Derive and cache a stable `@dxs ...` registry ref for out-of-band reuse.
5. Convert to provider payloads only at dispatch time with:
   - `catalog.export_for(ToolProviderTarget::OpenAi)`
   - `catalog.export_for(ToolProviderTarget::Anthropic)`
   - `catalog.export_for(ToolProviderTarget::Gemini)`
   - `catalog.export_for(ToolProviderTarget::Mcp)`
6. Store tool-heavy histories in `dx-serializer` conversation syntax or packed conversation syntax.
7. Expand to verbose remote JSON envelopes only at the network edge.

## Why This Is More Token Efficient

The savings come from specializing for agent-heavy patterns that JSON and
generic TOON alone do not compress well:

- field types shrink to short codes like `s`, `i`, `a<T>`, `o(...)`
- tool wrappers collapse into `@tool ...`
- message roles collapse into `S>`, `D>`, `U>`, `A>`, `R>`
- flat scalar tool calls collapse into `C#id tool(key=value,...)`
- flat scalar tool results collapse into `T#id ok tool(key=value,...)`
- repeated tool names collapse into aliases like `rf`
- repeated field names collapse into aliases like `p`, `e`, `t`
- repeated long or unsafe runtime ids collapse into aliases like `a` via `@i`
- header tags collapse into short forms like `@p`, `@l`, `@i`, and `@dxs`
- turn markers collapse into `>#` for calls and `<#` for results
- flat repeated tool calls collapse further into `>#id rf(v1,v2,...)`
- one-field flat calls can collapse further into `>#id=value`
- flat repeated tool results collapse further into `<#id+(v1,v2,...)`
- one-field flat results can collapse further into `<#id+=value`
- repeated long literals can collapse into a shared table like `@l a=...` and
  then be referenced as `^a`
- repeated shorter literals can also collapse when the runtime heuristic shows a
  net savings after header cost
- shared registries can collapse further into a stable out-of-band handle like
  `@dxs dxs_... t=...`, so the registry text itself does not need to be
  repeated in every session

That reduces repeated wrapper tokens while keeping deterministic conversion back
to provider-facing JSON.

## Operational Notes

- The CLI and TUI remain focused on JSON <-> TOON workflows.
- The `dx-serializer` layer is intended primarily for library embedding.
- The crate keeps actual provider integration at the JSON boundary, which makes
  it easier to update host apps without rewriting stored compact syntax.

## Current Scope

Provider bridge support in this crate now covers:

- OpenAI function-tool JSON
- Anthropic client-tool JSON
- Gemini function declarations
- MCP tool registries

The conversation syntax is intentionally provider-neutral so it can be reused
across all of those hosts without coupling transcript storage to one vendor.
