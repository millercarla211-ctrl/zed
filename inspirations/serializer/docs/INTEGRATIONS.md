# Integration Guide

This crate is designed to slot into Rust-first AI hosts where raw JSON is too
verbose for repeated tool schemas and repeated chat/tool transcripts.

Recommended integration targets:

- Zed-like editor assistants
- Codex-like coding agents
- ZeroClaw-like gateways, daemons, and channel runners
- local-first DX shells that cache prompts, tool registries, and transcripts

## What To Use

Use the TOON-compatible core when you need:

- generic nested data
- config snapshots
- cached model metadata
- compact object and array storage

Use the `dx-serializer` layer when you need:

- compact tool catalogs
- compact chat histories
- compact tool-call and tool-result traces
- provider bridge conversion to request-time JSON

Use the packed layer when you need:

- the same tools called repeatedly in long sessions
- the same argument names repeated many times
- long provider-generated call ids that should not leak into stored transcripts
- the absolute shortest stable local archive format for tool-heavy traces

## Tool Catalog Flow

Define tools once in `dx-serializer` tool syntax:

```text
@tool read_file(path!:s,encoding:s=utf8)->(text!:s,mime:s)|Read UTF-8 text from a workspace file
? path: Workspace-relative file path
```

Then load and bridge them at runtime:

```rust
use serializer::decode_dx_serializer_tool_catalog;

let catalog = decode_dx_serializer_tool_catalog(
    "@tool read_file(path!:s)->(text!:s)|Read a file\n? path: Workspace-relative path",
)?;

let openai_tools = catalog.to_openai_tools_json();
let anthropic_tools = catalog.to_anthropic_tools_json();
let gemini_tools = catalog.to_gemini_tools_json();
let mcp_tools = catalog.to_mcp_tools_json();
# Ok::<(), serializer::ToonError>(())
```

## Transcript Flow

Encode repeated tool-heavy histories in the `dx-serializer` conversation syntax:

```text
D> Prefer local tools first.
U> Summarize src/lib.rs
C#c1 read_file(path=src/lib.rs)
T#c1 ok read_file(text="pub mod llm;")
A> The file exports the llm module.
```

This saves tokens compared with repeated JSON wrappers like:

- `{"role":"user","content":"..."}`
- `{"type":"tool_call","id":"...","tool":"...","args":{...}}`
- `{"type":"tool_result","id":"...","tool":"...","status":"ok","result":{...}}`

For even tighter loops, derive the packed registry and store repeated traces in
the packed conversation syntax:

```text
@p rf=read_file(p!:path,e?:encoding=utf8)->(t!:text,m?:mime)
@i a=toolu_very_long_call_identifier_0001
U> Summarize src/lib.rs
>#a=src/lib.rs
<#a+("pub mod llm;",text/plain)
```

## Zed-Like Hosts

Recommended usage:

- keep the active tool registry in `dx-serializer` tool syntax form
- bridge to provider JSON only at request time
- archive assistant sessions in `dx-serializer` conversation syntax
- cache transcript snapshots in `rkyv` or similar outer layers

## Codex-Like Hosts

Recommended usage:

- store task tool registries in `dx-serializer` tool syntax
- store follow-up tool traces in `dx-serializer` conversation syntax
- preserve reasoning/task transcripts in compact role markers
- materialize OpenAI or Gemini tool JSON only when dispatching a request

## ZeroClaw-Like Hosts

Recommended usage:

- keep gateway- or channel-specific tool packs as `dx-serializer` tool catalogs
- persist daemon or channel traces in `dx-serializer` conversation syntax
- bridge to MCP registries when exposing tools to other agents
- keep provider-facing JSON as an edge representation, not the source format

## Suggested Storage Pattern

For best results in the parent DX stack:

1. Author and version tools in `dx-serializer` text form.
2. Parse once into `AgentToolCatalog`.
3. Derive `PackedToolCatalog` for repeated local tool loops.
4. Derive a stable `@dxs dxs_... t=...` registry ref for long-lived shared tool packs.
5. Export provider-specific JSON views on demand.
6. Store tool-heavy histories in `AgentConversation` or packed conversation syntax.
7. Only expand to verbose provider envelopes when a remote API requires it.

That keeps the authoring format compact while still preserving compatibility
with mainstream tool-calling APIs.
