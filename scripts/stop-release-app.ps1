<#
.SYNOPSIS
  Stops the installed L8 Toolkit (the release) before the development copy starts.

.DESCRIPTION
  Both copies draw a notch and want the same global shortcut, so only one may
  run at a time. `bun run app:dev` calls this first. The release comes back at
  the next sign-in (if "Launch at startup" is on) or from the Start menu.
  Nothing is uninstalled and no data is touched.
#>
$ErrorActionPreference = "Stop"

. "$PSScriptRoot\l8-app-paths.ps1"

$installDirectory = Get-ReleaseInstallDirectory
if (-not $installDirectory) {
    exit 0
}

$running = Get-Process -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and $_.Path.StartsWith($installDirectory, [StringComparison]::OrdinalIgnoreCase) }
foreach ($process in $running) {
    Write-Host "Stopping the installed L8 Toolkit (pid $($process.Id)) while the test version runs."
    Stop-Process -Id $process.Id -Force
}
