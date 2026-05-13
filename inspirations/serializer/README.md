# Serializer for Rust

[![Crates.io](https://img.shields.io/crates/v/serializer.svg)](https://crates.io/crates/serializer)
[![Documentation](https://docs.rs/serializer/badge.svg)](https://docs.rs/serializer)
[![Spec v3.0](https://img.shields.io/badge/spec-v3.0-brightgreen.svg)](https://github.com/toon-format/spec/blob/main/SPEC.md)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Tests](https://img.shields.io/badge/tests-%20passing-success.svg)]()

**Serializer** now has two layers:

- a **TOON-compatible core** for compact, lossless JSON-shaped data
- a **dx-serializer layer** for compact tool schemas and compact chat/tool transcripts

The TOON core remains the right default for generic nested data. The `dx-serializer` layer is aimed at the 2026 agent stack problem: tool schemas, tool calls, tool results, and chat histories consume a lot of tokens when represented as raw JSON.

This crate provides the official, **spec-compliant Rust implementation** of TOON v3.0 while also adding a library-first `dx-serializer` tool syntax and conversation syntax for modern LLM hosts.

## Quick Example

**JSON** (16 tokens, 40 bytes):
```json
{
  "users": [
    { "id": 1, "name": "Alice" },
    { "id": 2, "name": "Bob" }
  ]
}
```

**TOON** (13 tokens, 28 bytes) - **18.75% token savings**:
```toon
users[2]{id,name}:
  1,Alice
  2,Bob
```

## Features

- **Generic API**: Works with any `Serialize`/`Deserialize` type - custom structs, enums, JSON values, and more
- **Spec-Compliant**: Fully compliant with [TOON Specification v3.0](https://github.com/toon-format/spec/blob/main/SPEC.md)
- **Key Folding & Path Expansion**: Collapse and expand dotted key paths
- **dx-serializer Syntax**: Compact tool-schema syntax and conversation syntax for agent systems
- **Packed Tool Calling**: Alias-heavy positional tool syntax for maximum savings in repeated local tool loops
- **Provider Bridges**: Convert compact tool specs to OpenAI, Anthropic, Gemini, and MCP tool JSON
- **Safe & Performant**: Built with safe, fast Rust
- **Powerful CLI**: Full-featured command-line tool
- **Strict Validation**: Enforces all spec rules (configurable)
- **Well-Tested**: Comprehensive test suite with unit tests, spec fixtures, and real-world scenarios

## Why The dx-serializer Layer Exists

TOON is a strong generic structured-data format, but current agent systems still burn tokens on:

- verbose JSON Schema wrappers for tools
- repeated `role/content` message arrays
- repeated tool-call and tool-result envelopes

Recent primary-source work points the same way:

- TOON's own benchmark work shows wins over JSON for general structured data
- OpenAI's tool/function-calling surfaces still consume JSON Schema
- Anthropic's tool-use guidance explicitly treats tool overhead as important
- Simon Willison's `llm` project now ships a concise schema syntax because raw JSON Schema is too verbose for day-to-day model work

Serializer's `dx-serializer` layer specializes for those agent surfaces instead of forcing everything through generic JSON objects.

## Installation

### As a Library

```bash
cargo add serializer
```

### As a CLI Tool

```bash
cargo install serializer
```

---

## Library Usage

### Basic Encode & Decode

The `encode` and `decode` functions work with any type implementing `Serialize`/`Deserialize`:

**With custom structs:**

```rust
use serde::{Serialize, Deserialize};
use serializer::{encode_default, decode_default};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct User {
    name: String,
    age: u32,
    email: String,
}

fn main() -> Result<(), serializer::ToonError> {
    let user = User {
        name: "Alice".to_string(),
        age: 30,
        email: "alice@example.com".to_string(),
    };

    // Encode to TOON
    let toon = encode_default(&user)?;
    println!("{}", toon);
    // Output:
    // name: Alice
    // age: 30
    // email: alice@example.com

    // Decode back to struct
    let decoded: User = decode_default(&toon)?;
    assert_eq!(user, decoded);

    Ok(())
}
```

**With JSON values:**

```rust
use serde_json::{json, Value};
use serializer::{encode_default, decode_default};

fn main() -> Result<(), serializer::ToonError> {
    let data = json!({
        "users": [
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]
    });

    // Encode to TOON
    let toon_str = encode_default(&data)?;
    println!("{}", toon_str);
    // Output:
    // users[2]{id,name}:
    //   1,Alice
    //   2,Bob

    // Decode back to JSON
    let decoded: Value = decode_default(&toon_str)?;
    assert_eq!(decoded, data);
    
    Ok(())
}
```

## dx-serializer Tool Syntax

```text
@tool read_file(path!:s,encoding:s=utf8,from?:i,to?:i)->(text!:s,mime:s)|Read UTF-8 text from a workspace file
? path: Workspace-relative file path
? encoding: Text encoding; defaults to utf8
? return.text: File contents
```

Supported short types:

- `s` string
- `i` integer
- `n` number
- `b` boolean
- `z` null
- `*` any
- `a<T>` array
- `o(...)` object
- `e[a|b|c]` enum

Rust usage:

```rust
use serializer::{decode_dx_serializer_tool_catalog, encode_dx_serializer_tool_catalog};

let dsl = "@tool read_file(path!:s)->(text!:s)|Read a file\n? path: Workspace-relative path";
let catalog = decode_dx_serializer_tool_catalog(dsl)?;
let round_trip = encode_dx_serializer_tool_catalog(&catalog);
assert!(round_trip.contains("@tool read_file("));

let openai_tools = catalog.to_openai_tools_json();
let anthropic_tools = catalog.to_anthropic_tools_json();
let gemini_tools = catalog.to_gemini_tools_json();
let mcp_tools = catalog.to_mcp_tools_json();
let generic_export = catalog.export_for(serializer::ToolProviderTarget::OpenAi);
# Ok::<(), serializer::ToonError>(())
```

## dx-serializer Conversation Syntax

```text
S> You are DX.
U> Summarize src/lib.rs

C#c1 read_file(path=src/lib.rs)
T#c1 ok read_file(text="pub mod llm;")

A> The file exports the llm module.
```

Rust usage:

```rust
use serde_json::json;
use serializer::{
    AgentConversation, AgentMessage, AgentRole, AgentTurn, ToolResultStatus,
    decode_dx_serializer_conversation, encode_dx_serializer_conversation,
};

let conversation = AgentConversation {
    turns: vec![
        AgentTurn::Message(AgentMessage {
            role: AgentRole::System,
            content: "You are DX.".to_string(),
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
    ],
};

let encoded = encode_dx_serializer_conversation(&conversation)?;
let decoded = decode_dx_serializer_conversation(&encoded)?;
assert_eq!(decoded, conversation);
# Ok::<(), serializer::ToonError>(())
```

See [docs/DX_SERIALIZER.md](docs/DX_SERIALIZER.md) for the full syntax guide, [docs/INTEGRATIONS.md](docs/INTEGRATIONS.md) for Zed/Codex/ZeroClaw embed patterns, and [docs/PRODUCTION_READY.md](docs/PRODUCTION_READY.md) for the recommended production integration surface.

## Packed Tool Calling

For repeated tool-heavy sessions, Serializer can derive an even tighter packed
registry:

```text
@p rf=read_file(p!:path,e?:encoding=utf8)->(t!:text,m?:mime)
```

And then compress the conversation further:

```text
U> Summarize src/lib.rs
>#c1=src/lib.rs
<#c1+("pub mod llm;",text/plain)
```

If a payload is nested or carries extra host metadata, the packed layer falls
back to a TOON block while preserving those extra keys.

If a long string repeats across calls, the packed layer can also emit a literal
table:

```text
@l a=src/features/editor/very_long_file_name.rs
>#c1=^a
>#c2=^a
```

Packed scalar shortcuts:

- `+` for `true`
- `-` for `false`
- `~` for `null`
- `_` for an omitted/default positional slot
- `^a` for a shared literal-table reference

Single-field packed forms:

- `>#c1=src/lib.rs`
- `<#c1+="pub mod llm;"`

If call ids are long or unsafe for compact inline syntax, the packed layer can
also alias them:

```text
@i a=toolu_very_long_call_identifier_0001
>#a=src/lib.rs
<#a+="pub mod llm;"
```

Literal references are selected by a simple cost model, not only by a fixed
length threshold, so frequently repeated short strings can also be packed when
that is still a net win.

For the absolute minimum repeat overhead, derive a stable shared registry ref
once and reuse it across sessions:

```text
@dxs dxs_0123456789abcdef t=1
```

That lets your host keep the semantic tool registry out-of-band while the model
only sees the shortest packed calls and results.

Rust usage:

```rust
use serializer::{
    PackedToolCatalog,
    decode_dx_serializer_tool_catalog,
    encode_dx_serializer_packed_catalog,
    encode_dx_serializer_registry_ref,
};

let tools = decode_dx_serializer_tool_catalog(
    "@tool read_file(path!:s,encoding:s=utf8)->(text!:s,mime:s)|Read a file",
)?;
let packed = PackedToolCatalog::from_agent_catalog(&tools);
let packed_registry = encode_dx_serializer_packed_catalog(&packed);
let registry_ref = encode_dx_serializer_registry_ref(&tools);
assert!(packed_registry.contains("@p rf=read_file"));
assert!(registry_ref.starts_with("@dxs dxs_"));
# Ok::<(), serializer::ToonError>(())
```
---

## API Reference

### Encoding

#### `encode<T: Serialize>(&value, &options) -> Result<String, ToonError>`

Encode any serializable type to TOON format. Works with custom structs, enums, collections, and `serde_json::Value`.

```rust
use serializer::{encode, EncodeOptions, Delimiter, Indent};
use serde_json::json;

let data = json!({"items": ["a", "b", "c"]});

// Default encoding
let toon = encode(&data, &EncodeOptions::default())?;
// items[3]: a,b,c

// Custom delimiter
let opts = EncodeOptions::new()
    .with_delimiter(Delimiter::Pipe);
let toon = encode(&data, &opts)?;
// items[3|]: a|b|c

// Custom indentation
let opts = EncodeOptions::new()
    .with_indent(Indent::Spaces(4));
let toon = encode(&data, &opts)?;
```

#### `EncodeOptions`

| Method | Description | Default |
|--------|-------------|---------|
| `with_delimiter(d)` | Set delimiter: `Comma`, `Tab`, or `Pipe` | `Comma` |
| `with_indent(i)` | Set indentation (spaces only) | `Spaces(2)` |
| `with_spaces(n)` | Shorthand for `Indent::Spaces(n)` | `2` |
| `with_key_folding(mode)` | Enable key folding (v1.5) | `Off` |
| `with_flatten_depth(n)` | Set max folding depth | `usize::MAX` |

### Decoding

#### `decode<T: Deserialize>(&input, &options) -> Result<T, ToonError>`

Decode TOON format into any deserializable type. Works with custom structs, enums, collections, and `serde_json::Value`.

**With custom structs:**
```rust
use serde::Deserialize;
use serializer::{decode, DecodeOptions};

#[derive(Deserialize)]
struct Config {
    host: String,
    port: u16,
}

let toon = "host: localhost\nport: 8080";
let config: Config = decode(toon, &DecodeOptions::default())?;
```

**With JSON values:**
```rust
use serde_json::Value;
use serializer::{decode, DecodeOptions};

let toon = "name: Alice\nage: 30";

// Default (strict) decode
let json: Value = decode(toon, &DecodeOptions::default())?;

// Non-strict mode (relaxed validation)
let opts = DecodeOptions::new().with_strict(false);
let json: Value = decode(toon, &opts)?;

// Disable type coercion
let opts = DecodeOptions::new().with_coerce_types(false);
let json: Value = decode("active: true", &opts)?;
// With coercion: {"active": true}
// Without: {"active": "true"}
```

**Helper functions:**
- `encode_default<T>(&value)` - Encode with default options
- `decode_default<T>(&input)` - Decode with default options

#### `DecodeOptions`

| Method | Description | Default |
|--------|-------------|---------|
| `with_strict(b)` | Enable strict validation | `true` |
| `with_coerce_types(b)` | Auto-convert strings to types | `true` |
| `with_expand_paths(mode)` | Enable path expansion (v1.5) | `Off` |

---

## v1.5 Features

### Key Folding (Encoder)

**New in v1.5**: Collapse single-key object chains into dotted paths to reduce tokens.

**Standard nesting:**
```toon
data:
  metadata:
    items[2]: a,b
```

**With key folding:**
```toon
data.metadata.items[2]: a,b
```

**Example:**

```rust
use serde_json::json;
use serializer::{encode, EncodeOptions, KeyFoldingMode};

let data = json!({
    "data": {
        "metadata": {
            "items": ["a", "b"]
        }
    }
});

// Enable key folding
let opts = EncodeOptions::new()
    .with_key_folding(KeyFoldingMode::Safe);

let toon = encode(&data, &opts)?;
// Output: data.metadata.items[2]: a,b
```

#### With Depth Control

```rust
let data = json!({"a": {"b": {"c": {"d": 1}}}});

// Fold only 2 levels
let opts = EncodeOptions::new()
    .with_key_folding(KeyFoldingMode::Safe)
    .with_flatten_depth(2);

let toon = encode(&data, &opts)?;
// Output:
// a.b:
//   c:
//     d: 1
```

#### Safety Features

Key folding only applies when:
- All segments are valid identifiers (`a-z`, `A-Z`, `0-9`, `_`)
- Each level contains exactly one key
- No collision with sibling literal keys
- Within the specified `flatten_depth`

Keys like `full-name`, `user.email` (if quoted), or numeric keys won't be folded.

### Path Expansion (Decoder)

**New in v1.5**: Automatically expand dotted keys into nested objects.

**Compact input:**
```toon
a.b.c: 1
a.b.d: 2
a.e: 3
```

**Expanded output:**
```json
{
  "a": {
    "b": {
      "c": 1,
      "d": 2
    },
    "e": 3
  }
}
```

**Example:**

```rust
use serde_json::Value;
use serializer::{decode, DecodeOptions, PathExpansionMode};

let toon = "a.b.c: 1\na.b.d: 2";

// Enable path expansion
let opts = DecodeOptions::new()
    .with_expand_paths(PathExpansionMode::Safe);

let json: Value = decode(toon, &opts)?;
// {"a": {"b": {"c": 1, "d": 2}}}
```

**Round-Trip Example:**

```rust
use serde_json::{json, Value};
use serializer::{encode, decode, EncodeOptions, DecodeOptions, KeyFoldingMode, PathExpansionMode};

let original = json!({
    "user": {
        "profile": {
            "name": "Alice"
        }
    }
});

// Encode with folding
let encode_opts = EncodeOptions::new()
    .with_key_folding(KeyFoldingMode::Safe);
let toon = encode(&original, &encode_opts)?;
// Output: "user.profile.name: Alice"

// Decode with expansion
let decode_opts = DecodeOptions::new()
    .with_expand_paths(PathExpansionMode::Safe);
let restored: Value = decode(&toon, &decode_opts)?;

assert_eq!(restored, original); // Perfect round-trip!
```

**Quoted Keys Remain Literal:**

```rust
use serde_json::Value;
use serializer::{decode, DecodeOptions, PathExpansionMode};

let toon = r#"a.b: 1
"c.d": 2"#;

let opts = DecodeOptions::new()
    .with_expand_paths(PathExpansionMode::Safe);
let json: Value = decode(toon, &opts)?;
// {
//   "a": {"b": 1},
//   "c.d": 2        <- quoted key preserved
// }
```

---

## Interactive TUI

Serializer includes a full-featured Terminal User Interface for interactive conversions!

```bash
# Launch interactive mode
serializer --interactive
# or
serializer -i
```

### Features:
- Real-time conversion as you type
- Live statistics (tokens, bytes, savings)
- Interactive settings - adjust all options on-the-fly
- File browser with visual navigation
- Side-by-side diff viewer
- Conversion history tracking
- File operations (open, save, new)
- Clipboard integration (copy/paste)
- REPL mode for command-line interaction
- Round-trip testing
- Theme support (Dark/Light)
- Built-in help with keyboard shortcuts

**Perfect for:**
- Learning TOON format interactively
- Testing conversions in real-time
- Experimenting with different settings
- Visual before/after comparisons
- Quick data transformations

See [docs/TUI.md](docs/TUI.md) for complete documentation and keyboard shortcuts!

The current TUI remains focused on JSON <-> TOON conversion. The `dx-serializer` tool and conversation syntaxes are library-first surfaces intended for integration into host apps and editor forks.

---

## CLI Usage

### Basic Commands

```bash
# Auto-detect from extension
serializer data.json        # Encode
serializer data.toon        # Decode

# Force mode
serializer -e data.txt      # Force encode
serializer -d output.txt    # Force decode

# Pipe from stdin
cat data.json | serializer
echo '{"name": "Alice"}' | serializer -e
```

### Encode Options

```bash
# Custom delimiter
serializer data.json --delimiter pipe
serializer data.json --delimiter tab

# Custom indentation
serializer data.json --indent 4

# Key folding (v1.5)
serializer data.json --fold-keys
serializer data.json --fold-keys --flatten-depth 2

# Show statistics
serializer data.json --stats
```

### Decode Options

```bash
# Pretty-print JSON
serializer data.toon --json-indent 2

# Relaxed validation
serializer data.toon --no-strict

# Disable type coercion
serializer data.toon --no-coerce

# Path expansion (v1.5)
serializer data.toon --expand-paths
```

### Full Example

```bash
$ echo '{"data":{"meta":{"items":["x","y"]}}}' | serializer --fold-keys --stats

data.meta.items[2]: x,y

Stats:
+--------------+------+------+---------+
| Metric       | JSON | TOON | Savings |
+======================================+
| Tokens       | 13   | 8    | 38.46%  |
|--------------+------+------+---------|
| Size (bytes) | 38   | 23   | 39.47%  |
+--------------+------+------+---------+
```

---

## Testing

The library includes a comprehensive test suite covering core functionality, edge cases, spec compliance, and real-world scenarios.

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --test spec_fixtures
cargo test --lib

# With output
cargo test -- --nocapture
```

## Error Handling

All operations return `Result<T, ToonError>` with descriptive error messages:

```rust
use serde_json::Value;
use serializer::{decode_strict, ToonError};

match decode_strict::<Value>("items[3]: a,b") {
    Ok(value) => println!("Success: {:?}", value),
    Err(ToonError::LengthMismatch { expected, found, .. }) => {
        eprintln!("Array length mismatch: expected {}, found {}", expected, found);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Error Types

- `ParseError` - Syntax errors with line/column info
- `LengthMismatch` - Array length doesn't match header
- `TypeMismatch` - Unexpected value type
- `InvalidStructure` - Malformed TOON structure
- `SerializationError` / `DeserializationError` - Conversion failures

---


## Examples
Run with `cargo run --example examples` to see all examples:
- `agent_syntax.rs` - Compact dx-serializer tool and conversation syntax
- `structs.rs` - Custom struct serialization
- `tabular.rs` - Tabular array formatting
- `arrays.rs` - Various array formats
- `arrays_of_arrays.rs` - Nested arrays
- `objects.rs` - Object encoding
- `mixed_arrays.rs` - Mixed-type arrays
- `delimiters.rs` - Custom delimiters
- `round_trip.rs` - Encode/decode round-trips
- `decode_strict.rs` - Strict validation
- `empty_and_root.rs` - Edge cases

---

## Resources

- [TOON Specification v3.0](https://github.com/toon-format/spec/blob/main/SPEC.md)
- [dx-serializer Syntax Guide](docs/DX_SERIALIZER.md)
- [Integration Guide](docs/INTEGRATIONS.md)
- [Production Notes](docs/PRODUCTION_READY.md)
- [Crates.io Package](https://crates.io/crates/serializer)
- [API Documentation](https://docs.rs/serializer)
- [Main Repository (JS/TS)](https://github.com/toon-format/toon)
- [Benchmarks & Performance](https://github.com/toon-format/toon#benchmarks)

Primary-source references that informed the `dx-serializer` layer:

- [Anthropic tool use overview](https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/overview)
- [Anthropic token-efficient tool use](https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/token-efficient-tool-use)
- [OpenAI function calling guide](https://platform.openai.com/docs/guides/function-calling)
- [OpenAI structured outputs guide](https://platform.openai.com/docs/guides/structured-outputs)
- [Gemini function calling guide](https://ai.google.dev/gemini-api/docs/function-calling)
- [MCP tools concept](https://modelcontextprotocol.io/legacy/concepts/tools)
- [Simon Willison concise schema syntax](https://llm.datasette.io/en/stable/schemas.html)

---

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development

```bash
# Clone the repository
git clone https://github.com/essence/serializer-rust.git
cd serializer-rust

# Run tests
cargo test --all

# Run lints
cargo clippy -- -D warnings

# Format code
cargo fmt

# Build docs
cargo doc --open
```

---

## License

MIT License Copyright 2025-PRESENT [Johann Schopplich](https://github.com/johannschopplich) and [Shreyas K S](https://github.com/shreyasbhat0)
