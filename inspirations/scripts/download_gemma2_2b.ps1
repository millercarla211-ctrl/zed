# Download Gemma 2 2B Q4_K_M model (compatible with llama-cpp-2 v0.1.141)
# Gemma 2 is well-supported, unlike Gemma 4 E2B which requires newer llama.cpp

$modelDir = "F:\flow\models\llm"
$modelUrl = "https://huggingface.co/bartowski/gemma-2-2b-it-GGUF/resolve/main/gemma-2-2b-it-Q4_K_M.gguf"
$modelPath = "$modelDir\gemma-2-2b-it-Q4_K_M.gguf"

Write-Host "Downloading Gemma 2 2B Q4_K_M model..." -ForegroundColor Cyan
Write-Host "This model is compatible with llama-cpp-2 v0.1.141" -ForegroundColor Green
Write-Host "Size: ~1.5GB" -ForegroundColor Yellow
Write-Host ""

# Create directory if it doesn't exist
if (!(Test-Path $modelDir)) {
    New-Item -ItemType Directory -Path $modelDir -Force | Out-Null
}

# Download with progress
$ProgressPreference = 'Continue'
try {
    Invoke-WebRequest -Uri $modelUrl -OutFile $modelPath -UseBasicParsing
    Write-Host "Download complete!" -ForegroundColor Green
    Write-Host "Model saved to: $modelPath" -ForegroundColor Cyan
} catch {
    Write-Host "Download failed: $_" -ForegroundColor Red
    exit 1
}
