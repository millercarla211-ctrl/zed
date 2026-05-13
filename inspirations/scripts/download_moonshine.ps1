# Download Moonshine Tiny ONNX model from HuggingFace
$modelDir = "models/stt"
$baseUrl = "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main"

# Create directory
New-Item -ItemType Directory -Force -Path $modelDir | Out-Null

Write-Host "Downloading Moonshine Tiny ONNX model..."
Write-Host "Size: ~35MB, WER: 7.2%, MIT License"
Write-Host ""

# Download preprocessor
Write-Host "Downloading preprocessor.onnx..."
Invoke-WebRequest -Uri "$baseUrl/preprocessor.onnx" -OutFile "$modelDir/moonshine-tiny-preprocessor.onnx"

# Download encoder
Write-Host "Downloading encoder.onnx..."
Invoke-WebRequest -Uri "$baseUrl/encoder.onnx" -OutFile "$modelDir/moonshine-tiny-encoder.onnx"

# Download decoder
Write-Host "Downloading decoder.onnx..."
Invoke-WebRequest -Uri "$baseUrl/decoder.onnx" -OutFile "$modelDir/moonshine-tiny-decoder.onnx"

# Download tokenizer
Write-Host "Downloading tokenizer.json..."
Invoke-WebRequest -Uri "$baseUrl/tokenizer.json" -OutFile "$modelDir/moonshine-tiny-tokenizer.json"

# Download config
Write-Host "Downloading config.json..."
Invoke-WebRequest -Uri "$baseUrl/config.json" -OutFile "$modelDir/moonshine-tiny-config.json"

Write-Host ""
Write-Host "Moonshine Tiny downloaded successfully"
Write-Host "Location: models/stt"
