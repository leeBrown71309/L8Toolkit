<#
.SYNOPSIS
  Installs (or updates to) the latest published L8 Toolkit release on this PC.

.DESCRIPTION
  Downloads the installer of the latest GitHub release with the GitHub CLI,
  runs it silently (per-user install, no administrator rights) and starts the
  app. Does nothing when the installed version is already the latest one.
  Once installed, later versions arrive through Settings > Updates.
#>
$ErrorActionPreference = "Stop"

. "$PSScriptRoot\l8-app-paths.ps1"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "The GitHub CLI (gh) is required: https://cli.github.com"
}

$release = gh release view --repo $RepositorySlug --json tagName,assets | ConvertFrom-Json
if (-not $release) {
    throw "No published release found on $RepositorySlug"
}
$latestVersion = $release.tagName.TrimStart("v")

$installed = Get-InstalledEntry $ReleaseProductName
if ($installed -and $installed.DisplayVersion -eq $latestVersion) {
    Write-Host "L8 Toolkit $latestVersion is already installed."
    exit 0
}

$installer = $release.assets | Where-Object { $_.name -like "*_x64-setup.exe" } | Select-Object -First 1
if (-not $installer) {
    throw "The release $($release.tagName) has no Windows installer"
}

$downloadDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "l8toolkit-$latestVersion"
New-Item -ItemType Directory -Force -Path $downloadDirectory | Out-Null
gh release download $release.tagName --repo $RepositorySlug --pattern $installer.name --dir $downloadDirectory --clobber
$installerPath = Join-Path $downloadDirectory $installer.name

Write-Host "Installing L8 Toolkit $latestVersion..."
Start-Process -FilePath $installerPath -ArgumentList "/S" -Wait
Remove-Item -Path $downloadDirectory -Recurse -Force

$installDirectory = Get-ReleaseInstallDirectory
if (-not $installDirectory) {
    throw "The installer finished but L8 Toolkit is not registered as installed"
}
$executable = Get-ChildItem -Path $installDirectory -Filter "*.exe" |
    Where-Object { $_.Name -notlike "uninstall*" } |
    Select-Object -First 1
if ($executable) {
    Start-Process -FilePath $executable.FullName
}
Write-Host "L8 Toolkit $latestVersion is installed and running."
