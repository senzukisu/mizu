# Builds a Mizu release: optimized browser, installer with the browser inside, portable zip,
# and the signed update manifest.
#
#   pwsh scripts/release.ps1 [-Key "$env:USERPROFILE\.onibi\mizu-release.key"] [-Notes "…"]
#
# Output goes to dist/. The signing key must never be committed.
param(
    [string]$Key = "$env:USERPROFILE\.onibi\mizu-release.key",
    [string]$Notes = ""
)
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
Write-Host "Mizu $version"

cargo build --release -p mizu-browser
if ($LASTEXITCODE) { throw "browser build failed" }
cargo build --release -p mizu-update --bin mizu-sign
if ($LASTEXITCODE) { throw "signer build failed" }

$env:MIZU_SETUP_PAYLOAD = (Resolve-Path target\release\mizu.exe).Path
cargo build --release -p mizu-setup
if ($LASTEXITCODE) { throw "installer build failed" }
Remove-Item Env:\MIZU_SETUP_PAYLOAD

New-Item -ItemType Directory -Force dist | Out-Null
Remove-Item dist\* -Force -ErrorAction SilentlyContinue
$setup = "dist\MizuSetup-$version.exe"
Copy-Item target\release\MizuSetup.exe $setup

$portable = "dist\portable"
New-Item -ItemType Directory -Force $portable | Out-Null
Copy-Item target\release\mizu.exe, LICENSE, README.md $portable
Compress-Archive -Path "$portable\*" -DestinationPath "dist\Mizu-$version-portable-x64.zip" -Force
Remove-Item $portable -Recurse -Force

if (Test-Path $Key) {
    & target\release\mizu-sign.exe manifest $Key $setup $version dist $Notes
    if ($LASTEXITCODE) { throw "signing failed" }
} else {
    Write-Warning "Signing key not found at $Key - no update manifest produced."
}

Get-ChildItem dist | ForEach-Object {
    $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
    "{0,-40} {1,12:N0} bytes  {2}" -f $_.Name, $_.Length, $hash
}
