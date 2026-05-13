# Flow - Quick Start Guide

> **Ultra-lightweight, 100% local voice assistant**  
> **Status**: Foundation complete, ready for Phase 1 implementation

---

## 🚀 Current Status

### ✅ What Works Now
- Live microphone recording with Voice Activity Detection
- Audio playback (fixed!)
- Basic text enhancement (filler word removal, punctuation)
- LLM integration (Qwen 3.5 0.8B)
- CLI interface

### ⚠️ What's Mock (Needs Implementation)
- STT: Returns hardcoded text instead of real transcription
- TTS: Generates sine wave instead of real speech

---

## 🎯 Quick Commands

### Test Live Recording
```bash
cargo run --release -- --live
```
This will:
1. Listen for speech (VAD detects when you start talking)
2. Record until silence detected
3. "Transcribe" (currently mock - returns hardcoded text)
4. Enhance text (remove fillers, add punctuation)
5. Play audio (currently sine wave)

### Test Transcription
```bash
cargo run --release -- --transcribe tests/fixtures/audio.mp3
```

### Test Full Pipeline
```bash
cargo run --release -- --wispr tests/fixtures/audio.mp3
```

### Test Text-to-Speech
```bash
cargo run --release -- --speak "Hello world"
```

---

## 📋 Next Steps (For You or Next AI)

### Phase 1: Real STT/TTS (CRITICAL)

#### 1. Implement Real Moonshine STT
**File**: `src/models/stt.rs`

**What to do**:
1. Implement mel spectrogram in `src/audio/features.rs`:
   - 80 mel bins
   - 16kHz sample rate
   - Use `rustfft` crate
   
2. Load ONNX models:
   ```rust
   let encoder = Session::builder()?
       .with_intra_threads(1)?
       .commit_from_file("models/stt/moonshine-tiny-encoder.onnx")?;
   
   let decoder = Session::builder()?
       .with_intra_threads(1)?
       .commit_from_file("models/stt/moonshine-tiny-decoder.onnx")?;
   ```

3. Run encoder: audio features → hidden states
4. Run decoder: hidden states → token IDs (autoregressive)
5. Parse `models/stt/moonshine-tiny-tokenizer.json`
6. Decode tokens to text

**Test**: Should transcribe `tests/fixtures/audio.mp3` correctly  
**Expected**: "hello mike testing one two three hello"

---

#### 2. Implement Real Kokoro TTS
**File**: `src/models/tts.rs`

**Option A - Use kokorox crate (EASIER)**:
```bash
cargo add kokorox
```
Then use the crate directly (see kokorox docs)

**Option B - Implement from scratch**:
1. Text → phonemes (using espeak-rs - already added)
2. Phonemes → tokens
3. Load voice embeddings from `models/tts/voices-v1.0.bin`
4. Run ONNX inference:
   ```rust
   let sess = InferenceSession::new("models/tts/kokoro-v1.0.int8.onnx")?;
   let audio = sess.run(None, dict(
       input_ids=tokens,
       style=ref_s,
       speed=np.ones(1, dtype=np.float32),
   ))?[0];
   ```

**Test**: Should generate natural-sounding speech

---

### Phase 2: Audio Enhancement

#### Add Noise Cancellation
```bash
cargo add nnnoiseless
```

**File**: Create `src/audio/enhancement.rs`

**What to do**:
1. Apply noise reduction to recorded audio
2. Normalize audio levels
3. Test in noisy environment

---

### Phase 3: Smart Features

#### Self-Corrections
**File**: Create `src/pipeline/corrections.rs`

**What to do**:
1. Detect patterns: "actually", "I mean", "no wait"
2. Parse and apply corrections
3. Example: "4 pm, actually 3 pm" → "3 pm"

#### Context-Aware Formatting
**File**: Create `src/pipeline/formatter.rs`

**What to do**:
1. Detect context (email, code, chat)
2. Apply context-specific rules
3. Professional vs casual tone

---

### Phase 4: System Integration

#### Hotkey Activation
```bash
cargo add global-hotkey
```

**File**: Create `src/utils/hotkey.rs`

**What to do**:
1. Register global hotkey (e.g., Ctrl+Shift+Space)
2. Start recording on press
3. Stop on release (push-to-talk)

#### Clipboard Integration
```bash
cargo add arboard
```

**File**: Create `src/utils/clipboard.rs`

**What to do**:
1. Copy transcribed text to clipboard
2. Optional auto-paste

---

## 📚 Important Files

### Documentation
- `ROADMAP_TO_BEAT_WISPRFLOW.md` - Complete 10-week roadmap
- `SESSION_SUMMARY.md` - What was accomplished this session
- `AGENTS.md` - Original project instructions
- `README.md` - User-facing documentation

### Code Structure
```
src/
├── audio/
│   ├── features.rs      # ⚠️ TODO: Implement mel spectrogram
│   ├── loader.rs        # ✅ Audio loading (MP3, WAV)
│   ├── player.rs        # ✅ Audio playback (FIXED!)
│   └── recorder.rs      # ✅ Microphone + VAD
├── models/
│   ├── stt.rs          # ⚠️ TODO: Real Moonshine ONNX
│   ├── tts.rs          # ⚠️ TODO: Real Kokoro ONNX
│   └── llm.rs          # ✅ Qwen 3.5 (working)
├── pipeline/
│   └── voice.rs        # ✅ Pipeline orchestration
└── cli/
    ├── args.rs         # ✅ CLI arguments
    └── commands.rs     # ✅ Command execution
```

### Models
```
models/
├── stt/
│   ├── moonshine-tiny-encoder.onnx      # ✅ Downloaded
│   ├── moonshine-tiny-decoder.onnx      # ✅ Downloaded
│   ├── moonshine-tiny-config.json       # ✅ Downloaded
│   └── moonshine-tiny-tokenizer.json    # ✅ Downloaded
├── tts/
│   ├── kokoro-v1.0.int8.onnx           # ✅ Downloaded
│   └── voices-v1.0.bin                 # ✅ Downloaded
└── llm/
    └── Qwen3.5-0.8B-Q4_K_M.gguf        # ✅ Downloaded
```

---

## 🔍 Debugging Tips

### Check if models are loaded
```bash
ls -lh models/stt/
ls -lh models/tts/
ls -lh models/llm/
```

### Test audio loading
```bash
cargo run --release -- --transcribe tests/fixtures/audio.mp3
```

### Check compilation
```bash
cargo check
```

### Run with verbose output
```bash
RUST_LOG=debug cargo run --release -- --live
```

---

## 📊 Performance Targets

### Current (Mock)
- Latency: ~1s (mostly fake delays)
- RAM: ~4GB
- CPU: Low (no real inference)

### Target (Real Implementation)
- Latency: < 0.5s (end-to-end)
- RAM: ~4GB (same)
- CPU: Medium (local inference)
- WER: < 10% (Word Error Rate)
- RTF: < 0.3 (Real-Time Factor)

---

## 🎯 Success Criteria

### Minimum Viable Product (MVP)
- [ ] Real Moonshine STT (accurate transcription)
- [ ] Real Kokoro TTS (natural speech)
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
- [x] 100% local (no cloud) ✅
- [x] Free and open source ✅
- [ ] Lower latency (< 0.5s)
- [x] Better privacy ✅
- [x] Customizable ✅

---

## 🚨 Common Issues

### Issue: "No input device available"
**Solution**: Make sure microphone is connected and permissions granted

### Issue: "No output device available"
**Solution**: Make sure speakers/headphones are connected

### Issue: ONNX model not found
**Solution**: Run download scripts in `scripts/` folder

### Issue: Compilation takes forever
**Solution**: Use `cargo build --release` (optimized) or `cargo check` (faster)

---

## 💡 Pro Tips

1. **Use `--release` for testing** - Much faster than debug builds
2. **Test incrementally** - Don't implement everything at once
3. **Use web search** - Always search for latest crate versions
4. **Read the roadmap** - Everything is documented in `ROADMAP_TO_BEAT_WISPRFLOW.md`
5. **Check examples** - Look at `examples/` folder for usage patterns

---

## 🎉 You're Ready!

The foundation is solid. The roadmap is clear. The models are downloaded.

**Next step**: Implement real Moonshine STT in `src/models/stt.rs`

Good luck! 🚀

---

**Last Updated**: April 2, 2026  
**Project**: Flow v0.1.0  
**Status**: Ready for Phase 1 implementation
