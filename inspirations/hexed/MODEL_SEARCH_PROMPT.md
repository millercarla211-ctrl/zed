# Model Search Requirements for Flow Voice Assistant

**Date**: April 2, 2026  
**Project**: Flow - Open-Source Voice Assistant  
**Current Status**: Need latest best models for production deployment

---

## Search Mission

Find and download the absolute BEST and SMALLEST models available as of April 2, 2026 for:
1. Speech-to-Text (STT)
2. Large Language Model (LLM)
3. Text-to-Speech (TTS)

---

## Requirements & Constraints

### Hardware Constraints
- Target: Consumer hardware (8-16GB RAM typical)
- CPU inference preferred (ONNX Runtime, llama.cpp)
- Must run locally, no API calls

### Performance Requirements
- **STT**: Real-time capable, WER < 10%, streaming support preferred
- **LLM**: Fast inference (<1s response), good reasoning, 1-8B parameters
- **TTS**: Natural voice, low latency (<500ms), emotion support preferred

### Format Requirements
- **STT**: ONNX format (for ort crate in Rust)
- **LLM**: GGUF format (for llama-cpp-2 in Rust), Q4_K_M quantization preferred
- **TTS**: ONNX format (for ort crate in Rust)

---

## Current Project Structure

```
flow/
├── models/
│   ├── llm/          # GGUF models go here
│   ├── stt/          # ONNX STT models (encoder/decoder/tokenizer)
│   └── tts/          # ONNX TTS models
```

---

## Search Tasks

### Task 1: Find Best STT Model (April 2026)

**Search for:**
- Moonshine v2 (if still best in April 2026)
- Any newer streaming STT models released in 2026
- Models that beat Whisper Large v3 with fewer parameters
- ONNX format, INT8 quantized preferred

**Key metrics:**
- Word Error Rate (WER)
- Real-time factor (RTF)
- Model size
- Streaming capability
- Release date (prefer March-April 2026)

**Download requirements:**
- encoder.onnx (or encoder_model_int8.onnx)
- decoder.onnx (or decoder_model_int8.onnx)
- decoder_with_past.onnx (if streaming)
- tokenizer.json
- config.json

**Hugging Face search terms:**
- "moonshine streaming ONNX 2026"
- "speech recognition ONNX INT8 2026"
- "STT model streaming April 2026"

---

### Task 2: Find Best Small LLM (April 2026)

**Search for:**
- Qwen3-8B or newer Qwen models
- DeepSeek-R1-Distill-Qwen-7B (if still competitive)
- Any new small reasoning models released in Q1 2026
- GGUF format, Q4_K_M quantization

**Key metrics:**
- Reasoning capability
- Tokens per second
- Model size (prefer 4-8B parameters)
- Benchmark scores (MMLU, HumanEval, etc.)
- Release date (prefer 2026)

**Download requirements:**
- Single GGUF file (Q4_K_M quantization)
- Size: 4-6GB ideal

**Hugging Face search terms:**
- "Qwen3 8B GGUF Q4_K_M"
- "DeepSeek R1 Distill Qwen 7B GGUF"
- "small reasoning LLM GGUF 2026"
- "best 7B 8B model April 2026"

**Specific repos to check:**
- bartowski/DeepSeek-R1-Distill-Qwen-7B-GGUF
- Qwen/Qwen3-8B-GGUF
- bartowski/Qwen_Qwen3-8B-GGUF

---

### Task 3: Find Best TTS Model (April 2026)

**Search for:**
- Fish Audio S1 or S2 (if S2 released by April 2026)
- Kokoro v1.0 or newer
- Any new open-source TTS models from Q1 2026
- ONNX format, INT8 quantized preferred

**Key metrics:**
- TTS-Arena ranking
- Naturalness score
- Latency
- Model size
- Emotion/prosody control
- Release date (prefer 2026)

**Download requirements:**
- TTS model in ONNX format
- Voice data files
- Config files

**Hugging Face search terms:**
- "Fish Audio S1 ONNX"
- "Fish Audio S2 ONNX 2026"
- "Kokoro TTS ONNX INT8"
- "best TTS model ONNX April 2026"
- "TTS Arena #1 2026"

**Specific repos to check:**
- fishaudio/s1-mini (0.5B, open-source)
- hexgrad/Kokoro-82M
- Any new "fish-speech" or "kokoro" variants

---

## Download Instructions Format

For each model found, provide:

1. **Model Name**: Full name and version
2. **Release Date**: When it was released
3. **Why Best**: Key advantages over alternatives
4. **Performance Stats**: WER/benchmarks/latency
5. **Size**: File size and parameter count
6. **Download Command**: Exact huggingface-cli command
7. **File List**: All files that will be downloaded
8. **Destination**: Where to place in `models/` directory

### Example Download Command Format:

```bash
# STT Model
huggingface-cli download Mazino0/moonshine-streaming-medium-onnx \
  --include "encoder_model_int8.onnx" \
  --include "decoder_model_int8.onnx" \
  --include "decoder_with_past_model_int8.onnx" \
  --include "tokenizer.json" \
  --include "config.json" \
  --local-dir ./models/stt/

# LLM Model
huggingface-cli download bartowski/DeepSeek-R1-Distill-Qwen-7B-GGUF \
  --include "DeepSeek-R1-Distill-Qwen-7B-Q4_K_M.gguf" \
  --local-dir ./models/llm/

# TTS Model
huggingface-cli download fishaudio/s1-mini \
  --include "*.onnx" \
  --include "*.bin" \
  --local-dir ./models/tts/
```

---

## Comparison Criteria

When comparing models, prioritize:

1. **Recency**: Models from 2026 > 2025
2. **Performance**: Benchmark scores and real-world metrics
3. **Size**: Smaller is better (if performance is similar)
4. **Format**: Must be ONNX (STT/TTS) or GGUF (LLM)
5. **Open-source**: Fully open weights, no API required
6. **Community adoption**: Active repos, recent updates

---

## Expected Output

Provide a structured report with:

### Section 1: STT Model Recommendation
- Model name and version
- Why it's the best choice
- Download commands
- Expected performance

### Section 2: LLM Model Recommendation
- Model name and version
- Why it's the best choice
- Download commands
- Expected performance

### Section 3: TTS Model Recommendation
- Model name and version
- Why it's the best choice
- Download commands
- Expected performance

### Section 4: Alternative Options
- Second-best choices for each category
- Trade-offs (size vs quality, speed vs accuracy)

---

## Important Notes

- **Date matters**: April 2, 2026 is the reference date
- **Search recent releases**: Check for models released in Q1 2026
- **Verify availability**: Ensure models are actually downloadable
- **Check format**: ONNX for STT/TTS, GGUF for LLM
- **Quantization**: INT8 for ONNX, Q4_K_M for GGUF preferred
- **No API models**: Must be downloadable weights, not API services

---

## Success Criteria

The search is successful when you provide:

✅ 3 specific model recommendations (STT, LLM, TTS)  
✅ Exact download commands that work  
✅ Performance metrics and comparisons  
✅ File sizes and hardware requirements  
✅ Evidence these are the best available as of April 2, 2026  

---

**End of Search Prompt**
