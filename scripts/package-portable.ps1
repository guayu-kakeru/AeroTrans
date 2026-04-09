param(
  [string]$SourceDir = "src-tauri/target/release",
  [string]$LocalArtifactDir = "artifacts",
  [string]$RootArtifactDir = "D:/software/AeroTrans/artifacts"
)

$ErrorActionPreference = "Stop"

$required = @(
  "aerotrans.exe",
  "WebView2Loader.dll"
)

foreach ($name in $required) {
  $sourcePath = Join-Path $SourceDir $name
  if (-not (Test-Path $sourcePath)) {
    throw "Missing build output: $sourcePath"
  }
}

New-Item -ItemType Directory -Force -Path $LocalArtifactDir | Out-Null
New-Item -ItemType Directory -Force -Path $RootArtifactDir | Out-Null

Copy-Item -Force (Join-Path $SourceDir "aerotrans.exe") (Join-Path $LocalArtifactDir "AeroTrans.exe")
Copy-Item -Force (Join-Path $SourceDir "WebView2Loader.dll") (Join-Path $LocalArtifactDir "WebView2Loader.dll")

Copy-Item -Force (Join-Path $LocalArtifactDir "AeroTrans.exe") (Join-Path $RootArtifactDir "AeroTrans.exe")
Copy-Item -Force (Join-Path $LocalArtifactDir "WebView2Loader.dll") (Join-Path $RootArtifactDir "WebView2Loader.dll")

Write-Host "PORTABLE_PACKAGE: DONE"
