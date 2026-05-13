# dx-serializer Syntax

Serializer now ships two specialized compact syntaxes on top of the TOON-compatible core:

- tool syntax for compact tool definitions that can be compiled to provider JSON
- conversation syntax for compact chat, tool-call, and tool-result transcripts
- packed tool syntax for alias-heavy, repeated-call workloads where even the
  regular `dx-serializer` layer still repeats too many field names

These are intended for agent systems such as Zed forks, Codex-like hosts, ZeroClaw-like gateways, and local-first DX shells.

## Why this layer exists

The generic TOON core is strong for nested data and tabular arrays, but modern agent systems also spend many tokens on:

- verbose JSON Schema wrappers for tools
- repeated `role/content` chat objects
- repeated `tool_call` / `tool_result` envelopes

The `dx-serializer` layer removes that boilerplate while keeping deterministic conversion back to provider JSON shapes.

## Packed Tool Layer

When a host calls the same tools repeatedly, the main remaining overhead is:

- repeating full tool names
- repeating full field names
- repeating `ok` / `error` result markers

Serializer now adds a more aggressive packed layer that derives:

- short tool aliases like `rf` for `read_file`
- short field aliases like `p` for `path`
- short call-id aliases like `a` for long runtime ids
- short header tags like `@p`, `@l`, `@i`, and `@dxs`
- positional inline payloads for flat scalar objects
- single-field scalar payloads that collapse to `tool=value`
- `+` / `!` result markers for success and failure

Example packed registry:

```text
@p rf=read_file(p!:path,e?:encoding=utf8)->(t!:text,m?:mime)
```

Example packed conversation:

```text
U> Summarize src/lib.rs
>#c1=src/lib.rs
<#c1+("pub mod llm;",text/plain)
```

For the lowest repeated-session overhead, derive a stable shared registry ref
from the tool catalog and carry only that handle:

```text
@dxs dxs_0123456789abcdef t=1
```

That lets the host keep the semantic tool registry out-of-band while the prompt
surface only carries packed calls, packed results, `@lit`, and `@cid`.

This is intended for the highest-volume tool-calling loops where the host
already has a shared registry and can safely map aliases back to full tool
definitions.

Packed scalar shortcuts:

- `+` boolean `true`
- `-` boolean `false`
- `~` `null`
- `_` omitted or defaulted positional slot
- `^a` shared literal-table reference

Single-field packed forms:

- `>#c1=src/lib.rs`
- `<#c1+="pub mod llm;"`
- `<#a!=permission-denied`

Packed call-id headers:

- `@i a=toolu_very_long_call_identifier_0001`
- `C#a ...`
- `T#a+...`

When a payload is nested or contains extra non-registry keys, the packed layer
falls back to an indented TOON block and preserves the unknown keys instead of
dropping them.

When a long scalar string repeats across packed calls, Serializer can also emit
an inline literal table:

```text
@l a=src/features/editor/very_long_file_name.rs
>#c1=^a
>#c2=^a
```

That avoids repeating the same long path or URL over and over.

Literal-table entries are selected by a simple savings heuristic, so repeated
short strings can also be packed when they repeat often enough to offset the
header cost.

## Tool Syntax

Example:

```text
@tool read_file(path!:s,encoding:s=utf8,from?:i,to?:i)->(text!:s,mime:s)|Read UTF-8 text from a workspace file
? path: Workspace-relative file path
? encoding: Text encoding; defaults to utf8
? return.text: File contents
```

### Field markers

- `!` required field
- `?` optional field
- `=` default value

### Type codes

- `s` string
- `i` integer
- `n` number
- `b` boolean
- `z` null
- `*` any
- `a<T>` array
- `o(...)` object
- `e[a|b|c]` enum

### Provider conversion

The DSL can be compiled into provider-specific tool definitions:

- OpenAI function tools with strict JSON Schema
- Anthropic client tools with `input_schema`
- Gemini function declarations with OpenAPI-style `parameters`
- MCP tool descriptors with `inputSchema` and optional `outputSchema`

## Conversation Syntax

Example:

```text
S> You are DX.
U> Summarize src/lib.rs
C#c1 read_file(path=src/lib.rs)
T#c1 ok read_file(text="pub mod llm;")

A> The file exports the llm module.
```

### Message markers

- `S>` system
- `D>` developer
- `U>` user
- `A>` assistant
- `R>` reasoning

### Multiline messages

```text
D>>>
Line 1
Line 2
<<<
```

### Tool markers

- `C#<id> <tool>` tool call
- `C#<id> <tool>(key=value,...)` inline scalar tool call
- `T#<id> ok <tool>` successful tool result
- `T#<id> ok <tool>(key=value,...)` inline scalar tool result
- `T#<id> error <tool>` failed tool result

Structured tool bodies are stored either:

- inline when the payload is a flat scalar object
- as indented TOON blocks when the payload is nested or multiline

Example block form:

```text
C#c2 search_docs:
  query: serializer llm syntax
  limit: 5
```

## Integration targets

Recommended uses:

- Zed-style local assistant transcripts
- Codex-style task and follow-up histories
- ZeroClaw-style gateway, daemon, and channel tool histories
- prompt caching and memmap/rkyv archives in the parent DX stack

Recommended split:

- use the normal `dx-serializer` tool syntax for human-authored registries
- derive the packed layer for runtime-heavy tool loops and archived transcripts
- derive a stable `@dxs dxs_... t=...` registry ref for the absolute
  minimum repeated schema overhead
