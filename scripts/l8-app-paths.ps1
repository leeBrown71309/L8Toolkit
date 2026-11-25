# Shared locations of the two L8 Toolkit copies, dot-sourced by the other scripts.
#   Release: installed by the NSIS installer, identifier com.l8.toolkit.
#   Test (dev): run by `bun run app:dev` from src-tauri\target, identifier com.l8.toolkit.dev.

$ReleaseProductName = "L8 Toolkit"
$DevProductName = "L8 Toolkit Dev"
$DevIdentifier = "com.l8.toolkit.dev"
$RepositorySlug = "leeBrown71309/L8Toolkit"
$TargetDirectory = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\src-tauri\target"))

$UninstallKeys = @(
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*"
)

function Get-InstalledEntry([string] $displayName) {
    Get-ItemProperty $UninstallKeys -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -eq $displayName } |
        Select-Object -First 1
}

# Folder of the installed release, or $null when it is not installed.
function Get-ReleaseInstallDirectory {
    $entry = Get-InstalledEntry $ReleaseProductName
    if (-not $entry -or -not $entry.InstallLocation) {
        return $null
    }
    return $entry.InstallLocation.Trim('"').TrimEnd('\') + '\'
}
