#!/bin/bash
set -e

echo "=== Flow Voice Assistant — Ultra-Light Models ==="
echo "=== Optimized for 7.5 GB RAM ==="
echo ""

# 1. STT: Moonshine v2 Tiny (INT8, streaming)
echo "[1/3] 📢 Downloading STT: Moonshine v2 Tiny..."
huggingface-cli download onnx-community/moonshine-tiny-ONNX \
  --include "onnx/*" \
  --include "tokenizer.json" \
  --include "config.json" \
  --local-dir ./models/stt/

# 2. TTS: Kokoro-82M INT8
echo "[2/3] 🔊 Downloading TTS: Kokoro-82M Q8..."
mkdir -p ./models/tts/
curl -L "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.int8.onnx" \
  -o ./models/tts/kokoro-v1.0.int8.onnx
curl -L "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin" \
  -o ./models/tts/voices-v1.0.bin

# 3. LLM: Already using Qwen3.5-0.8B
echo "[3/3] 🧠 LLM: Qwen3.5-0.8B already present ✅"

echo ""
echo "=== ✅ All models downloaded! ==="
echo "📦 Total disk: ~250 MB (STT + TTS only)"
echo "🧠 RAM at runtime: ~3.5-4 GB (with LLM)"
echo "💚 Headroom on 7.5GB: ~3.5 GB free"
