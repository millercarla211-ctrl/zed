# Download Moonshine v2 Base model for sherpa-onnx
# Model: UsefulSensors/moonshine-base

$baseUrl = "https://huggingface.co/csukuangfj/sherpa-onnx-moonshine-tiny-en-int8/resolve/main"
$outputDir = "models/stt/moonshine-v2"

# Create output directory
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

Write-Host "[DOWNLOAD] Moonshine v2 Tiny (int8 quantized) model..."
Write-Host "[INFO] Downloading to: $outputDir"

# Download encoder
Write-Host "[1/3] Downloading encoder..."
Invoke-WebRequest -Uri "$baseUrl/encoder-epoch-5-avg-1.int8.onnx" -OutFile "$outputDir/encoder.onnx"

# Download decoder
Write-Host "[2/3] Downloading decoder..."
Invoke-WebRequest -Uri "$baseUrl/decoder-epoch-5-avg-1.int8.onnx" -OutFile "$outputDir/decoder.onnx"

# Download tokens
Write-Host "[3/3] Downloading tokens..."
Invoke-WebRequest -Uri "$baseUrl/tokens.txt" -OutFile "$outputDir/tokens.txt"

Write-Host "[DONE] Moonshine v2 model downloaded successfully!"
Write-Host "[INFO] Model size: ~27MB (int8 quantized)"
Write-Host "[INFO] Location: $outputDir"
