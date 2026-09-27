<#
.SYNOPSIS
    Builds Keeboy and packages a distributable ZIP for GitHub Releases.

.DESCRIPTION
    Produces dist\Keeboy-v<version>-win-x64.zip containing the standalone binary, a
    one-click installer, and a plain-text quick start. The binary statically links the
    MSVC runtime (see .cargo\config.toml), so it runs on any Windows 10/11 x64 machine
    with no Rust toolchain and no Visual C++ Redistributable.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\package.ps1
#>
[CmdletBinding()]
param(
    # Package the already-built target\release\keeboy.exe.
    [switch]$NoBuild
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

# Version comes from Cargo.toml so the archive name never drifts from the crate.
$cargoToml = Get-Content (Join-Path $repoRoot 'Cargo.toml') -Raw
if ($cargoToml -notmatch '(?m)^version\s*=\s*"([^"]+)"') { throw 'Could not read version from Cargo.toml' }
$version = $Matches[1]

$stageName = "Keeboy-v$version-win-x64"
$distDir   = Join-Path $repoRoot 'dist'
$stageDir  = Join-Path $distDir $stageName
$zipPath   = Join-Path $distDir "$stageName.zip"

if (-not $NoBuild) {
    Write-Host 'Building release binary...'
    Get-Process keeboy -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 400
    Push-Location $repoRoot
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw 'cargo build --release failed' }
    } finally {
        Pop-Location
    }
}

$exe = Join-Path $repoRoot 'target\release\keeboy.exe'
if (-not (Test-Path $exe)) { throw "Missing $exe" }

# Fail loudly if the binary would need the VC++ Redistributable on the target machine.
$bytes = [System.IO.File]::ReadAllBytes($exe)
$ascii = [System.Text.Encoding]::ASCII.GetString($bytes)
if ($ascii -match 'VCRUNTIME140') {
    throw 'Binary links VCRUNTIME140.dll. Ensure .cargo\config.toml sets +crt-static, then rebuild.'
}

if (Test-Path $stageDir) { Remove-Item $stageDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stageDir | Out-Null

Copy-Item $exe (Join-Path $stageDir 'keeboy.exe')
Copy-Item (Join-Path $PSScriptRoot 'Install-Keeboy.ps1') $stageDir
foreach ($extra in @('README.md', 'LICENSE')) {
    $path = Join-Path $repoRoot $extra
    if (Test-Path $path) { Copy-Item $path $stageDir }
}

$quickStart = @"
Keeboy v$version - Lightweight Mechanical Keyboard Sound Engine
================================================================

Requirements: Windows 10 or 11 (64-bit). Nothing else to install.

OPTION 1 - Just run it
----------------------
Double-click keeboy.exe. No window opens; look for the keycap icon in your
system tray (notification area near the clock, possibly behind the '^' arrow).
Start typing in any app.

OPTION 2 - Install properly (Start Menu shortcut)
-------------------------------------------------
Right-click Install-Keeboy.ps1 and choose "Run with PowerShell".
Then launch Keeboy from the Start Menu.

To also start Keeboy when you log in to Windows, run this in PowerShell from
this folder:
    powershell -ExecutionPolicy Bypass -File Install-Keeboy.ps1 -Startup

To remove it:
    powershell -ExecutionPolicy Bypass -File Install-Keeboy.ps1 -Uninstall

ADJUSTING THE SOUND
-------------------
Right-click the tray icon to pick a switch profile and to set master volume,
key-release volume, and the current profile's volume independently.
To balance Keeboy against other apps, use the Windows Volume Mixer
(right-click the speaker icon -> Open Volume mixer) and move the Keeboy slider.

WINDOWS SMARTSCREEN
-------------------
The binary is not code-signed, so Windows may show "Windows protected your PC".
Click "More info" -> "Run anyway". Keeboy installs a global keyboard hook to
detect keystrokes for sound playback; it does not log, store, or transmit
anything you type.
"@
Set-Content -Path (Join-Path $stageDir 'QUICK-START.txt') -Value $quickStart -Encoding UTF8

if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path (Join-Path $stageDir '*') -DestinationPath $zipPath
Remove-Item $stageDir -Recurse -Force

$sha = (Get-FileHash $zipPath -Algorithm SHA256).Hash
Write-Host ''
Write-Host "Package: $zipPath"
Write-Host ("Size:    {0:N0} bytes" -f (Get-Item $zipPath).Length)
Write-Host "SHA256:  $sha"
Write-Host ''
Write-Host 'Upload this ZIP as a GitHub Release asset, or run:'
Write-Host "  gh release create v$version `"$zipPath`" --title `"Keeboy v$version`" --notes `"...`""
