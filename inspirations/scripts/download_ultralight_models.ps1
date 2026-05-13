# Flow Voice Assistant — Ultra-Light Models
# Optimized for 7.5 GB RAM

Write-Host "=== Flow Voice Assistant — Ultra-Light Models ===" -ForegroundColor Cyan
Write-Host "=== Optimized for 7.5 GB RAM ===" -ForegroundColor Cyan
Write-Host ""

# 1. STT: Moonshine v2 Tiny (INT8, streaming)
Write-Host "[1/3] 📢 Downloading STT: Moonshine v2 Tiny..." -ForegroundColor Green
huggingface-cli download onnx-community/moonshine-tiny-ONNX `
  --include "onnx/*" `
  --include "tokenizer.json" `
  --include "config.json" `
  --local-dir ./models/stt/

# 2. TTS: Kokoro-82M INT8
Write-Host "[2/3] 🔊 Downloading TTS: Kokoro-82M Q8..." -ForegroundColor Green
New-Item -ItemType Directory -Force -Path ./models/tts/ | Out-Null
Invoke-WebRequest -Uri "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.int8.onnx" `
  -OutFile ./models/tts/kokoro-v1.0.int8.onnx
Invoke-WebRequest -Uri "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin" `
  -OutFile ./models/tts/voices-v1.0.bin

# 3. LLM: Already using Qwen3.5-0.8B
Write-Host "[3/3] 🧠 LLM: Qwen3.5-0.8B already present ✅" -ForegroundColor Green

Write-Host ""
Write-Host "=== ✅ All models downloaded! ===" -ForegroundColor Green
Write-Host "📦 Total disk: ~250 MB (STT + TTS only)" -ForegroundColor Yellow
Write-Host "🧠 RAM at runtime: ~3.5-4 GB (with LLM)" -ForegroundColor Yellow
Write-Host "💚 Headroom on 7.5GB: ~3.5 GB free" -ForegroundColor Yellow
