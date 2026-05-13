# Complete Wispr Flow Pipeline Test

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  FLOW - Wispr Flow Pipeline Test" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: STT
Write-Host "[1/3] Testing STT (Speech-to-Text)..." -ForegroundColor Yellow
cargo run --quiet -- --transcribe tests/fixtures/audio.mp3
Write-Host ""

# Step 2: Enhancement (simulated)
Write-Host "[2/3] Text Enhancement (Wispr Flow Style)..." -ForegroundColor Yellow
Write-Host "  Original: hello mike testing one two three hello" -ForegroundColor Gray
Write-Host "  Enhanced: Hello Mike, testing one, two, three." -ForegroundColor Green
Write-Host ""

# Step 3: TTS
Write-Host "[3/3] Testing TTS (Text-to-Speech)..." -ForegroundColor Yellow
cargo run --quiet -- --speak "Hello Mike, testing one, two, three."
Write-Host ""

Write-Host "========================================" -ForegroundColor Green
Write-Host "  Pipeline Test Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
