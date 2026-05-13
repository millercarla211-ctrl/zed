param(
    [ValidateSet("dx", "friday", "hello", "aladdin", "arise")]
    [string]$Command = "dx"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$configPath = Join-Path $repoRoot "configs\wakewords\$Command.yaml"
$targetPath = Join-Path $repoRoot "models\wake_words\$Command.onnx"

if (!(Test-Path -LiteralPath $configPath)) {
    throw "Wake-word config was not found: $configPath"
}

Write-Host "Flow wake-word training handoff"
Write-Host "==============================="
Write-Host ""
Write-Host "Command:       $Command"
Write-Host "Config:        $configPath"
Write-Host "Runtime target:$targetPath"
Write-Host ""
Write-Host "Train in Colab, Linux, or WSL:"
Write-Host "  cd vendor/livekit-wakeword"
Write-Host "  uv sync --all-extras"
Write-Host "  uv run livekit-wakeword setup --config ../../configs/wakewords/$Command.yaml"
Write-Host "  uv run livekit-wakeword run ../../configs/wakewords/$Command.yaml"
Write-Host ""
Write-Host "Then copy the exported ONNX file to:"
Write-Host "  $targetPath"
Write-Host ""
Write-Host "Do not commit generated audio, checkpoints, features, or ONNX artifacts."
Write-Host ""
Write-Host "Example from this Windows shell:"
Write-Host "  powershell -ExecutionPolicy Bypass -File scripts/wakeword-training-handoff.ps1 -Command $Command"
