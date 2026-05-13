# Flow Architecture

## Project Structure

```
flow/
├── src/
│   ├── audio/           # Audio processing
│   │   ├── features.rs  # Mel spectrogram computation
│   │   └── loader.rs    # Audio file loading
│   ├── cli/             # Command-line interface
│   │   ├── args.rs      # Argument parsing
│   │   └── commands.rs  # Command execution
│   ├── models/          # ML model inference
│   │   ├── llm.rs       # Qwen 3.5 LLM
│   │   ├── stt.rs       # Moonshine STT
│   │   └── tts.rs       # Kokoro TTS
│   ├── pipeline/        # Processing pipelines
│   │   └── voice.rs     # Voice processing pipeline
│   ├── utils/           # Utility functions
│   │   └── system.rs    # System info
│   ├── lib.rs           # Library entry point
│   └── main.rs          # Binary entry point
├── tests/               # Integration tests
├── benches/             # Performance benchmarks
├── examples/            # Usage examples
├── models/              # Model files
│   ├── llm/            # Qwen GGUF models
│   ├── stt/            # Moonshine ONNX models
│   └── tts/            # Kokoro ONNX models
└── scripts/             # Download scripts
```

## Module Overview

### Audio Module
- Audio file loading and preprocessing
- Mel spectrogram feature extraction
- Audio format conversion

### CLI Module
- Command-line argument parsing
- Command execution logic
- User interface

### Models Module
- LLM inference (Qwen 3.5)
- STT inference (Moonshine v2)
- TTS inference (Kokoro v1.0)

### Pipeline Module
- Voice processing pipeline
- Multi-stage processing coordination

### Utils Module
- System information
- Memory management
- Helper functions

## Data Flow

```
Audio File → AudioLoader → Mel Spectrogram → STT Model → Raw Text
                                                              ↓
                                                         LLM Model
                                                              ↓
                                                       Enhanced Text
                                                              ↓
                                                         TTS Model
                                                              ↓
                                                        Audio Output
```

## Design Principles

1. **Modularity**: Each component is independent and testable
2. **Performance**: Zero-copy where possible, efficient memory usage
3. **Extensibility**: Easy to add new models or features
4. **Type Safety**: Leverage Rust's type system for correctness
5. **Error Handling**: Comprehensive error handling with anyhow
