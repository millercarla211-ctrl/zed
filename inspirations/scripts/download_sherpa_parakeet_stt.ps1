$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$targetDir = Join-Path $repoRoot "models\stt\parakeet-tdt-0.6b-v3-int8"
$repoId = "csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8"
$baseUrl = "https://huggingface.co/$repoId/resolve/main"
$files = @(
    "encoder.int8.onnx",
    "decoder.int8.onnx",
    "joiner.int8.onnx",
    "tokens.txt"
)

New-Item -ItemType Directory -Force -Path $targetDir | Out-Null

Write-Host "Downloading Parakeet TDT 0.6B v3 INT8 sherpa-onnx bundle..."

if (Get-Command huggingface-cli -ErrorAction SilentlyContinue) {
    huggingface-cli download $repoId `
      --include "encoder.int8.onnx" `
      --include "decoder.int8.onnx" `
      --include "joiner.int8.onnx" `
      --include "tokens.txt" `
      --local-dir $targetDir
}
else {
    Write-Host "huggingface-cli was not found; using direct Hugging Face downloads."
    if (!(Get-Command curl.exe -ErrorAction SilentlyContinue)) {
        throw "curl.exe is required when huggingface-cli is unavailable."
    }

    foreach ($file in $files) {
        $target = Join-Path $targetDir $file
        if ((Test-Path $target) -and ((Get-Item $target).Length -gt 0)) {
            Write-Host "  $file already exists"
            continue
        }

        Write-Host "  Downloading $file..."
        curl.exe -L --fail --retry 3 --output $target "$baseUrl/$file"
        if ($LASTEXITCODE -ne 0) {
            if (Test-Path $target) {
                Remove-Item -LiteralPath $target -Force
            }
            throw "Failed to download $file"
        }
    }
}

Write-Host ""
Write-Host "Checking files..."
foreach ($file in $files) {
    $target = Join-Path $targetDir $file
    if (!(Test-Path $target)) {
        throw "Missing $file"
    }
    $sizeMb = [math]::Round((Get-Item $target).Length / 1MB, 2)
    Write-Host "  $file ($sizeMb MB)"
}

Write-Host ""
Write-Host "Parakeet bundle target: $targetDir"
Write-Host "Verify:"
Write-Host "  powershell -ExecutionPolicy Bypass -File scripts/flow-voice-status.ps1"
