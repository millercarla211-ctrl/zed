Param(
    [switch]$WhatIf
)

# Determine repository root (assumes this script lives in scripts/ under repo root)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir '..') | Select-Object -ExpandProperty Path
$TargetDir = Join-Path $RepoRoot 'hexed'

if (-not (Test-Path -Path $TargetDir)) {
    New-Item -ItemType Directory -Path $TargetDir | Out-Null
}

$files = Get-ChildItem -Path $RepoRoot -Filter '*.txt' -File -ErrorAction SilentlyContinue

if ($null -eq $files -or $files.Count -eq 0) {
    Write-Output "No .txt files found in $RepoRoot"
    exit 0
}

foreach ($f in $files) {
    $dest = Join-Path $TargetDir $f.Name
    if ($WhatIf) {
        Write-Output "Would move: $($f.FullName) -> $dest"
    } else {
        Move-Item -LiteralPath $f.FullName -Destination $dest -Force
        Write-Output "Moved: $($f.Name) -> $TargetDir"
    }
}
