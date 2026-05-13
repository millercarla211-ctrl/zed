#!/usr/bin/env pwsh
# Download Gemma 4 2B Q4_K_M GGUF model from HuggingFace

$ErrorActionPreference = "Stop"

$MODEL_REPO = "bartowski/google_gemma-4-E2B-it-GGUF"
$MODEL_FILE = "google_gemma-4-E2B-it-Q4_K_M.gguf"
$OUTPUT_DIR = "models/llm"
$OUTPUT_FILE = "$OUTPUT_DIR/google_gemma-4-E2B-it-Q4_K_M.gguf"

Write-Host "Downloading Gemma 4 2B Q4_K_M model..." -ForegroundColor Cyan
Write-Host "Repository: $MODEL_REPO" -ForegroundColor Gray
Write-Host "File: $MODEL_FILE (3.46 GB)" -ForegroundColor Gray

# Create output directory if it doesn't exist
if (-not (Test-Path $OUTPUT_DIR)) {
    New-Item -ItemType Directory -Path $OUTPUT_DIR -Force | Out-Null
}

# Check if file already exists
if (Test-Path $OUTPUT_FILE) {
    Write-Host "Model already exists at $OUTPUT_FILE" -ForegroundColor Yellow
    $response = Read-Host "Do you want to re-download? (y/N)"
    if ($response -ne "y" -and $response -ne "Y") {
        Write-Host "Skipping download." -ForegroundColor Green
        exit 0
    }
}

# Check if huggingface_hub is installed
try {
    python -c "import huggingface_hub" 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "huggingface_hub not found. Installing..." -ForegroundColor Yellow
        pip install -U "huggingface_hub[cli]"
    }
} catch {
    Write-Host "huggingface_hub not found. Installing..." -ForegroundColor Yellow
    pip install -U "huggingface_hub[cli]"
}

# Download the model using Python
Write-Host "Starting download..." -ForegroundColor Cyan
python -c "from huggingface_hub import hf_hub_download; hf_hub_download(repo_id='$MODEL_REPO', filename='$MODEL_FILE', local_dir='$OUTPUT_DIR', local_dir_use_symlinks=False)"

if ($LASTEXITCODE -eq 0) {
    Write-Host "Download complete!" -ForegroundColor Green
    Write-Host "Model saved to: $OUTPUT_FILE" -ForegroundColor Green
} else {
    Write-Host "Download failed!" -ForegroundColor Red
    exit 1
}
