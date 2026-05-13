# Flow - Complete Implementation Guide

> **Project**: Flow - Open-Source Voice Assistant  
> **Status**: Foundation Complete, STT/TTS Integration Needed  
> **Date**: March 31, 2026  
> **Version**: 0.1.0

---

## 🎯 Mission: Build the Ultimate Open-Source Voice Assistant

Create a production-ready Wispr Flow alternative using the latest March 2026 models:
- **STT**: Moonshine v2 (beats Whisper Large v3 with 6x fewer params)
- **LLM**: Qwen 3.5 Small Series (beats models 3x its size)
- **TTS**: Kokoro v1.0 (#1 on TTS Arena) + Voxtral TTS (beats ElevenLabs)

**Current State**: Professional structure implemented, LLM integrated (Qwen 3.5 0.8B), Moonshine ONNX models downloaded. Need: Real ONNX inference implementation.

---

## 📋 Implementation Checklist

### Phase 1: Moonshine STT Integration (PRIORITY)
- [x] Add `ort` crate to Cargo.toml with proper threading config
- [x] Create professional project structure
- [ ] Implement mel spectrogram feature extraction (80 mel bins, 16kHz) in `src/audio/features.rs`
- [ ] Load encoder/decoder ONNX models in `src/models/stt.rs`
- [ ] Implement encoder inference (audio features → hidden states)
- [ ] Implement decoder inference (hidden states → token IDs)
- [ ] Parse tokenizer.json and implement token-to-text decoding
- [ ] Test with `tests/fixtures/audio.mp3`: "hello mike testing one two three hello"
- [ ] Verify WER < 10% on test audio

### Phase 2: Text Enhancement (Wispr Flow Style)
- [ ] Remove filler words: "um", "uh", "like", "you know", "sort of", "kind of"
- [ ] Add proper punctuation using simple rules or LLM
- [ ] Capitalize sentences correctly
- [ ] Format output cleanly (trim whitespace, fix spacing)
- [ ] Test `--wispr` command (STT + LLM enhancement pipeline)

### Phase 3: TTS Integration (Optional but Recommended)
- [ ] Implement Kokoro v1.0 INT8 inference in `src/models/tts.rs`
- [ ] Add `--speak` command to read back enhanced text
- [ ] Test voice output quality
- [ ] Add Voxtral TTS as premium option (if 16GB+ RAM available)

### Phase 4: Production Features
- [ ] Add real-time microphone input support
- [ ] Implement streaming transcription (Moonshine supports this!)
- [ ] Add voice activity detection (VAD)
- [ ] Support multiple audio formats (MP3, WAV, FLAC, OGG)
- [ ] Add hotkey activation for system-wide use
- [ ] Implement clipboard injection for transcribed text

---

## 🏗️ Professional Project Structure

```
flow/                                    # Project root (rebranded from wispr-flow)
├── .gitignore                          # Git ignore rules
├── Cargo.toml                          # Rust dependencies (edition 2024)
├── Cargo.lock                          # Dependency lock file
├── AGENTS.md                           # This file (implementation guide)
├── README.md                           # User documentation (DO NOT MODIFY)
│
├── src/                                # Source code (modular architecture)
│   ├── lib.rs                         # Library entry point
│   ├── main.rs                        # Binary entry point
│   │
│   ├── audio/                         # Audio processing module
│   │   ├── mod.rs                     # Module exports
│   │   ├── features.rs                # Mel spectrogram computation
│   │   └── loader.rs                  # Audio file loading (WAV, MP3, etc.)
│   │
│   ├── cli/                           # Command-line interface
│   │   ├── mod.rs                     # Module exports
│   │   ├── args.rs                    # Argument parsing
│   │   └── commands.rs                # Command execution logic
│   │
│   ├── models/                        # ML model inference
│   │   ├── mod.rs                     # Module exports
│   │   ├── llm.rs                     # Qwen 3.5 LLM (✓ Working)
│   │   ├── stt.rs                     # Moonshine STT (needs ONNX impl)
│   │   └── tts.rs                     # Kokoro TTS (stub)
│   │
│   ├── pipeline/                      # Processing pipelines
│   │   ├── mod.rs                     # Module exports
│   │   └── voice.rs                   # Voice processing pipeline
│   │
│   └── utils/                         # Utility functions
│       ├── mod.rs                     # Module exports
│       └── system.rs                  # System info, memory checks
│
├── tests/                              # Integration tests
│   ├── integration_test.rs            # Main integration tests
│   └── fixtures/                      # Test data
│       ├── README.md                  # Test fixtures documentation
│       └── audio.mp3                  # Test audio (3.14s, "hello mike...")
│
├── benches/                            # Performance benchmarks
│   └── benchmark.rs                   # Criterion benchmarks
│
├── examples/                           # Usage examples
│   ├── transcribe.rs                  # Basic STT example
│   └── wispr_flow.rs                  # Full pipeline example
│
├── docs/                               # Documentation
│   ├── ARCHITECTURE.md                # System architecture
│   ├── DEVELOPMENT.md                 # Development guide
│   ├── MODELS.md                      # Model documentation
│   └── API.md                         # API documentation
│
├── models/                             # Model files (large, gitignored)
│   ├── llm/                           # Qwen GGUF models
│   │   ├── .gitkeep
│   │   ├── Qwen3.5-0.8B-Q4_K_M.gguf  # ✓ Working (1.5GB RAM)
│   │   ├── Qwen3.5-2B-Q4_K_M.gguf    # Available (2.5GB RAM)
│   │   └── Qwen3.5-4B-Q4_K_M.gguf    # Recommended (4.5GB RAM)
│   │
│   ├── stt/                           # Moonshine ONNX models
│   │   ├── .gitkeep
│   │   ├── moonshine-tiny-encoder.onnx      # ✓ Downloaded (~13MB)
│   │   ├── moonshine-tiny-decoder.onnx      # ✓ Downloaded (~14MB)
│   │   ├── moonshine-tiny-config.json       # Model configuration
│   │   └── moonshine-tiny-tokenizer.json    # BPE tokenizer
│   │
│   └── tts/                           # Kokoro TTS models
│       ├── .gitkeep
│       ├── kokoro-v1.0.int8.onnx     # ✓ Downloaded (~80MB)
│       └── voices-v1.0.bin           # Voice data
│
├── scripts/                            # Utility scripts
│   ├── download_moonshine_onnx.ps1   # Download Moonshine models
│   └── download_whisper.ps1          # Legacy Whisper download
│
└── trash/                              # Old/experimental code (ignore)
    └── [various old files]
```

---

## 🔧 Module Architecture

### Core Modules

#### `src/audio/` - Audio Processing
- **features.rs**: Mel spectrogram computation using rustfft
- **loader.rs**: Audio file loading with hound/rodio
- **Purpose**: Convert audio files to features for STT

#### `src/cli/` - Command-Line Interface
- **args.rs**: Argument parsing (--transcribe, --wispr, --speak)
- **commands.rs**: Command execution logic
- **Purpose**: User-facing CLI interface

#### `src/models/` - ML Model Inference
- **llm.rs**: Qwen 3.5 LLM inference (✓ Working)
- **stt.rs**: Moonshine STT inference (needs implementation)
- **tts.rs**: Kokoro TTS inference (stub)
- **Purpose**: Core ML inference engines

#### `src/pipeline/` - Processing Pipelines
- **voice.rs**: Multi-stage voice processing
- **Purpose**: Coordinate STT → LLM → TTS flow

#### `src/utils/` - Utilities
- **system.rs**: System info, memory checks
- **Purpose**: Helper functions

---

## 🛠️ Implementation Details

### Step 1: Implement Mel Spectrogram in `src/audio/features.rs`

```rust
use rustfft::{FftPlanner, num_complex::Complex};
use ndarray::Array2;

pub fn compute_mel_spectrogram(
    audio: &[f32],
    config: &MelSpectrogramConfig,
) -> Array2<f32> {
    // 1. Apply Hann window
    let window = hann_window(config.n_fft);
    
    // 2. Compute STFT using rustfft
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(config.n_fft);
    
    // 3. Convert to mel scale
    let mel_filters = create_mel_filterbank(
        config.n_fft,
        config.n_mels,
        config.sample_rate
    );
    
    // 4. Apply log scaling
    // 5. Normalize to [-1, 1] range
    
    // Return shape: [n_mels, time_steps]
}
```

### Step 2: Implement ONNX Inference in `src/models/stt.rs`

```rust
use ort::{Session, inputs};
use ndarray::{Array2, Array3};
use crate::audio::{AudioLoader, compute_mel_spectrogram, MelSpectrogramConfig};

pub struct MoonshineSTT {
    encoder: Session,
    decoder: Session,
    tokenizer: Tokenizer,
}

impl MoonshineSTT {
    pub fn new() -> Result<Self> {
        let encoder = Session::builder()?
            .with_intra_threads(1)?
            .commit_from_file("models/stt/moonshine-tiny-encoder.onnx")?;
        
        let decoder = Session::builder()?
            .with_intra_threads(1)?
            .commit_from_file("models/stt/moonshine-tiny-decoder.onnx")?;
        
        let tokenizer = Tokenizer::from_file(
            "models/stt/moonshine-tiny-tokenizer.json"
        )?;
        
        Ok(Self { encoder, decoder, tokenizer })
    }
    
    pub fn transcribe(audio_path: &str) -> Result<String> {
        // 1. Load audio
        let audio = AudioLoader::load(audio_path)?;
        
        // 2. Compute features
        let features = compute_mel_spectrogram(&audio, &Default::default());
        
        // 3. Run encoder
        let encoder_input = Array3::from_shape_vec(
            (1, features.nrows(), features.ncols()),
            features.into_raw_vec()
        )?;
        
        let encoder_outputs = self.encoder.run(inputs![encoder_input]?)?;
        let hidden_states = encoder_outputs[0].try_extract_tensor::<f32>()?;
        
        // 4. Run decoder (autoregressive)
        let mut tokens = vec![1]; // BOS token
        let max_length = 448;
        
        for _ in 0..max_length {
            let decoder_input = Array2::from_shape_vec(
                (1, tokens.len()),
                tokens.iter().map(|&t| t as i64).collect()
            )?;
            
            let decoder_outputs = self.decoder.run(inputs![
                decoder_input,
                hidden_states.view()
            ]?)?;
            
            let logits = decoder_outputs[0].try_extract_tensor::<f32>()?;
            let next_token = argmax(&logits);
            
            if next_token == 2 { break; } // EOS token
            tokens.push(next_token);
        }
        
        // 5. Decode tokens
        let text = self.tokenizer.decode(&tokens[1..], true)?;
        
        Ok(text)
    }
}
```

### Step 3: Create Tokenizer Parser

Add to `src/models/stt.rs`:

```rust
use serde_json::Value;
use std::collections::HashMap;

struct Tokenizer {
    vocab: HashMap<u32, String>,
}

impl Tokenizer {
    fn from_file(path: &str) -> Result<Self> {
        let json: Value = serde_json::from_str(
            &std::fs::read_to_string(path)?
        )?;
        
        let vocab = json["model"]["vocab"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (v.as_u64().unwrap() as u32, k.clone()))
            .collect();
        
        Ok(Self { vocab })
    }
    
    fn decode(&self, tokens: &[u32], skip_special: bool) -> Result<String> {
        let text = tokens
            .iter()
            .filter_map(|&t| {
                if skip_special && (t == 0 || t == 1 || t == 2) {
                    None
                } else {
                    self.vocab.get(&t).cloned()
                }
            })
            .collect::<Vec<_>>()
            .join("");
        
        Ok(text)
    }
}
```

---

## 🧪 Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --test integration_test
```

### Benchmarks
```bash
cargo bench
```

### Examples
```bash
# Basic transcription
cargo run --example transcribe

# Full pipeline
cargo run --example wispr_flow
```

### CLI Testing
```bash
# Transcribe audio
cargo run -- --transcribe tests/fixtures/audio.mp3

# Full Wispr Flow pipeline
cargo run -- --wispr tests/fixtures/audio.mp3

# Text-to-speech (when implemented)
cargo run -- --speak "Hello world"
```

---

## 📊 Dependencies (Cargo.toml)

```toml
[package]
name = "flow"
version = "0.1.0"
edition = "2024"
license = "Apache-2.0"

[dependencies]
# Async Runtime (Latest: 1.50.0 as of March 2026)
tokio = { version = "1.50", features = ["rt-multi-thread", "macros", "sync", "fs", "io-util"] }

# LLM Inference
llama-cpp-2 = "0.1"

# ONNX Runtime for STT/TTS (Latest: 2.0.0-rc.12)
ort = { version = "=2.0.0-rc.12", features = ["download-binaries"] }

# Tensor Operations
ndarray = "0.17"

# Audio Processing & FFT
hound = "3.5"
rodio = "0.22"
cpal = "0.17"
rustfft = "6.2"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error Handling
anyhow = "1.0"
thiserror = "2.0"

# System Info
sysinfo = "0.32"

# Token Counting
tiktoken-rs = "0.9"

# CLI Detection
atty = "0.2"

[dev-dependencies]
criterion = "0.8"

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
```

---

## 🎯 Success Criteria

### Must Have (Phase 1)
- ✅ Professional project structure
- ✅ Modular architecture (audio, cli, models, pipeline, utils)
- ✅ Latest dependencies (Tokio 1.50, ort 2.0.0-rc.12)
- [ ] Real Moonshine ONNX transcription
- [ ] Accurate transcription of test audio (WER < 10%)
- [ ] `--transcribe` command works
- [ ] `--wispr` command works (STT + LLM)

### Should Have (Phase 2)
- [ ] Filler word removal
- [ ] Proper punctuation
- [ ] Clean formatting
- [ ] Fast inference (< 1s for 3s audio)

### Nice to Have (Phase 3+)
- [ ] Kokoro TTS integration
- [ ] Real-time microphone input
- [ ] Streaming transcription
- [ ] Voice activity detection

---

## 🚨 Common Pitfalls & Solutions

### Issue 1: ONNX Threading Errors
**Problem**: `NonNull<OrtSessionOptions>: Send` not satisfied  
**Solution**: Use `.with_intra_threads(1)?` for single-threaded execution

### Issue 2: Tensor Shape Mismatches
**Problem**: Input shape doesn't match model expectations  
**Solution**: Check `models/stt/moonshine-tiny-config.json` for exact shapes

### Issue 3: Audio Loading
**Problem**: MP3 files not loading correctly  
**Solution**: Use rodio for MP3 support, convert to 16kHz mono

### Issue 4: Module Imports
**Problem**: Can't find modules after restructure  
**Solution**: Use `pub use` in mod.rs files, check lib.rs exports

---

## 📚 Resources & References

### Official Documentation
- [Moonshine v2 Model Card](https://huggingface.co/UsefulSensors/moonshine-tiny)
- [ONNX Runtime Rust](https://docs.rs/ort/latest/ort/)
- [Qwen 3.5 Release](https://qwenlm.github.io/blog/qwen3.5/)
- [Kokoro TTS](https://huggingface.co/hexgrad/Kokoro-82M)

### Project Documentation
- `docs/ARCHITECTURE.md` - System architecture
- `docs/DEVELOPMENT.md` - Development guide
- `docs/MODELS.md` - Model documentation
- `docs/API.md` - API documentation

---

## 🚀 Quick Start for Implementation

```bash
# 1. Verify structure
tree src/

# 2. Check dependencies
cargo check

# 3. Implement in this order:
# - src/audio/features.rs (mel spectrogram)
# - src/models/stt.rs (ONNX inference + tokenizer)
# - Test with: cargo run -- --transcribe tests/fixtures/audio.mp3

# 4. Run tests
cargo test

# 5. Run examples
cargo run --example transcribe
```

---

## 📝 Implementation Notes

- **Rust Edition 2024** (latest stable)
- **Professional structure** (modular, testable, documented)
- **Test fixtures** in `tests/fixtures/` (not root)
- **Don't modify README.md** (user documentation)
- **Focus on `src/models/stt.rs`** (main implementation)
- **Use existing modules** (audio, cli, utils)
- **Test incrementally** (unit → integration → examples)

**Priority**: Implement Moonshine ONNX inference in `src/models/stt.rs`

---

**End of Agent Instructions**

*Last updated: March 31, 2026*  
*Project: Flow v0.1.0*  
*Models: Moonshine v2, Qwen 3.5, Kokoro v1.0*
