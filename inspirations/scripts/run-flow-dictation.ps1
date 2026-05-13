param(
    [string]$Device = "",
    [switch]$Devices,
    [switch]$Meter,
    [switch]$Capture,
    [switch]$CaptureStt,
    [switch]$SaveWav,
    [double]$CaptureSeconds = 5,
    [string]$CapturePath = "recording_forced.wav"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $repoRoot "target\debug\flow-dictate.exe"
$source = Join-Path $repoRoot "src\bin\flow-dictate.rs"
$parakeetEncoder = Join-Path $repoRoot "models\stt\parakeet-tdt-0.6b-v3-int8\encoder.int8.onnx"

Set-Location $repoRoot

$needsBuild = !(Test-Path $exe)
if (!$needsBuild -and (Test-Path $source)) {
    $needsBuild = (Get-Item $source).LastWriteTimeUtc -gt (Get-Item $exe).LastWriteTimeUtc
}

if ($needsBuild) {
    Write-Host "[build] building Parakeet dictation binary..."
    if (Test-Path $parakeetEncoder) {
        cargo build --features sherpa-stt --bin flow-dictate
    }
    else {
        throw "Parakeet STT files are missing. Run scripts/download_sherpa_parakeet_stt.ps1 first."
    }
}

Write-Host "Flow Dictation"
Write-Host "=============="
Write-Host "Keep the target text box focused after this window opens."
Write-Host "STT preloads during startup so the first transcription is ready."
Write-Host "Fast mode is default: short end-of-speech timeout and no WAV saving."
Write-Host "With wake ONNX files missing, quiet speech activity starts recording automatically."
Write-Host "Use -Meter for live mic levels, -Capture for raw forced recording, -CaptureStt to record/transcribe/paste, or -Devices to list inputs."
Write-Host "When wake ONNX files exist, say: dx, friday, hello, aladdin, or arise."
if ($Device) {
    $env:FLOW_INPUT_DEVICE = $Device
    Write-Host "Requested input device: $Device"
}
if ($SaveWav) {
    $env:FLOW_SAVE_WAV = "1"
    Write-Host "Debug WAV saving enabled."
}
Write-Host ""

$args = @()
if ($Devices) {
    $args += "--devices"
}
elseif ($Meter) {
    $args += "--meter"
}
elseif ($CaptureStt) {
    $args += "--capture-stt"
    $args += ([string]$CaptureSeconds)
    $args += $CapturePath
}
elseif ($Capture) {
    $args += "--capture"
    $args += ([string]$CaptureSeconds)
    $args += $CapturePath
}

& $exe @args
