<#
.SYNOPSIS
  Removes the test copy ("L8 Toolkit Dev") from this PC, keeping its settings.

.DESCRIPTION
  Run before a release goes out (`bun run dev:remove`), so only the real,
  installed L8 Toolkit is left on this PC:
    - stops the test copy if it runs (app and the Vite server of `tauri dev`)
    - removes its "launch at startup" entry
    - uninstalls it if a test installer was ever installed
    - deletes the built test executables, so Windows Search only finds the release
  Its settings and history (com.l8.toolkit.dev) are KEPT, so the next development
  session does not start from scratch; -DeleteData removes them too.
  The release (com.l8.toolkit) and its settings are never touched.
  `bun run app:dev` rebuilds the test copy whenever development resumes.
#>
param(
    # Also delete the test copy's settings, history, logs and WebView data.
    [switch] $DeleteData
)
$ErrorActionPreference = "Stop"

. "$PSScriptRoot\l8-app-paths.ps1"

function Write-Step([string] $message) {
    Write-Host "- $message"
}

# 1. Stop the running test copy (anything started from src-tauri\target).
$running = @(Get-Process -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and $_.Path.StartsWith($TargetDirectory, [StringComparison]::OrdinalIgnoreCase) })
# The Vite server `tauri dev` started for it.
$viteListener = Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue |
    Select-Object -First 1
if ($viteListener) {
    $running += @(Get-Process -Id $viteListener.OwningProcess -ErrorAction SilentlyContinue)
}
foreach ($process in $running) {
    Stop-Process -Id $process.Id -Force
    Write-Step "Stopped the test copy (pid $($process.Id))"
}
if ($running) {
    Start-Sleep -Milliseconds 800
}

# 2. Its autostart entry (tauri-plugin-autostart names it after the product).
$runKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
if (Get-ItemProperty -Path $runKey -Name $DevProductName -ErrorAction SilentlyContinue) {
    Remove-ItemProperty -Path $runKey -Name $DevProductName
    Write-Step "Removed the test copy from startup"
}

# 3. A test copy installed with an installer.
$devInstall = Get-InstalledEntry $DevProductName
if ($devInstall -and $devInstall.UninstallString) {
    $uninstaller = $devInstall.UninstallString.Trim('"')
    Start-Process -FilePath $uninstaller -ArgumentList "/S" -Wait
    Write-Step "Uninstalled $DevProductName"
}

# 4. Its data folders, only on request.
if ($DeleteData) {
    foreach ($root in @($env:APPDATA, $env:LOCALAPPDATA)) {
        $folder = Join-Path $root $DevIdentifier
        if (Test-Path $folder) {
            Remove-Item -Path $folder -Recurse -Force
            Write-Step "Deleted $folder"
        }
    }
}

# 5. Built executables, so search tools only find the installed release.
foreach ($profileName in @("debug", "release")) {
    $executable = Join-Path $TargetDirectory "$profileName\l8-tools-kits.exe"
    if (Test-Path $executable) {
        Remove-Item -Path $executable -Force
        Write-Step "Deleted $executable"
    }
}

if (Get-ReleaseInstallDirectory) {
    Write-Host "Done. Only the installed L8 Toolkit is left."
} else {
    Write-Host "Done. The release is not installed yet: run 'bun run release:install'."
}
