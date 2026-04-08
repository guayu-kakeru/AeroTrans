param(
  [string]$ArtifactDir = "artifacts"
)

$ErrorActionPreference = "Stop"

$required = @(
  "AeroTrans.exe",
  "WebView2Loader.dll"
)

$missing = @()
foreach ($name in $required) {
  $path = Join-Path $ArtifactDir $name
  if (-not (Test-Path $path)) {
    $missing += $name
  }
}

if ($missing.Count -gt 0) {
  Write-Host "PORTABLE_CHECK: FAIL"
  $missing | ForEach-Object { Write-Host "Missing: $_" }
  exit 1
}

Write-Host "PORTABLE_CHECK: PASS"
exit 0
