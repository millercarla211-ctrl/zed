# Development Guide

## Setup

1. Install Rust (edition 2024):
```bash
rustup update
rustup default stable
```

2. Download models:
```bash
pwsh scripts/download_moonshine_onnx.ps1
```

3. Build project:
```bash
cargo build --release
```

## Project Structure

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed structure.

## Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_test

# Benchmarks
cargo bench
```

## Examples

```bash
# Basic transcription
cargo run --example transcribe

# Full Wispr Flow pipeline
cargo run --example wispr_flow
```

## CLI Usage

```bash
# Transcribe audio
cargo run -- --transcribe audio.mp3

# Full pipeline (STT + LLM)
cargo run -- --wispr audio.mp3

# Text-to-speech
cargo run -- --speak "Hello world"
```

## Code Style

- Use `rustfmt` for formatting
- Use `clippy` for linting
- Follow Rust 2024 edition idioms
- Document public APIs

## Performance Profiling

```bash
# Profile with flamegraph
cargo flamegraph --bin flow -- --transcribe audio.mp3

# Memory profiling
cargo run --release -- --transcribe audio.mp3
```

## Contributing

1. Create feature branch
2. Write tests
3. Run `cargo fmt` and `cargo clippy`
4. Submit pull request
