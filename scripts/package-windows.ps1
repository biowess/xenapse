
<#
.SYNOPSIS
    Package Xenapse release files for Windows x86_64.
.DESCRIPTION
    Builds the release binary and prepares a runtime staging directory,
    portable ZIP archive, and SHA-256 checksum.
    Run from the repository root in PowerShell on Windows.
#>

$ErrorActionPreference = "Stop"

$MetadataJson = cargo metadata --no-deps --format-version 1
if ($LASTEXITCODE -ne 0) {
    throw "Could not read Cargo package metadata."
}

$Metadata = $MetadataJson | ConvertFrom-Json
$CliPackages = @($Metadata.packages | Where-Object { $_.name -eq "xenapse-cli" })

if ($CliPackages.Count -ne 1) {
    throw "Expected exactly one xenapse-cli package in Cargo metadata."
}

$Version = [string]$CliPackages[0].version
$TargetName = "windows-x86_64"
$ArchiveName = "xenapse-$Version-$TargetName"
$DistDir = "dist\$ArchiveName"
$ArchiveFile = "dist\$ArchiveName.zip"
$ChecksumFile = "$ArchiveFile.sha256"

# Refuse to follow a redirected dist directory.
$DistItem = Get-Item -LiteralPath "dist" -Force -ErrorAction SilentlyContinue
if ($null -ne $DistItem -and
    ($DistItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
    throw "The dist directory is a reparse point. Refusing to proceed."
}

# Validate required source files before modifying dist.
$RequiredFiles = @(
    "LICENSE",
    "NOTICE",
    "README.md",
    "assets\icons\xenapse.ico",
    "packaging\windows\welcome.ps1",
    "packaging\windows\integration.ps1",
    "packaging\windows\install-filetypes.ps1",
    "packaging\windows\uninstall-filetypes.ps1",
    "packaging\windows\icons\xnp.ico",
    "packaging\windows\icons\xcss.ico"
)

$RequiredDirectories = @(
    "docs",
    "examples"
)

foreach ($Path in $RequiredFiles) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Required file is missing: $Path"
    }
}

foreach ($Path in $RequiredDirectories) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "Required directory is missing: $Path"
    }
}

# Build the Windows release binary.
cargo build --release -p xenapse-cli
if ($LASTEXITCODE -ne 0) {
    throw "Cargo release build failed."
}

if (-not (Test-Path -LiteralPath "target\release\xenapse.exe" -PathType Leaf)) {
    throw "Release executable was not produced."
}

# Refuse to remove a redirected staging directory.
$StageItem = Get-Item -LiteralPath $DistDir -Force -ErrorAction SilentlyContinue
if ($null -ne $StageItem -and
    ($StageItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
    throw "The staging directory is a reparse point. Refusing to proceed."
}

# Remove only Xenapse's exact staging directory and release archive files.
if (Test-Path -LiteralPath $DistDir) {
    Remove-Item -LiteralPath $DistDir -Recurse -Force
}

foreach ($Path in @($ArchiveFile, $ChecksumFile)) {
    $Item = Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    if ($null -ne $Item) {
        if ($Item.PSIsContainer) {
            throw "Expected a file but found a directory: $Path"
        }
        Remove-Item -LiteralPath $Path -Force
    }
}

New-Item -ItemType Directory -Path $DistDir -Force | Out-Null

# Core application files.
Copy-Item "target\release\xenapse.exe" -Destination $DistDir
Copy-Item "LICENSE" -Destination $DistDir
Copy-Item "NOTICE" -Destination $DistDir
Copy-Item "README.md" -Destination $DistDir
Copy-Item "assets\icons\xenapse.ico" -Destination "$DistDir\xenapse.ico"

# Welcome terminal and permanent installation integration.
Copy-Item "packaging\windows\welcome.ps1" -Destination "$DistDir\welcome.ps1"
Copy-Item "packaging\windows\integration.ps1" -Destination "$DistDir\integration.ps1"

# Documentation and examples.
New-Item -ItemType Directory -Path "$DistDir\docs" -Force | Out-Null
Copy-Item "docs\*" -Destination "$DistDir\docs" -Recurse -Force

New-Item -ItemType Directory -Path "$DistDir\examples" -Force | Out-Null
Copy-Item "examples\*" -Destination "$DistDir\examples" -Recurse -Force

# Optional Windows file associations.
# Keep the original relative icons directory expected by the scripts.
$WindowsPackageDir = "$DistDir\packaging\windows"
New-Item -ItemType Directory -Path "$WindowsPackageDir\icons" -Force | Out-Null

Copy-Item "packaging\windows\install-filetypes.ps1" -Destination $WindowsPackageDir
Copy-Item "packaging\windows\uninstall-filetypes.ps1" -Destination $WindowsPackageDir
Copy-Item "packaging\windows\icons\*" -Destination "$WindowsPackageDir\icons" -Force

# The installer source directory is deliberately not copied into the stage.
# This same stage will be used by Inno Setup.

# Create portable ZIP.
Compress-Archive -Path "$DistDir\*" -DestinationPath $ArchiveFile -Force

# Generate SHA-256 checksum.
$Hash = (Get-FileHash -Algorithm SHA256 $ArchiveFile).Hash.ToLowerInvariant()
"$Hash  $ArchiveName.zip" | Set-Content -LiteralPath $ChecksumFile -Encoding ASCII

Write-Host ""
Write-Host "Packaging complete." -ForegroundColor Green
Write-Host "Staging:  $DistDir"
Write-Host "Archive:  $ArchiveFile"
Write-Host "Checksum: $ChecksumFile"
Write-Host "SHA-256:  $Hash"
