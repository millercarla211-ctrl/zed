#!/usr/bin/env pwsh
# Download GLM-OCR Q4_K_M GGUF model from HuggingFace

$ErrorActionPreference = "Stop"

$MODEL_REPO = "mradermacher/GLM-OCR-GGUF"
$MODEL_FILE = "GLM-OCR.Q4_K_M.gguf"
$MMPROJ_FILE = "GLM-OCR.mmproj-Q8_0.gguf"
$OUTPUT_DIR = "models/ocr"

Write-Host "Downloading GLM-OCR Q4_K_M model..." -ForegroundColor Cyan
Write-Host "Repository: $MODEL_REPO" -ForegroundColor Gray
Write-Host "Files:" -ForegroundColor Gray
Write-Host "  - $MODEL_FILE (549 MB)" -ForegroundColor Gray
Write-Host "  - $MMPROJ_FILE (484 MB)" -ForegroundColor Gray

# Create output directory if it doesn't exist
if (-not (Test-Path $OUTPUT_DIR)) {
    New-Item -ItemType Directory -Path $OUTPUT_DIR -Force | Out-Null
}

# Check if huggingface_hub is installed
try {
    python -c "import huggingface_hub" 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "huggingface_hub not found. Installing..." -ForegroundColor Yellow
        pip install -U "huggingface_hub"
    }
} catch {
    Write-Host "huggingface_hub not found. Installing..." -ForegroundColor Yellow
    pip install -U "huggingface_hub"
}

# Download the main model
Write-Host "`nDownloading main model..." -ForegroundColor Cyan
python -c "from huggingface_hub import hf_hub_download; hf_hub_download(repo_id='$MODEL_REPO', filename='$MODEL_FILE', local_dir='$OUTPUT_DIR', local_dir_use_symlinks=False)"

if ($LASTEXITCODE -eq 0) {
    Write-Host "Main model downloaded successfully!" -ForegroundColor Green
} else {
    Write-Host "Main model download failed!" -ForegroundColor Red
    exit 1
}

# Download the mmproj file
Write-Host "`nDownloading mmproj file..." -ForegroundColor Cyan
python -c "from huggingface_hub import hf_hub_download; hf_hub_download(repo_id='$MODEL_REPO', filename='$MMPROJ_FILE', local_dir='$OUTPUT_DIR', local_dir_use_symlinks=False)"

if ($LASTEXITCODE -eq 0) {
    Write-Host "Mmproj file downloaded successfully!" -ForegroundColor Green
    Write-Host "`nDownload complete!" -ForegroundColor Green
    Write-Host "Model files saved to: $OUTPUT_DIR" -ForegroundColor Green
} else {
    Write-Host "Mmproj file download failed!" -ForegroundColor Red
    exit 1
}
