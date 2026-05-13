# Download Whisper Tiny GGML model
$modelDir = "models/stt"
$modelUrl = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"

# Create directory
New-Item -ItemType Directory -Force -Path $modelDir | Out-Null

Write-Host "Downloading Whisper Tiny GGML model..."
Write-Host "Size: ~75MB"
Write-Host ""

# Download model
Write-Host "Downloading ggml-tiny.bin..."
Invoke-WebRequest -Uri $modelUrl -OutFile "$modelDir/ggml-tiny.bin"

Write-Host ""
Write-Host "Whisper Tiny downloaded successfully"
Write-Host "Location: $modelDir/ggml-tiny.bin"
