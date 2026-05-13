param(
    [string]$ModelKey = "webgen-4b-preview-i1-q4km",
    [string]$OutputDir = "",
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

function Get-SafePathName {
    param([string]$Value)
    return ($Value -replace '[^A-Za-z0-9_-]', '-')
}

$root = (Resolve-Path ".").Path
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path "tmp/uigen-google" (Get-SafePathName $ModelKey)
}
$outputPath = Join-Path $OutputDir "index.html"
$desktopGoogle = Join-Path $OutputDir "google-desktop.png"
$desktopLocal = Join-Path $OutputDir "uigen-desktop.png"
$mobileGoogle = Join-Path $OutputDir "google-mobile.png"
$mobileLocal = Join-Path $OutputDir "uigen-mobile.png"
$reportPath = Join-Path $OutputDir "eval-report.md"
$flowExe = Join-Path $root "target\release\flow.exe"
$googlePrompt = "Make a responsive Google.com homepage look-alike for screenshot evaluation only. Do not use Google image assets, fonts, scripts, CDNs, or external assets. Use a plain text multicolor wordmark spelling Google centered in the first viewport, a 582px rounded search box with a subtle border and shadow, two light gray buttons labeled Google Search and I'm Feeling Lucky, top-right text navigation with Gmail, Images, an apps icon placeholder, and an avatar circle, and a two-row bottom footer with realistic links. Desktop layout: large whitespace, centered search composition around the vertical middle, footer pinned to bottom, top nav at the top-right. Mobile layout: same centered search composition, narrower search field, footer links wrapping cleanly. Keep it simple, accurate, complete, and screenshot-stable."

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

Write-Host "[1/4] Generating local UI model Google eval page with $ModelKey..."
if ((Test-Path $outputPath) -and !$ForceGenerate) {
    Write-Host "Using existing $outputPath. Pass -ForceGenerate to regenerate."
}
else {
    if (Test-Path $flowExe) {
        & $flowExe --uigen-model $ModelKey $outputPath $googlePrompt
    }
    else {
        cargo run --release -- --uigen-model $ModelKey $outputPath $googlePrompt
    }
}

if (!(Test-Path $outputPath)) {
    throw "Expected generated file missing: $outputPath"
}

$browser = Resolve-Browser
$localUrl = "file:///" + ([System.IO.Path]::GetFullPath($outputPath) -replace "\\", "/")

Write-Host "[2/4] Capturing latest Google homepage screenshots..."
Capture-Screenshot -Browser $browser -Url $GoogleUrl -ScreenshotPath $desktopGoogle -WindowSize "1365,768"
Capture-Screenshot -Browser $browser -Url $GoogleUrl -ScreenshotPath $mobileGoogle -WindowSize "390,844"

Write-Host "[3/4] Capturing local UIGEN screenshots..."
Capture-Screenshot -Browser $browser -Url $localUrl -ScreenshotPath $desktopLocal -WindowSize "1365,768"
Capture-Screenshot -Browser $browser -Url $localUrl -ScreenshotPath $mobileLocal -WindowSize "390,844"

Write-Host "[4/4] Writing rough visual diff report..."
$desktopDiff = Get-VisualDifference -A $desktopGoogle -B $desktopLocal
$mobileDiff = Get-VisualDifference -A $mobileGoogle -B $mobileLocal
$averageDiff = [Math]::Round(($desktopDiff + $mobileDiff) / 2.0, 2)
$verdict = if ($averageDiff -lt 25) {
    "Likely pass: low rough pixel difference. Still inspect screenshots manually."
} elseif ($averageDiff -lt 55) {
    "Partial: rough structure may be close, but manual review is required."
} else {
    "Fail: rough pixel difference is high. Do not claim the model cloned the reference UI."
}

@"
# Local UI Model Google Homepage Eval

Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss zzz")
Model: $ModelKey
Reference URL: $GoogleUrl
Local file: $outputPath

## Screenshots

- Google desktop: $desktopGoogle
- Local model desktop: $desktopLocal
- Google mobile: $mobileGoogle
- Local model mobile: $mobileLocal

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
