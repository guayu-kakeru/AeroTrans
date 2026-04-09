param(
  [switch]$Fix
)

$ErrorActionPreference = "Stop"

$base = "D:\environment"
$components = @("nodejs", "pnpm", "rustup", "tauri-cli")

foreach ($name in $components) {
  $path = Join-Path $base $name
  if (-not (Test-Path $path)) {
    New-Item -ItemType Directory -Force -Path $path | Out-Null
  }
}

function Test-Cmd($cmd) {
  $null = Get-Command $cmd -ErrorAction SilentlyContinue
  return $null -ne $null
}

function Test-Executable($path) {
  return Test-Path $path
}

$checks = @(
  @{ Name = "node"; Ok = (Test-Cmd "node") -or (Test-Executable "D:\environment\nodejs\node.exe") },
  @{ Name = "pnpm"; Ok = (Test-Cmd "pnpm") -or (Test-Executable "D:\environment\pnpm\pnpm.cmd") },
  @{ Name = "cargo"; Ok = (Test-Cmd "cargo") -or (Test-Executable "D:\environment\rustup\cargo\bin\cargo.exe") },
  @{ Name = "rustup"; Ok = (Test-Cmd "rustup") -or (Test-Executable "D:\environment\rustup\cargo\bin\rustup.exe") },
  @{ Name = "tauri"; Ok = (Test-Cmd "tauri") -or (Test-Executable "D:\environment\tauri-cli\tauri.cmd") }
)

$failed = $checks | Where-Object { -not $_.Ok }

if ($failed.Count -eq 0) {
  Write-Host "ENV CHECK: PASS"
  exit 0
}

Write-Host "ENV CHECK: FAIL"
$failed | ForEach-Object { Write-Host ("Missing command: " + $_.Name) }

if (-not $Fix) {
  Write-Host "Run with -Fix after placing installers under D:\environment\<component>."
  exit 1
}

Write-Host "-Fix mode enabled. Install missing tools using files in D:\environment."
exit 1
