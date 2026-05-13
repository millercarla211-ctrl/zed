# Model Documentation

## Speech-to-Text (STT)

### Moonshine v2
- **Model**: Moonshine Tiny (35M parameters)
- **Location**: `models/stt/encoder_model.onnx`, `models/stt/decoder_model_merged.onnx`, `models/stt/tokenizer.json`
- **License**: MIT
- **Performance**: ~7.2% WER
- **Latency**: < 1s for 3s audio
- **Use Case**: Smallest local on-demand fallback for low-resource Windows machines

### Parakeet TDT 0.6B v3 INT8
- **Model**: NVIDIA Parakeet TDT 0.6B v3 INT8 via sherpa-onnx
- **Location**: `models/stt/parakeet-tdt-0.6b-v3-int8/`
- **Required Files**: `encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt`
- **Build Feature**: `sherpa-stt`
- **Use Case**: Higher-quality local STT when the model bundle is present
- **Install Handoff**: `powershell -ExecutionPolicy Bypass -File scripts/download_sherpa_parakeet_stt.ps1`

### Nemotron Speech Streaming EN 0.6B INT8
- **Model**: NVIDIA Nemotron Speech Streaming EN 0.6B via sherpa-onnx
- **Location**: `models/stt/nemotron-speech-streaming-en-0.6b-int8/`
- **Required Files**: `encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt`
- **Build Feature**: `sherpa-stt`
- **Use Case**: Explicit NVIDIA/performance STT path; not the default resident engine on AMD Windows
- **Install Handoff**: `powershell -ExecutionPolicy Bypass -File scripts/nemotron-stt-handoff.ps1`

## Wake Words

Flow expects one LiveKit-compatible classifier per command:

- `models/wake_words/dx.onnx`
- `models/wake_words/friday.onnx`
- `models/wake_words/hello.onnx`
- `models/wake_words/aladdin.onnx`
- `models/wake_words/arise.onnx`

Use `docs/WAKEWORD_TRAINING.md` for the Colab/Linux/WSL training handoff.

## Large Language Model (LLM)

### Flow Local Role Policy

- **Fast helper / prompt cleanup / tiny conversions**: `qwen3-0.6b`
- **Tool-agent / strict function calls**: `xlam2-3b-fc-r-q4km`
- **Daily smart chat + coding + UI edits**: `qwen35-4b-revised-q4km`
- **Slow backup when 4B fails and latency is acceptable**: `qwen35-9b-q4km`

Use `cargo run --release --bin flow -- --model-roles` to print the active local role map and file readiness.
Use `cargo run --release --bin flow -- --tool-model-candidates` before downloading more agent models.

### Qwen3.5 4B Revised Q4_K_M
- **Model**: Qwen3.5 4B Revised (Q4_K_M GGUF)
- **Catalog Key**: `qwen35-4b-revised-q4km`
- **Source**: `Smoffyy/Qwen3.5-4B-Instruct-Revised-GGUF`
- **Original Family**: Qwen3.5
- **Location**: `models/llm/Qwen3.5-4B-q4_k_m.gguf`
- **Runtime**: Rust `llama-cpp-2`, CPU-first by default
- **Install**: `cargo run --release --bin flow -- --install-model qwen35-4b-revised-q4km`
- **Smoke Test**: `cargo run --release --bin flow -- --chat qwen35-4b-revised-q4km`
- **Use Case**: Daily local assistant brain for coding, UI edits, shadcn/Tailwind/Next.js work, normal useful answers, and smart chat
- **Runtime Policy**: Flow injects `/no_think`, strips hidden-thinking artifacts, and retries once when the cleaned answer is empty

### Qwen3.5 9B Q4_K_M
- **Model**: Qwen3.5 9B (Q4_K_M GGUF)
- **Catalog Key**: `qwen35-9b-q4km`
- **Source**: `jc-builds/Qwen3.5-9B-Q4_K_M-GGUF`
- **Original Family**: Qwen3.5
- **Location**: `models/llm/Qwen3.5-9B-Q4_K_M.gguf`
- **Runtime**: Rust `llama-cpp-2`, CPU-first by default
- **Install**: `cargo run --release --bin flow -- --install-model qwen35-9b-q4km`
- **Smoke Test**: `cargo run --release --bin flow -- --chat qwen35-9b-q4km`
- **Use Case**: Higher-quality fallback for coding checks when latency is acceptable; not the default loop because it is too slow on this OS

### xLAM-2 3B Function Calling Q4_K_M
- **Model**: xLAM-2 3B function-calling research release (Q4_K_M GGUF)
- **Catalog Key**: `xlam2-3b-fc-r-q4km`
- **Source**: `Salesforce/xLAM-2-3b-fc-r-gguf`
- **Location**: `models/llm/xLAM-2-3B-fc-r-Q4_K_M.gguf`
- **Runtime**: Rust `llama-cpp-2`, CPU-first by default
- **Install**: `cargo run --release --bin flow -- --install-model xlam2-3b-fc-r-q4km`
- **Bounded Tool Test**: `cargo run --release --bin flow -- --tool-agent "choose a tool for this request"`
- **Tools File Test**: `cargo run --release --bin flow -- --tool-agent-tools examples/tool-agent/weather-tools.json "weather in Dhaka tomorrow"`
- **Use Case**: Dedicated local research model for strict JSON tool-routing/function-call decisions
- **License Note**: CC-BY-NC-4.0; keep commercial defaults on Apache/MIT candidates such as Ministral 3 3B or Granite 4.0 H Micro

### Tool-Calling Runner-Ups
- `ministral3-3b-instruct-q4km`: best commercial-safe small general agent/chat replacement candidate; Apache-2.0.
- `granite4-h-micro-q4km`: best tiny commercial-safe structured-output/router candidate; Apache-2.0.
- `phi4-mini-instruct-q4km`: strong small reasoning backup with documented function-calling format; MIT.
- `smollm3-3b-q4km`: fast general small fallback; Apache-2.0.

### Qwen3 0.6B Q4_K_M
- **Model**: Qwen3 0.6B (Q4_K_M GGUF)
- **Catalog Key**: `qwen3-0.6b`
- **Source**: `jc-builds/Qwen3-0.6B-Q4_K_M-GGUF`
- **Location**: `models/llm/Qwen3-0.6B-Q4_K_M.gguf`
- **Runtime**: Rust `llama-cpp-2`, CPU-first by default
- **Install**: `cargo run --release --bin flow -- --install-model qwen3-0.6b`
- **Smoke Test**: `cargo run --release --bin flow -- --chat qwen3-0.6b`
- **Use Case**: Fastest helper for Flow prompt enhancement, text cleanup, short conversions, labels, and tiny rewrites

### Gemma 4 E4B Frontend
- **Model**: Gemma-4-E4B-Frontend (Q4_K_M GGUF + BF16 mmproj)
- **Catalog Key**: `gemma4-e4b-frontend-q4km`
- **Source**: `DuoNeural/Gemma-4-E4B-Frontend-GGUF`
- **Location**: `models/llm/gemma-4-E4B-it.Q4_K_M.gguf`
- **MMProj**: `models/llm/gemma-4-E4B-it.BF16-mmproj.gguf`
- **Runtime**: `llama-cpp-python` vision bridge, CPU-first by default
- **Install**: `cargo run --release --bin flow -- --install-model gemma4-e4b-frontend-q4km`
- **Chat Alias**: `gemma4-e4b-frontend-text-q4km`
- **Use Case**: Avoid for daily chat on this OS. Keep it for UI-generation, vision, and benchmark experiments. The Qwen3.5 4B revised model is the daily smart/chat model.

### WEBGEN 4B Preview i1
- **Model**: WEBGEN-4B-Preview i1 (Q4_K_M GGUF)
- **Catalog Key**: `webgen-4b-preview-i1-q4km`
- **Source**: `mradermacher/WEBGEN-4B-Preview-i1-GGUF`
- **Original**: `Tesslate/WEBGEN-4B-Preview`
- **Location**: `models/llm/WEBGEN-4B-Preview.i1-Q4_K_M.gguf`
- **Runtime**: Rust `llama-cpp-2`, CPU-first by default
- **Install**: `cargo run --release --bin flow -- --install-model webgen-4b-preview-i1-q4km`
- **Use Case**: Local HTML/CSS/Tailwind-style website generation candidate
- **Google Eval Result**: Failed standalone generation in this runtime; output was incomplete and needed recovery before screenshotting

### Qwendean 4B
- **Model**: Qwendean-4B (Q4_K_M GGUF)
- **Catalog Key**: `qwendean-4b-q4km`
- **Source**: `iamdyeus/qwendean-4b-GGUF`
- **Location**: `models/llm/Qwendean-4B.Q4_K_M.gguf`
- **Runtime**: Rust `llama-cpp-2`, CPU-first by default
- **Install**: `cargo run --release --bin flow -- --install-model qwendean-4b-q4km`
- **Use Case**: Default local UI candidate for React/TypeScript/shadcn-style prompts; screenshot eval asks it for standalone HTML/CSS
- **Google Eval Result**: Completed standalone HTML, but rough screenshot diff remained high and the clone is not pass-quality

## Text-to-Speech (TTS)

### Kokoro v1.0
- **Model**: Kokoro 82M (INT8 quantized)
- **Location**: `models/tts/kokoro-v1.0.int8.onnx`
- **License**: Apache 2.0
- **Quality**: #1 on TTS Arena (44% win rate)
- **Latency**: Very fast, runs on CPU

## Model Upgrades

### Recommended Upgrades by RAM:

**4-8GB RAM:**
- STT: Moonshine Base (100M)
- LLM: Qwen 3.5 4B
- TTS: Kokoro 82M

**8-16GB RAM:**
- STT: Moonshine Large (245M)
- LLM: Qwen 3.5 9B
- TTS: Kokoro + CosyVoice2

**16GB+ RAM:**
- STT: Moonshine Large + Canary Qwen
- LLM: Qwen 3.5 27B
- TTS: Voxtral TTS 4B
