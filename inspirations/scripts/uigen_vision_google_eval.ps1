param(
    [string]$OutputDir = "tmp/uigen-vision-google/gemma4-e4b-frontend-q4km",
    [string]$GoogleUrl = "https://www.google.com/?hl=en&gl=US&pws=0",
    [switch]$ForceGenerate
)

$ErrorActionPreference = "Stop"

function Resolve-Browser {
    $candidates = @(
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles(x86)\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
        "$env:ProgramFiles(x86)\Google\Chrome\Application\chrome.exe"
    )

    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            return $candidate
        }
    }

    throw "No headless Edge or Chrome executable was found."
}

function Capture-Screenshot {
    param(
        [string]$Browser,
        [string]$Url,
        [string]$ScreenshotPath,
        [string]$WindowSize
    )

    $fullPath = [System.IO.Path]::GetFullPath($ScreenshotPath)
    $args = @(
        "--headless=new",
        "--disable-gpu",
        "--hide-scrollbars",
        "--window-size=$WindowSize",
        "--screenshot=$fullPath",
        $Url
    )

    $process = Start-Process -FilePath $Browser -ArgumentList $args -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0) {
        throw "Browser screenshot failed for $Url with exit code $($process.ExitCode)."
    }
}

function Get-VisualDifference {
    param(
        [string]$A,
        [string]$B
    )

    Add-Type -AssemblyName System.Drawing
    $imgA = [System.Drawing.Bitmap]::new([System.IO.Path]::GetFullPath($A))
    $imgB = [System.Drawing.Bitmap]::new([System.IO.Path]::GetFullPath($B))

    try {
        $width = [Math]::Min($imgA.Width, $imgB.Width)
        $height = [Math]::Min($imgA.Height, $imgB.Height)
        $step = 12
        $total = 0.0
        $count = 0

        for ($y = 0; $y -lt $height; $y += $step) {
            for ($x = 0; $x -lt $width; $x += $step) {
                $pa = $imgA.GetPixel($x, $y)
                $pb = $imgB.GetPixel($x, $y)
                $total += [Math]::Abs($pa.R - $pb.R)
                $total += [Math]::Abs($pa.G - $pb.G)
                $total += [Math]::Abs($pa.B - $pb.B)
                $count += 3
            }
        }

        if ($count -eq 0) {
            return 100.0
        }

        return [Math]::Round(($total / ($count * 255.0)) * 100.0, 2)
    }
    finally {
        $imgA.Dispose()
        $imgB.Dispose()
    }
}

$root = (Resolve-Path ".").Path
$googleDesktop = Join-Path $OutputDir "google-desktop.png"
$googleMobile = Join-Path $OutputDir "google-mobile.png"
$outputPath = Join-Path $OutputDir "index.html"
$localDesktop = Join-Path $OutputDir "vision-desktop.png"
$localMobile = Join-Path $OutputDir "vision-mobile.png"
$reportPath = Join-Path $OutputDir "eval-report.md"
$rawPath = Join-Path $OutputDir "index.raw.txt"
$partialPath = Join-Path $OutputDir "index.partial.html"
$flowExe = Join-Path $root "target\release\flow.exe"
$prompt = "Use the screenshot as the source of truth and recreate the visible search homepage as a complete standalone HTML/CSS file for evaluation only. Return only HTML code. Do not use external scripts, external fonts, CDNs, images, or brand assets. Use semantic HTML and inline CSS in one style tag. Keep CSS compact and do not repeat selectors. Keep the whole file under 180 lines. Match the visible layout: top navigation, centered logo/search composition, search controls, bottom footer, spacing, and mobile responsiveness. Include body and html closing tags."

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$browser = Resolve-Browser

Write-Host "[1/5] Capturing latest Google homepage screenshots..."
Capture-Screenshot -Browser $browser -Url $GoogleUrl -ScreenshotPath $googleDesktop -WindowSize "1365,768"
Capture-Screenshot -Browser $browser -Url $GoogleUrl -ScreenshotPath $googleMobile -WindowSize "390,844"

Write-Host "[2/5] Generating local vision UI clone..."
$generationExitCode = 0
if ((Test-Path $outputPath) -and !$ForceGenerate) {
    Write-Host "Using existing $outputPath. Pass -ForceGenerate to regenerate."
}
else {
    if (Test-Path $flowExe) {
        & $flowExe --uigen-vision $googleDesktop $outputPath $prompt
        $generationExitCode = $LASTEXITCODE
    }
    else {
        cargo run --release -- --uigen-vision $googleDesktop $outputPath $prompt
        $generationExitCode = $LASTEXITCODE
    }
}

if (($generationExitCode -ne 0) -or !(Test-Path $outputPath)) {
    $reason = if ($generationExitCode -ne 0) {
        "model generation exited with code $generationExitCode before producing complete HTML"
    }
    else {
        "expected generated file missing"
    }

    @"
# Gemma Frontend Vision Google Homepage Eval

Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss zzz")
Model: gemma4-e4b-frontend-q4km
Reference URL: $GoogleUrl
Input screenshot: $googleDesktop
Local file: $outputPath

## Result

Fail: $reason.

The model was not scored visually because the complete HTML gate failed. Do not claim this model cloned the reference UI until it returns a valid standalone document and browser screenshots can be compared.

## Artifacts

- Google desktop: $googleDesktop
- Google mobile: $googleMobile
- Raw model output: $rawPath
- Partial HTML: $partialPath

## Manual Checklist

- Complete HTML document: failed
- Centered wordmark and search composition: not scored
- Top-right navigation/account area: not scored
- Search input proportions: not scored
- Buttons and spacing: not scored
- Footer layout: not scored
- Mobile responsiveness: not scored
"@ | Set-Content -Path $reportPath -Encoding UTF8

    Write-Error "Generation failed complete-HTML gate. Report: $reportPath"
    exit 1
}

$localUrl = "file:///" + ([System.IO.Path]::GetFullPath($outputPath) -replace "\\", "/")

Write-Host "[3/5] Capturing local vision screenshots..."
Capture-Screenshot -Browser $browser -Url $localUrl -ScreenshotPath $localDesktop -WindowSize "1365,768"
Capture-Screenshot -Browser $browser -Url $localUrl -ScreenshotPath $localMobile -WindowSize "390,844"

Write-Host "[4/5] Calculating rough visual diff..."
$desktopDiff = Get-VisualDifference -A $googleDesktop -B $localDesktop
$mobileDiff = Get-VisualDifference -A $googleMobile -B $localMobile
$averageDiff = [Math]::Round(($desktopDiff + $mobileDiff) / 2.0, 2)
$verdict = if ($averageDiff -lt 25) {
    "Likely pass: low rough pixel difference. Still inspect screenshots manually."
} elseif ($averageDiff -lt 55) {
    "Partial: rough structure may be close, but manual review is required."
} else {
    "Fail: rough pixel difference is high. Do not claim the model cloned the reference UI."
}

Write-Host "[5/5] Writing report..."
@"
# Gemma Frontend Vision Google Homepage Eval

Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss zzz")
Model: gemma4-e4b-frontend-q4km
Reference URL: $GoogleUrl
Input screenshot: $googleDesktop
Local file: $outputPath

## Screenshots

- Google desktop: $googleDesktop
- Vision desktop: $localDesktop
- Google mobile: $googleMobile
- Vision mobile: $localMobile

## Rough Pixel Difference

- Desktop sampled difference: $desktopDiff percent
- Mobile sampled difference: $mobileDiff percent
- Average sampled difference: $averageDiff percent

## Manual Checklist

- Centered wordmark and search composition:
- Top-right navigation/account area:
- Search input proportions:
- Buttons and spacing:
- Footer layout:
- Mobile responsiveness:

Result: $verdict
"@ | Set-Content -Path $reportPath -Encoding UTF8

Write-Host "Done. Report: $reportPath"
