<#
.SYNOPSIS
    Installs Keeboy from a downloaded release folder. No Rust toolchain required.

.DESCRIPTION
    Copies keeboy.exe (sitting next to this script) into your user profile and creates
    a Start Menu shortcut, so Keeboy launches like any other Windows app.

.EXAMPLE
    Right-click this file -> "Run with PowerShell"

    Or from a terminal:
      powershell -ExecutionPolicy Bypass -File Install-Keeboy.ps1
      powershell -ExecutionPolicy Bypass -File Install-Keeboy.ps1 -Startup
      powershell -ExecutionPolicy Bypass -File Install-Keeboy.ps1 -Uninstall
#>
[CmdletBinding()]
param(
    # Also launch Keeboy automatically when you log in to Windows.
    [switch]$Startup,
    # Remove the installed copy and all shortcuts.
    [switch]$Uninstall,
    # Install without starting it.
    [switch]$NoLaunch
)

$ErrorActionPreference = 'Stop'

$installDir   = Join-Path $env:LOCALAPPDATA 'Keeboy'
$installedExe = Join-Path $installDir 'keeboy.exe'
$startMenu    = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$menuLink     = Join-Path $startMenu 'Keeboy.lnk'
$startupLink  = Join-Path $startMenu 'Startup\Keeboy.lnk'
$sourceExe    = Join-Path $PSScriptRoot 'keeboy.exe'

function Stop-Keeboy {
    $procs = Get-Process keeboy -ErrorAction SilentlyContinue
    if ($procs) {
        Write-Host 'Stopping running Keeboy instance...'
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
    Write-Host 'Keeboy uninstalled.'
    return
}

if (-not (Test-Path $sourceExe)) {
    throw "keeboy.exe was not found next to this script. Keep both files in the same folder."
}

Stop-Keeboy
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item $sourceExe $installedExe -Force
Write-Host "Installed $installedExe"

# Custom soundpacks are read from a 'soundpacks' folder beside the executable.
$soundpacks = Join-Path $PSScriptRoot 'soundpacks'
if (Test-Path $soundpacks) {
    Copy-Item $soundpacks (Join-Path $installDir 'soundpacks') -Recurse -Force
    Write-Host 'Copied soundpacks folder'
}

New-Shortcut $menuLink $installedExe 'Keeboy - Mechanical Keyboard Sound Engine'
Write-Host "Start Menu shortcut created (search 'Keeboy')"

if ($Startup) {
    New-Shortcut $startupLink $installedExe 'Keeboy - start with Windows'
    Write-Host 'Startup shortcut created (launches at login)'
}

if (-not $NoLaunch) {
    Start-Process $installedExe
    Write-Host 'Keeboy launched - look for the keycap icon in your system tray.'
}
