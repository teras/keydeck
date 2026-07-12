# Stage the keydeck daemon as a Tauri sidecar (externalBin) so that
# `cargo tauri build` bundles it next to the config UI on Windows. Tauri strips
# the target-triple suffix on install, so keydeck.exe lands beside
# keydeck-config.exe where find_keydeck_binary() locates it.
#
# Run before `cargo tauri build`:  .\stage-sidecar.ps1
$ErrorActionPreference = 'Stop'
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path   # keydeck-config\
$root = Split-Path -Parent $scriptDir                          # repo root
$triple = ((rustc -vV | Select-String '^host: ') -replace 'host: ', '').Trim()
$binDir = Join-Path $scriptDir 'src-tauri\binaries'

Write-Host "Building keydeck daemon (release) for $triple ..."
Push-Location $root
cargo build --release --bin keydeck
Pop-Location
New-Item -ItemType Directory -Force -Path $binDir | Out-Null
Copy-Item (Join-Path $root 'target\release\keydeck.exe') (Join-Path $binDir "keydeck-$triple.exe") -Force
Write-Host "Staged sidecar: $binDir\keydeck-$triple.exe"
