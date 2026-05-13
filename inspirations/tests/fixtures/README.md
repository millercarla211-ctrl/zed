# Test Fixtures

This directory contains test audio files for Flow voice assistant testing.

## Audio Files

### audio.mp3
- **Duration**: 3.14 seconds
- **Format**: MP3, 48kHz mono
- **Content**: "hello mike testing one two three hello"
- **Purpose**: Primary test file for STT accuracy validation
- **Expected WER**: < 10% with Moonshine Tiny

## Usage

```rust
// In tests
let audio_path = "tests/fixtures/audio.mp3";
let stt = MoonshineSTT;
let result = stt.transcribe(audio_path)?;
```

```bash
# CLI testing
cargo run -- --transcribe tests/fixtures/audio.mp3
cargo run -- --wispr tests/fixtures/audio.mp3
```

## Adding New Test Files

When adding new test audio files:
1. Use descriptive names (e.g., `long_speech.mp3`, `noisy_audio.wav`)
2. Document the content and expected output
3. Keep files small (< 10MB) for fast CI/CD
4. Support formats: MP3, WAV, FLAC, OGG
