# Flow - Roadmap to Beat Wispr Flow

> **Goal**: Build the best open-source, 100% local voice assistant that surpasses Wispr Flow  
> **Date**: April 2, 2026  
> **Current Status**: Foundation complete, STT/TTS mock implementations working

---

## 🎯 Wispr Flow Features (As of April 2026)

### Core Features
1. **Real-time Auto-Editing**
   - Removes "um", "uh", and filler words automatically
   - Self-corrections ("4 pm, actually 3 pm" → "3 pm")
   - Automatic punctuation based on pauses and tone
   - Numbered list formatting

2. **Context-Aware Formatting**
   - Tone matching (professional vs casual)
   - App-specific formatting (Slack vs email vs docs)
   - Automatic capitalization

3. **Multi-Language Support**
   - 100+ languages
   - Mixed-language dictation (Hinglish, Spanglish, etc.)

4. **Smart Features**
   - Dictionary that learns corrected spellings
   - Snippets for reusable voice shortcuts
   - Context recognition for uncommon names
   - Transcript history

5. **System Integration**
   - Works across all apps (Mac, Windows, iOS, Android)
   - Hotkey activation
   - Floating bubble UI (Android)
   - Direct text insertion via accessibility APIs

### Wispr Flow Limitations (Our Advantages)
- ❌ Cloud-based (privacy concerns)
- ❌ $19/month subscription
- ❌ Requires internet connection
- ❌ Potential AI "hallucinations"
- ❌ Recording limitations
- ❌ Closed source

---

## 🚀 Flow Advantages (100% Local & Open Source)

### What We Already Have
- ✅ 100% local processing (no cloud, no internet needed)
- ✅ Free and open source
- ✅ Ultra-lightweight models (< 1GB total)
- ✅ Fast inference (< 1s for 3s audio)
- ✅ Basic filler word removal
- ✅ Voice Activity Detection (VAD)
- ✅ Live microphone recording
- ✅ Audio playback

### What We Need to Build
Everything below is organized by priority and includes specific Rust crates to use.

---

## 📋 Implementation Roadmap

### Phase 1: Core STT/TTS (PRIORITY)

#### 1.1 Real Moonshine STT Implementation
**Status**: Mock implementation exists  
**Priority**: CRITICAL  
**Effort**: High

**Tasks**:
- [ ] Implement mel spectrogram feature extraction (80 mel bins, 16kHz)
- [ ] Load Moonshine ONNX encoder/decoder models
- [ ] Implement encoder inference (audio → hidden states)
- [ ] Implement decoder inference (hidden states → tokens)
- [ ] Parse tokenizer.json and decode tokens to text
- [ ] Test with `tests/fixtures/audio.mp3`
- [ ] Verify WER < 10%

**Crates to use**:
```bash
# Already have these
cargo add ort --features download-binaries
cargo add ndarray
cargo add rustfft
cargo add hound
```

**Files to modify**:
- `src/audio/features.rs` - Mel spectrogram computation
- `src/models/stt.rs` - ONNX inference implementation

---

#### 1.2 Real Kokoro TTS Implementation
**Status**: Mock implementation (sine wave)  
**Priority**: HIGH  
**Effort**: High

**Tasks**:
- [ ] Implement text-to-phoneme conversion using espeak-ng
- [ ] Load Kokoro ONNX model
- [ ] Load voice embeddings from `voices-v1.0.bin`
- [ ] Implement ONNX inference (phonemes + style → audio)
- [ ] Support multiple voices (af_sky, af_bella, am_adam, etc.)
- [ ] Test audio quality

**Crates to use**:
```bash
cargo add espeak-rs  # Already added
# Or use kokorox for complete implementation:
cargo add kokorox
```

**Alternative**: Use `kokorox` crate directly (easier but less control)

**Files to modify**:
- `src/models/tts.rs` - Real Kokoro ONNX inference

---

### Phase 2: Audio Enhancement (Beat Wispr Flow)

#### 2.1 Noise Cancellation & Audio Enhancement
**Status**: Not implemented  
**Priority**: HIGH  
**Effort**: Medium

**Tasks**:
- [ ] Integrate Silero VAD for better voice detection
- [ ] Add noise reduction/suppression
- [ ] Implement audio enhancement (normalize, denoise)
- [ ] Test in noisy environments

**Crates to use**:
```bash
# Option 1: Use voice-engine (comprehensive but deprecated, merged into active-call)
cargo add nnnoiseless  # Noise reduction

# Option 2: Build custom solution
cargo add dasp  # Digital audio signal processing
cargo add rubato  # Resampling
```

**Recommended approach**: Use `nnnoiseless` for noise reduction

**Files to create**:
- `src/audio/enhancement.rs` - Noise reduction and audio enhancement

---

#### 2.2 Advanced Filler Word Removal
**Status**: Basic implementation exists  
**Priority**: MEDIUM  
**Effort**: Low

**Tasks**:
- [ ] Expand filler word list (language-specific)
- [ ] Context-aware removal (don't remove "like" in "I like pizza")
- [ ] Confidence-based removal (only remove if STT is confident)
- [ ] Support for multiple languages

**Crates to use**:
```bash
cargo add whatlang  # Language detection
cargo add unicode-segmentation  # Text segmentation
```

**Files to modify**:
- `src/models/stt.rs` - Enhanced `enhance_transcript()` function

---

### Phase 3: Smart Text Enhancement

#### 3.1 Context-Aware Formatting
**Status**: Not implemented  
**Priority**: HIGH  
**Effort**: Medium

**Tasks**:
- [ ] Detect context (email, code, chat, document)
- [ ] Apply context-specific formatting rules
- [ ] Professional vs casual tone detection
- [ ] Smart capitalization (names, acronyms, etc.)

**Crates to use**:
```bash
cargo add regex  # Already have
cargo add unicode-normalization  # Text normalization
```

**Files to create**:
- `src/pipeline/formatter.rs` - Context-aware text formatting

---

#### 3.2 Smart Punctuation
**Status**: Basic implementation (adds period at end)  
**Priority**: MEDIUM  
**Effort**: Medium

**Tasks**:
- [ ] Detect sentence boundaries
- [ ] Add commas based on pauses
- [ ] Question mark detection (rising intonation)
- [ ] Exclamation mark detection (emphasis)
- [ ] Quote handling

**Approach**: Use LLM (Qwen 3.5) for intelligent punctuation

**Files to modify**:
- `src/models/llm.rs` - Add punctuation-specific prompts

---

#### 3.3 Self-Corrections
**Status**: Not implemented  
**Priority**: MEDIUM  
**Effort**: High

**Tasks**:
- [ ] Detect correction patterns ("actually", "I mean", "no wait")
- [ ] Parse and apply corrections
- [ ] Handle multiple corrections in sequence
- [ ] Test with real-world examples

**Example**:
- Input: "Meet at 4 pm, actually 3 pm"
- Output: "Meet at 3 pm"

**Crates to use**:
```bash
cargo add pest  # Parser generator
# Or use regex for simpler patterns
```

**Files to create**:
- `src/pipeline/corrections.rs` - Self-correction detection and application

---

### Phase 4: Advanced Features

#### 4.1 Personal Dictionary
**Status**: Not implemented  
**Priority**: MEDIUM  
**Effort**: Medium

**Tasks**:
- [ ] Store user-specific corrections
- [ ] Learn from manual edits
- [ ] Sync across sessions
- [ ] Export/import dictionary

**Crates to use**:
```bash
cargo add sled  # Embedded database
# Or use simple JSON file
cargo add serde_json  # Already have
```

**Files to create**:
- `src/utils/dictionary.rs` - Personal dictionary management

---

#### 4.2 Voice Snippets
**Status**: Not implemented  
**Priority**: LOW  
**Effort**: Low

**Tasks**:
- [ ] Define voice shortcuts ("my email" → "user@example.com")
- [ ] Support variables ("today's date" → "April 2, 2026")
- [ ] Snippet management UI (CLI)

**Files to create**:
- `src/utils/snippets.rs` - Voice snippet management

---

#### 4.3 Multi-Language Support
**Status**: English only  
**Priority**: MEDIUM  
**Effort**: High

**Tasks**:
- [ ] Add language detection
- [ ] Support mixed-language dictation
- [ ] Language-specific filler words
- [ ] Language-specific formatting rules

**Crates to use**:
```bash
cargo add whatlang  # Language detection
cargo add lingua-rs  # More accurate language detection
```

**Files to modify**:
- `src/models/stt.rs` - Multi-language support
- `src/models/tts.rs` - Multi-language voices

---

### Phase 5: System Integration

#### 5.1 Hotkey Activation
**Status**: Not implemented  
**Priority**: HIGH  
**Effort**: Medium

**Tasks**:
- [ ] Global hotkey registration (Windows)
- [ ] Start recording on hotkey press
- [ ] Stop recording on hotkey release (push-to-talk)
- [ ] Or toggle mode (press once to start, press again to stop)

**Crates to use**:
```bash
cargo add global-hotkey  # Cross-platform hotkey
cargo add rdev  # Device event listening
```

**Files to create**:
- `src/utils/hotkey.rs` - Global hotkey management

---

#### 5.2 Clipboard Integration
**Status**: Not implemented  
**Priority**: HIGH  
**Effort**: Low

**Tasks**:
- [ ] Copy transcribed text to clipboard
- [ ] Paste automatically (optional)
- [ ] Clipboard history

**Crates to use**:
```bash
cargo add arboard  # Cross-platform clipboard
```

**Files to create**:
- `src/utils/clipboard.rs` - Clipboard integration

---

#### 5.3 System Tray Icon
**Status**: Not implemented  
**Priority**: MEDIUM  
**Effort**: Medium

**Tasks**:
- [ ] System tray icon
- [ ] Quick settings menu
- [ ] Start/stop recording
- [ ] Show status (listening, processing, etc.)

**Crates to use**:
```bash
cargo add tray-icon  # System tray
cargo add winit  # Window management (if needed)
```

**Files to create**:
- `src/ui/tray.rs` - System tray implementation

---

### Phase 6: Performance & Quality

#### 6.1 Streaming Transcription
**Status**: Not implemented  
**Priority**: MEDIUM  
**Effort**: High

**Tasks**:
- [ ] Implement streaming STT (Moonshine supports this)
- [ ] Real-time text updates
- [ ] Partial results display
- [ ] Lower latency

**Files to modify**:
- `src/models/stt.rs` - Streaming inference
- `src/pipeline/voice.rs` - Streaming pipeline

---

#### 6.2 Model Optimization
**Status**: Using quantized models  
**Priority**: LOW  
**Effort**: Medium

**Tasks**:
- [ ] Benchmark current performance
- [ ] Optimize ONNX inference (GPU support)
- [ ] Reduce memory usage
- [ ] Faster model loading

**Crates to use**:
```bash
# Already using ort with download-binaries
# For GPU support, may need to compile with CUDA/DirectML
```

---

#### 6.3 Quality Metrics
**Status**: Not implemented  
**Priority**: LOW  
**Effort**: Low

**Tasks**:
- [ ] Measure Word Error Rate (WER)
- [ ] Measure Real-Time Factor (RTF)
- [ ] Measure latency (end-to-end)
- [ ] Create benchmark suite

**Files to create**:
- `benches/quality_metrics.rs` - Quality benchmarks

---

## 🎨 User Experience Improvements

### 7.1 Better CLI Interface
**Status**: Basic CLI exists  
**Priority**: MEDIUM  
**Effort**: Low

**Tasks**:
- [ ] Add progress bars
- [ ] Better error messages
- [ ] Colorized output
- [ ] Interactive mode improvements

**Crates to use**:
```bash
cargo add indicatif  # Progress bars
cargo add colored  # Colored terminal output
cargo add dialoguer  # Interactive prompts
```

---

### 7.2 Configuration File
**Status**: Not implemented  
**Priority**: MEDIUM  
**Effort**: Low

**Tasks**:
- [ ] Create config file format (TOML/YAML)
- [ ] Load user preferences
- [ ] Save settings
- [ ] Config validation

**Crates to use**:
```bash
cargo add config  # Configuration management
cargo add toml  # TOML parsing
```

**Files to create**:
- `src/utils/config.rs` - Configuration management
- `flow.toml` - Default configuration file

---

### 7.3 Logging & Debugging
**Status**: Basic println! logging  
**Priority**: LOW  
**Effort**: Low

**Tasks**:
- [ ] Structured logging
- [ ] Log levels (debug, info, warn, error)
- [ ] Log to file
- [ ] Performance tracing

**Crates to use**:
```bash
cargo add tracing  # Structured logging
cargo add tracing-subscriber  # Log formatting
```

---

## 📊 Comparison: Flow vs Wispr Flow

| Feature | Wispr Flow | Flow (Current) | Flow (Target) |
|---------|-----------|----------------|---------------|
| **Privacy** | ❌ Cloud-based | ✅ 100% Local | ✅ 100% Local |
| **Cost** | ❌ $19/month | ✅ Free | ✅ Free |
| **Open Source** | ❌ Closed | ✅ Open | ✅ Open |
| **Internet Required** | ❌ Yes | ✅ No | ✅ No |
| **STT Quality** | ⭐⭐⭐⭐⭐ | ⭐⭐ (mock) | ⭐⭐⭐⭐⭐ |
| **TTS Quality** | N/A | ⭐⭐ (mock) | ⭐⭐⭐⭐⭐ |
| **Filler Removal** | ✅ Yes | ✅ Basic | ✅ Advanced |
| **Auto Punctuation** | ✅ Yes | ⚠️ Basic | ✅ Yes |
| **Self-Corrections** | ✅ Yes | ❌ No | ✅ Yes |
| **Context-Aware** | ✅ Yes | ❌ No | ✅ Yes |
| **Multi-Language** | ✅ 100+ | ⚠️ English | ✅ 100+ |
| **Noise Cancellation** | ✅ Yes | ❌ No | ✅ Yes |
| **Personal Dictionary** | ✅ Yes | ❌ No | ✅ Yes |
| **Voice Snippets** | ✅ Yes | ❌ No | ✅ Yes |
| **Hotkey Activation** | ✅ Yes | ❌ No | ✅ Yes |
| **System Integration** | ✅ Yes | ❌ No | ✅ Yes |
| **Streaming** | ✅ Yes | ❌ No | ✅ Yes |
| **RAM Usage** | Unknown | ✅ ~4GB | ✅ ~4GB |
| **Latency** | < 1s | ~1s | < 0.5s |

---

## 🎯 Success Criteria

### Minimum Viable Product (MVP)
- [ ] Real Moonshine STT (WER < 10%)
- [ ] Real Kokoro TTS (natural voice)
- [ ] Noise cancellation
- [ ] Advanced filler word removal
- [ ] Smart punctuation
- [ ] Hotkey activation
- [ ] Clipboard integration

### Feature Parity with Wispr Flow
- [ ] All MVP features
- [ ] Self-corrections
- [ ] Context-aware formatting
- [ ] Personal dictionary
- [ ] Voice snippets
- [ ] Multi-language support
- [ ] Streaming transcription

### Beyond Wispr Flow
- [ ] 100% local (no cloud)
- [ ] Free and open source
- [ ] Lower latency (< 0.5s)
- [ ] Better privacy
- [ ] Customizable (open source)
- [ ] Plugin system (future)

---

## 📅 Timeline Estimate

### Phase 1: Core STT/TTS (2-3 weeks)
- Week 1: Moonshine STT implementation
- Week 2: Kokoro TTS implementation
- Week 3: Testing and refinement

### Phase 2: Audio Enhancement (1 week)
- Noise cancellation and audio enhancement

### Phase 3: Smart Text Enhancement (2 weeks)
- Context-aware formatting, smart punctuation, self-corrections

### Phase 4: Advanced Features (2 weeks)
- Personal dictionary, voice snippets, multi-language

### Phase 5: System Integration (1 week)
- Hotkey, clipboard, system tray

### Phase 6: Performance & Quality (1 week)
- Streaming, optimization, benchmarks

**Total: ~10 weeks to feature parity**

---

## 🚀 Quick Start Commands

### Add Dependencies (Use Latest Versions)
```bash
# Core (already have most)
cargo add ort --features download-binaries
cargo add ndarray
cargo add rustfft
cargo add hound
cargo add rodio
cargo add cpal
cargo add espeak-rs

# Audio Enhancement
cargo add nnnoiseless
cargo add dasp
cargo add rubato

# Text Processing
cargo add whatlang
cargo add unicode-segmentation
cargo add unicode-normalization
cargo add regex

# System Integration
cargo add global-hotkey
cargo add arboard
cargo add tray-icon

# Database & Config
cargo add sled
cargo add config
cargo add toml

# UI & Logging
cargo add indicatif
cargo add colored
cargo add dialoguer
cargo add tracing
cargo add tracing-subscriber
```

### Test Current Implementation
```bash
# Test live recording
cargo run -- --live

# Test transcription
cargo run -- --transcribe tests/fixtures/audio.mp3

# Test full pipeline
cargo run -- --wispr tests/fixtures/audio.mp3
```

---

## 📚 Resources

### Models
- [Moonshine v2](https://huggingface.co/UsefulSensors/moonshine-tiny) - STT
- [Kokoro v1.0](https://huggingface.co/hexgrad/Kokoro-82M) - TTS
- [Qwen 3.5](https://qwenlm.github.io/blog/qwen3.5/) - LLM

### Rust Crates
- [ort](https://docs.rs/ort) - ONNX Runtime
- [rodio](https://docs.rs/rodio) - Audio playback
- [cpal](https://docs.rs/cpal) - Audio I/O
- [nnnoiseless](https://docs.rs/nnnoiseless) - Noise reduction

### Inspiration
- [Wispr Flow](https://wisprflow.ai) - Commercial reference
- [Kokoros](https://github.com/lucasjinreal/Kokoros) - Rust Kokoro implementation
- [kokorox](https://lib.rs/crates/kokorox) - Complete Kokoro crate

---

**Last Updated**: April 2, 2026  
**Status**: Roadmap complete, ready for implementation  
**Next Step**: Implement Phase 1.1 (Real Moonshine STT)
