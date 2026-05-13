param(
    [Parameter(Mandatory = $true)]
    [string]$Url,
    [Parameter(Mandatory = $true)]
    [string]$Output,
    [Parameter(Mandatory = $true)]
    [Int64]$ExpectedBytes,
    [string]$LogPath = "",
    [int]$MaxAttempts = 40
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $LogPath = "$Output.download.log"
}

$parent = Split-Path -Parent $Output
if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$logParent = Split-Path -Parent $LogPath
if ($logParent) {
    New-Item -ItemType Directory -Force -Path $logParent | Out-Null
}

for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
    $currentBytes = if (Test-Path $Output) { (Get-Item $Output).Length } else { 0 }
    if ($currentBytes -ge $ExpectedBytes) {
        "done bytes=$currentBytes expected=$ExpectedBytes" | Add-Content -Path $LogPath
        exit 0
    }

    "attempt=$attempt bytes=$currentBytes expected=$ExpectedBytes time=$(Get-Date -Format o)" | Add-Content -Path $LogPath
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & curl.exe `
        -L `
        --fail `
        --retry 3 `
        --retry-delay 5 `
        --connect-timeout 30 `
        --speed-time 180 `
        --speed-limit 1024 `
        --continue-at - `
        --output $Output `
        $Url 2>&1 | ForEach-Object { $_.ToString() } | Add-Content -Path $LogPath

    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference
    $currentBytes = if (Test-Path $Output) { (Get-Item $Output).Length } else { 0 }
    "curl_exit=$exitCode bytes=$currentBytes time=$(Get-Date -Format o)" | Add-Content -Path $LogPath

    if ($currentBytes -ge $ExpectedBytes) {
        "done bytes=$currentBytes expected=$ExpectedBytes" | Add-Content -Path $LogPath
        exit 0
    }

    Start-Sleep -Seconds 5
}

throw "Download did not reach expected size after $MaxAttempts attempts: $Output"
