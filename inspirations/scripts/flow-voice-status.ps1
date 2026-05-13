$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Test-AllPaths {
    param([string[]]$Paths)
    foreach ($path in $Paths) {
        if (!(Test-Path -LiteralPath (Join-Path $repoRoot $path))) {
            return $false
        }
    }
    return $true
}

function Write-Check {
    param(
        [string]$Name,
        [bool]$Ready,
        [string]$Detail
    )
    $label = if ($Ready) { "[ready]" } else { "[missing]" }
    $color = if ($Ready) { "Green" } else { "Yellow" }
    Write-Host ("{0} {1}" -f $label, $Name) -ForegroundColor $color
    if ($Detail) {
        Write-Host ("        {0}" -f $Detail)
    }
}

$wakeCommands = @("dx", "friday", "hello", "aladdin", "arise")
$wakeReady = 0

Write-Host "Flow Voice Stack Status"
Write-Host "======================="
Write-Host ""

Write-Host "Wake commands"
Write-Host "-------------"
foreach ($command in $wakeCommands) {
    $model = "models/wake_words/$command.onnx"
    $config = "configs/wakewords/$command.yaml"
    $ready = Test-AllPaths @($model, $config)
    if ($ready) { $wakeReady++ }
    Write-Check $command $ready "$model"
}

Write-Host ""
Write-Host "Wake runtime"
Write-Host "------------"
Write-Check "LiveKit feature frontend resources" (Test-AllPaths @(
    "vendor/livekit-wakeword/src/livekit/wakeword/resources/melspectrogram.onnx",
    "vendor/livekit-wakeword/src/livekit/wakeword/resources/embedding_model.onnx"
)) "vendored ONNX frontend"
Write-Check "Training handoff" (Test-AllPaths @(
    "scripts/wakeword-training-handoff.ps1",
    "docs/WAKEWORD_TRAINING.md"
)) "Colab/Linux/WSL training path"

Write-Host ""
Write-Host "Local STT"
Write-Host "---------"
$moonshine = Test-AllPaths @(
    "models/stt/encoder_model.onnx",
    "models/stt/decoder_model_merged.onnx",
    "models/stt/tokenizer.json"
)
$parakeet = Test-AllPaths @(
    "models/stt/parakeet-tdt-0.6b-v3-int8/encoder.int8.onnx",
    "models/stt/parakeet-tdt-0.6b-v3-int8/decoder.int8.onnx",
    "models/stt/parakeet-tdt-0.6b-v3-int8/joiner.int8.onnx",
    "models/stt/parakeet-tdt-0.6b-v3-int8/tokens.txt"
)
$nemotron = Test-AllPaths @(
    "models/stt/nemotron-speech-streaming-en-0.6b-int8/encoder.int8.onnx",
    "models/stt/nemotron-speech-streaming-en-0.6b-int8/decoder.int8.onnx",
    "models/stt/nemotron-speech-streaming-en-0.6b-int8/joiner.int8.onnx",
    "models/stt/nemotron-speech-streaming-en-0.6b-int8/tokens.txt"
)
Write-Check "Moonshine tiny fallback" $moonshine "models/stt/"
Write-Check "Parakeet TDT 0.6B v3 INT8" $parakeet "models/stt/parakeet-tdt-0.6b-v3-int8/"
Write-Check "Nemotron Speech Streaming EN 0.6B INT8" $nemotron "models/stt/nemotron-speech-streaming-en-0.6b-int8/"

Write-Host ""
Write-Host "Runtime policy"
Write-Host "--------------"
$cargoToml = Get-Content -Raw (Join-Path $repoRoot "Cargo.toml")
Write-Check "Sherpa STT optional feature" ($cargoToml -match 'sherpa-stt') "normal Windows builds stay Moonshine-safe"
Write-Check "Local-only production default" (Select-String -Path "configs/production/*.json" -Pattern '"local_only_default": true' -Quiet) "remote audio remains opt-in fallback"

$repoScore = 0
$repoScore += 20 # canonical wake command wiring
$repoScore += 15 # training templates and handoff
$repoScore += 20 # lazy local runtime/STT abstraction
$repoScore += 15 # Parakeet/Nemotron catalog and feature gating
$repoScore += 10 # remote fallback policy
$artifactScore = 0
$artifactScore += [int](($wakeReady / $wakeCommands.Count) * 10)
if ($moonshine) { $artifactScore += 4 }
if ($parakeet) { $artifactScore += 4 }
if ($nemotron) { $artifactScore += 2 }
$score = [Math]::Min(100, $repoScore + $artifactScore)

Write-Host ""
Write-Host "Progress"
Write-Host "--------"
Write-Host ("Repo implementation: {0}/80" -f $repoScore)
Write-Host ("Local artifacts:      {0}/20" -f $artifactScore)
Write-Host ("Current score:        {0}/100" -f $score)

Write-Host ""
Write-Host "Next unblockers"
Write-Host "---------------"
if ($wakeReady -lt $wakeCommands.Count) {
    Write-Host "1. Train wake ONNX files in Colab/Linux/WSL, then place them in models/wake_words/."
    Write-Host "   powershell -ExecutionPolicy Bypass -File scripts/wakeword-training-handoff.ps1 -Command dx"
}
if (!$parakeet) {
    Write-Host "2. Install Parakeet sherpa bundle for the best practical local STT upgrade."
    Write-Host "   powershell -ExecutionPolicy Bypass -File scripts/download_sherpa_parakeet_stt.ps1"
}
if ($wakeReady -eq $wakeCommands.Count -and ($moonshine -or $parakeet -or $nemotron)) {
    Write-Host "3. Run a live microphone pass: cargo run -- --live"
}
