#!/usr/bin/env pwsh
# Build a portable Windows package: the three binaries (and any staged optional
# runtime DLLs) zipped up. This is a portable distribution, not an installer;
# a signed MSI/NSIS installer is future work.
#
# Prerequisites (same as a normal Windows build):
#   - Strawberry Perl on PATH (builds vendored OpenSSL), e.g.
#       $env:Path = "$HOME\scoop\apps\perl\current\perl\bin;$env:Path"
#   - Visual Studio / MSVC build tools (cl.exe, link.exe, nmake).
#
# Usage:
#   pwsh scripts/build-windows.ps1            # release build + zip
#   pwsh scripts/build-windows.ps1 -Debug     # package an existing debug build

param(
    [switch]$Debug
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $profileDir = if ($Debug) { "debug" } else { "release" }

    if (-not $Debug) {
        Write-Host "Building release binaries (this takes a while: vendored OpenSSL/SQLite + LTO)..."
        cargo build --release --locked -p kaku -p kaku-gui
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    }

    $rel = Join-Path $root "target\$profileDir"
    $stageName = "kaku-windows-x64"
    $stage = Join-Path $root "dist\$stageName"
    if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $stage | Out-Null

    foreach ($exe in @("kaku-gui.exe", "kaku.exe", "k.exe")) {
        $src = Join-Path $rel $exe
        if (-not (Test-Path $src)) { throw "missing $src - build first" }
        Copy-Item $src (Join-Path $stage $exe) -Force
    }

    # Optional vendored runtime DLLs (ConPTY/ANGLE/Mesa) if they were staged next
    # to the exe by kaku-gui/build.rs. Not required for the default DX12 renderer.
    foreach ($dll in @("conpty.dll", "OpenConsole.exe", "libEGL.dll", "libGLESv2.dll")) {
        $p = Join-Path $rel $dll
        if (Test-Path $p) { Copy-Item $p (Join-Path $stage $dll) -Force }
    }

    $zip = Join-Path $root "dist\$stageName.zip"
    if (Test-Path $zip) { Remove-Item $zip -Force }
    Compress-Archive -Path "$stage\*" -DestinationPath $zip

    $sizeMb = [int]((Get-Item $zip).Length / 1MB)
    Write-Host "Packaged ($profileDir): $zip ($sizeMb MB)"
} finally {
    Pop-Location
}
