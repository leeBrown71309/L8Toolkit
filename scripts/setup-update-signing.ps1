<#
.SYNOPSIS
  One-time setup of the key that signs L8 Toolkit updates.

.DESCRIPTION
  1. Creates the signing key pair if it does not exist yet. Tauri asks for a
     password (typed twice, never shown). The private key is written OUTSIDE
     the repository: %USERPROFILE%\.tauri\l8toolkit-updater.key
  2. Writes the public key into src-tauri\tauri.conf.json (it is not secret).
  3. If the GitHub repository exists, stores the private key and its password
     as Actions secrets (TAURI_SIGNING_PRIVATE_KEY and
     TAURI_SIGNING_PRIVATE_KEY_PASSWORD), after checking the password is right.

  Run it again at any time: existing keys are kept, secrets are refreshed.
  BACK UP the key file and keep the password in a password manager: losing
  either means installed copies can never accept another update.
#>
$ErrorActionPreference = "Stop"

. "$PSScriptRoot\l8-app-paths.ps1"

$keyDirectory = Join-Path $env:USERPROFILE ".tauri"
$keyPath = Join-Path $keyDirectory "l8toolkit-updater.key"
$publicKeyPath = "$keyPath.pub"
$configPath = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\src-tauri\tauri.conf.json"))
$projectRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))

# 1. Key pair.
if (Test-Path $keyPath) {
    Write-Host "Signing key found: $keyPath"
} else {
    New-Item -ItemType Directory -Force -Path $keyDirectory | Out-Null
    Write-Host "Creating the update signing key. Choose a strong password and save it in your password manager."
    Push-Location $projectRoot
    try {
        bun x tauri signer generate -w $keyPath
        if ($LASTEXITCODE -ne 0) { throw "Key generation failed" }
    } finally {
        Pop-Location
    }
}

# 2. Public key into the app configuration.
$publicKey = (Get-Content -Raw $publicKeyPath).Trim()
$configText = [System.IO.File]::ReadAllText($configPath)
$config = $configText | ConvertFrom-Json
if ($config.plugins.updater.pubkey -ne $publicKey) {
    # A text edit keeps the file's formatting (ConvertTo-Json would rewrite all of it).
    $updated = [regex]::Replace($configText, '"pubkey":\s*"[^"]*"', ('"pubkey": "' + $publicKey + '"'))
    [System.IO.File]::WriteAllText($configPath, $updated, (New-Object System.Text.UTF8Encoding $false))
    Write-Host "Public key written to src-tauri\tauri.conf.json"
} else {
    Write-Host "The public key is already in src-tauri\tauri.conf.json"
}

# 3. GitHub Actions secrets.
# Windows PowerShell turns a native command's stderr into an error under "Stop":
# a missing repository is an expected answer here, not a failure.
$ErrorActionPreference = "Continue"
gh repo view $RepositorySlug --json name 2>&1 | Out-Null
$repositoryExists = $LASTEXITCODE -eq 0
$ErrorActionPreference = "Stop"
if (-not $repositoryExists) {
    Write-Host "The repository $RepositorySlug does not exist yet: run this script again once it is created."
    exit 0
}

$securePassword = Read-Host "Password of the signing key (to store it as a GitHub secret)" -AsSecureString
$password = [System.Net.NetworkCredential]::new("", $securePassword).Password

# Check the password by signing a scratch file before anything is uploaded.
$probe = New-TemporaryFile
try {
    $env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $password
    Push-Location $projectRoot
    bun x tauri signer sign $probe.FullName 2>&1 | Out-Null
    $signed = $LASTEXITCODE -eq 0
    Pop-Location
} finally {
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY, Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    Remove-Item $probe.FullName, "$($probe.FullName).sig" -ErrorAction SilentlyContinue
}
if (-not $signed) {
    throw "This password does not open the signing key. Nothing was uploaded."
}

Get-Content -Raw $keyPath | gh secret set TAURI_SIGNING_PRIVATE_KEY --repo $RepositorySlug
$password | gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo $RepositorySlug
$password = $null
Write-Host "Secrets TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD are set on $RepositorySlug."
