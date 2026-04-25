param(
  [string]$ArtifactDir = "artifacts",
  [string]$BuiltExePath = "src-tauri/target/release/aerotrans.exe",
  [string]$FrontendDistPath = "dist/index.html"
)

$ErrorActionPreference = "Stop"
$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = (Resolve-Path (Join-Path $scriptRoot "..")).Path

Push-Location $projectRoot
try {

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

  $artifactExePath = Join-Path $ArtifactDir "AeroTrans.exe"
  if (-not (Test-Path $BuiltExePath)) {
    Write-Host "PORTABLE_CHECK: FAIL"
    Write-Host "Missing build output: $BuiltExePath"
    exit 1
  }

  $artifactExe = Get-Item $artifactExePath
  $builtExe = Get-Item $BuiltExePath
  if ($artifactExe.LastWriteTime -lt $builtExe.LastWriteTime) {
    Write-Host "PORTABLE_CHECK: FAIL"
    Write-Host "Artifact exe is stale:"
    Write-Host "  artifact: $($artifactExe.LastWriteTime.ToString('s'))"
    Write-Host "  built:    $($builtExe.LastWriteTime.ToString('s'))"
    exit 1
  }

  if (Test-Path $FrontendDistPath) {
    $frontendDist = Get-Item $FrontendDistPath
    if ($artifactExe.LastWriteTime -lt $frontendDist.LastWriteTime) {
      Write-Host "PORTABLE_CHECK: FAIL"
      Write-Host "Artifact exe predates frontend dist:"
      Write-Host "  artifact: $($artifactExe.LastWriteTime.ToString('s'))"
      Write-Host "  dist:     $($frontendDist.LastWriteTime.ToString('s'))"
      exit 1
    }
  }

  Write-Host "PORTABLE_CHECK: PASS"
  exit 0
}
finally {
  Pop-Location
}
