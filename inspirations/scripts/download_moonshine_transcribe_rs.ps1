# Download Moonshine Tiny models in transcribe-rs compatible format
# Source: https://huggingface.co/onnx-community/moonshine-tiny-ONNX

$baseUrl = "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main"
$outputDir = "models/stt"

Write-Host "[DOWNLOAD] Moonshine Tiny (transcribe-rs format)..."
Write-Host "[INFO] Downloading to: $outputDir"

# Create output directory
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$downloads = @(
    @{ Label = "encoder"; Remote = "onnx/encoder_model.onnx"; Local = "encoder_model.onnx" },
    @{ Label = "decoder"; Remote = "onnx/decoder_model_merged.onnx"; Local = "decoder_model_merged.onnx" },
    @{ Label = "tokenizer"; Remote = "tokenizer.json"; Local = "tokenizer.json" }
)

for ($i = 0; $i -lt $downloads.Count; $i++) {
    $item = $downloads[$i]
    $target = Join-Path $outputDir $item.Local
    if ((Test-Path $target) -and ((Get-Item $target).Length -gt 0)) {
        Write-Host "[$($i + 1)/$($downloads.Count)] $($item.Local) already exists"
        continue
    }

    Write-Host "[$($i + 1)/$($downloads.Count)] Downloading $($item.Local)..."
    try {
        Invoke-WebRequest -Uri "$baseUrl/$($item.Remote)?download=true" -OutFile $target
        Write-Host "  Downloaded $($item.Local)"
    }
    catch {
        if (Test-Path $target) {
            Remove-Item -LiteralPath $target -Force
        }
        Write-Host "  Failed to download $($item.Label): $_"
    }
}

Write-Host ""
Write-Host "[INFO] Checking downloaded files..."
$files = @("encoder_model.onnx", "decoder_model_merged.onnx", "tokenizer.json")
$allPresent = $true

foreach ($file in $files) {
    $path = Join-Path $outputDir $file
    if (Test-Path $path) {
        $size = (Get-Item $path).Length / 1MB
        Write-Host "  $file ($([math]::Round($size, 2)) MB)"
    }
    else {
        Write-Host "  $file (missing)"
        $allPresent = $false
    }
}

if ($allPresent) {
    Write-Host ""
    Write-Host "[DONE] All Moonshine models downloaded successfully!"
    Write-Host "[INFO] You can now test STT with:"
    Write-Host "       cargo run --release -- --transcribe output.wav"
}
else {
    Write-Host ""
    Write-Host "[ERROR] Some files are missing. Please check the URLs or download manually."
    Write-Host "[INFO] Manual download from: https://huggingface.co/onnx-community/moonshine-tiny-ONNX/tree/main"
}
