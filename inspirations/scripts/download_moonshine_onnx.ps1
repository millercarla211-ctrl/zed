# Download Moonshine Tiny ONNX models from HuggingFace
$baseUrl = "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main"
$outputDir = "models/stt"

Write-Host "Downloading Moonshine Tiny ONNX models..." -ForegroundColor Green

# Create output directory if it doesn't exist
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

# Download encoder model (quantized, smaller)
$encoderUrl = "$baseUrl/onnx/encoder_model_quantized.onnx"
$encoderPath = "$outputDir/moonshine-tiny-encoder.onnx"
Write-Host "Downloading encoder model..."
Invoke-WebRequest -Uri $encoderUrl -OutFile $encoderPath

# Download decoder model (quantized, smaller)
$decoderUrl = "$baseUrl/onnx/decoder_model_quantized.onnx"
$decoderPath = "$outputDir/moonshine-tiny-decoder.onnx"
Write-Host "Downloading decoder model..."
Invoke-WebRequest -Uri $decoderUrl -OutFile $decoderPath

Write-Host "Download complete!" -ForegroundColor Green
Write-Host "Models saved to: $outputDir"
