<#
.SYNOPSIS
    Installs Keeboy as a normal Windows app: copies the binary to your user profile
    and creates Start Menu (and optionally startup) shortcuts.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\install.ps1
    powershell -ExecutionPolicy Bypass -File scripts\install.ps1 -Startup
    powershell -ExecutionPolicy Bypass -File scripts\install.ps1 -Uninstall
#>
[CmdletBinding()]
param(
    # Also launch Keeboy automatically when you log in to Windows.
    [switch]$Startup,
    # Skip `cargo build --release` and install whatever is already built.
    [switch]$NoBuild,
    # Remove the installed copy and all shortcuts.
    [switch]$Uninstall,
    # Don't launch Keeboy after installing.
    [switch]$NoLaunch
)

$ErrorActionPreference = 'Stop'
$repoRoot   = Split-Path -Parent $PSScriptRoot
$installDir = Join-Path $env:LOCALAPPDATA 'Keeboy'
$exeName    = 'keeboy.exe'
$installedExe = Join-Path $installDir $exeName
$startMenu  = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$menuLink   = Join-Path $startMenu 'Keeboy.lnk'
$startupLink = Join-Path $startMenu 'Startup\Keeboy.lnk'

function Stop-Keeboy {
    $procs = Get-Process keeboy -ErrorAction SilentlyContinue
    if ($procs) {
        Write-Host "Stopping running Keeboy instance..."
        $procs | Stop-Process -Force
        Start-Sleep -Milliseconds 600
    }
}

function New-Shortcut([string]$linkPath, [string]$target, [string]$description) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $linkPath) | Out-Null
    $shell = New-Object -ComObject WScript.Shell
    $sc = $shell.CreateShortcut($linkPath)
    $sc.TargetPath = $target
    $sc.WorkingDirectory = Split-Path -Parent $target
    $sc.IconLocation = "$target,0"
    $sc.Description = $description
    $sc.Save()
}

if ($Uninstall) {
    Stop-Keeboy
    foreach ($link in @($menuLink, $startupLink)) {
        if (Test-Path $link) { Remove-Item $link -Force; Write-Host "Removed $link" }
    }
    if (Test-Path $installDir) { Remove-Item $installDir -Recurse -Force; Write-Host "Removed $installDir" }
    Write-Host "Keeboy uninstalled."
    return
}

if (-not $NoBuild) {
    Write-Host "Building release binary..."
    Stop-Keeboy
    Push-Location $repoRoot
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }
    } finally {
        Pop-Location
    }
}

$builtExe = Join-Path $repoRoot 'target\release\keeboy.exe'
if (-not (Test-Path $builtExe)) { throw "Missing $builtExe. Run without -NoBuild first." }

Stop-Keeboy
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item $builtExe $installedExe -Force
Write-Host "Installed $installedExe"

# Custom soundpacks are loaded relative to the working directory, so they live beside the exe.
$soundpacks = Join-Path $repoRoot 'soundpacks'
if (Test-Path $soundpacks) {
    Copy-Item $soundpacks (Join-Path $installDir 'soundpacks') -Recurse -Force
    Write-Host "Copied custom soundpacks"
}

New-Shortcut $menuLink $installedExe 'Keeboy - Mechanical Keyboard Sound Engine'
Write-Host "Start Menu shortcut created (search 'Keeboy')"

if ($Startup) {
    New-Shortcut $startupLink $installedExe 'Keeboy - start with Windows'
    Write-Host "Startup shortcut created (launches at login)"
} elseif (Test-Path $startupLink) {
    Write-Host "Existing startup shortcut left in place"
}

if (-not $NoLaunch) {
    Start-Process $installedExe
    Write-Host "Keeboy launched - look for the keycap icon in your system tray."
}
