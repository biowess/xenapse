
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Install", "Uninstall")]
    [string]$Action,

    [Parameter(Mandatory = $true)]
    [string]$InstallDir
)

$ErrorActionPreference = "Stop"

# ---------------------------------------------------------------------------
# Paths and registry locations
# ---------------------------------------------------------------------------

function Normalize-Path([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $root = [System.IO.Path]::GetPathRoot($fullPath)

    if ($fullPath.Length -gt $root.Length) {
        $fullPath = $fullPath.TrimEnd('\', '/')
    }

    return $fullPath
}

$InstallDir = Normalize-Path $InstallDir
$Executable = Join-Path $InstallDir "xenapse.exe"

$StateDir = Join-Path $env:LOCALAPPDATA "Xenapse"
$StateFile = Join-Path $StateDir "integration-state.json"

$EnvironmentKey = "Environment"
$AppPathsSubKey = "Software\Microsoft\Windows\CurrentVersion\App Paths\xenapse.exe"

# ---------------------------------------------------------------------------
# User PATH management
# ---------------------------------------------------------------------------

function Read-UserPath {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey(
        $EnvironmentKey,
        $false
    )

    if ($null -eq $key) {
        return @{
            Value = ""
            Kind = [Microsoft.Win32.RegistryValueKind]::String
        }
    }

    try {
        if ($key.GetValueNames() -contains "Path") {
            return @{
                Value = [string]$key.GetValue(
                    "Path",
                    "",
                    [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
                )
                Kind = $key.GetValueKind("Path")
            }
        }

        return @{
            Value = ""
            Kind = [Microsoft.Win32.RegistryValueKind]::String
        }
    }
    finally {
        $key.Close()
    }
}

function Write-UserPath(
    [string]$Value,
    [Microsoft.Win32.RegistryValueKind]$Kind
) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey(
        $EnvironmentKey
    )

    if ($null -eq $key) {
        throw "Unable to open the current user's environment registry key."
    }

    try {
        $key.SetValue("Path", $Value, $Kind)
    }
    finally {
        $key.Close()
    }
}

function Test-PathContainsInstallDir([string]$PathValue) {
    foreach ($entry in ($PathValue -split ';')) {
        if ([string]::IsNullOrWhiteSpace($entry)) {
            continue
        }

        try {
            $normalized = Normalize-Path $entry.Trim('"')

            if ($normalized -ieq $InstallDir) {
                return $true
            }
        }
        catch {
            # Preserve unrecognized or non-filesystem PATH entries.
        }
    }

    return $false
}

# ---------------------------------------------------------------------------
# State management
# ---------------------------------------------------------------------------

function Save-State($State) {
    if (-not (Test-Path -LiteralPath $StateDir -PathType Container)) {
        New-Item -ItemType Directory -Path $StateDir -Force |
            Out-Null
    }

    $tempFile = "$StateFile.tmp.$PID"

    try {
        $State |
            ConvertTo-Json -Depth 3 |
            Set-Content -LiteralPath $tempFile -Encoding UTF8

        Move-Item -LiteralPath $tempFile `
            -Destination $StateFile -Force
    }
    finally {
        if (Test-Path -LiteralPath $tempFile) {
            Remove-Item -LiteralPath $tempFile -Force `
                -ErrorAction SilentlyContinue
        }
    }
}

function Get-ValidatedState {
    if (-not (Test-Path -LiteralPath $StateFile -PathType Leaf)) {
        return $null
    }

    try {
        $state = Get-Content -LiteralPath $StateFile -Raw |
            ConvertFrom-Json
    }
    catch {
        throw "The Xenapse integration state cannot be read. No changes were made."
    }

    if (
        $null -eq $state -or
        $state.InstallDir -isnot [string] -or
        (Normalize-Path $state.InstallDir) -ine $InstallDir -or
        $state.PathAdded -isnot [bool] -or
        $state.AppPathsDefaultAdded -isnot [bool] -or
        $state.AppPathsPathAdded -isnot [bool]
    ) {
        throw "The Xenapse integration state is invalid or belongs to another installation. No changes were made."
    }

    return $state
}

function New-IntegrationState {
    return [PSCustomObject]@{
        InstallDir = $InstallDir
        PathAdded = $false
        AppPathsDefaultAdded = $false
        AppPathsPathAdded = $false
    }
}

# ---------------------------------------------------------------------------
# Windows environment notification
# ---------------------------------------------------------------------------

function Notify-EnvironmentChanged {
    $source = @'
using System;
using System.Runtime.InteropServices;

public static class XenapseEnvironmentNotifier
{
    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
    public static extern IntPtr SendMessageTimeout(
        IntPtr hWnd,
        uint Msg,
        UIntPtr wParam,
        string lParam,
        uint fuFlags,
        uint uTimeout,
        out UIntPtr lpdwResult
    );
}
'@

    try {
        if (-not ("XenapseEnvironmentNotifier" -as [type])) {
            Add-Type -TypeDefinition $source -ErrorAction Stop
        }

        $result = [UIntPtr]::Zero

        [XenapseEnvironmentNotifier]::SendMessageTimeout(
            [IntPtr]0xffff,
            0x001A,
            [UIntPtr]::Zero,
            "Environment",
            2,
            5000,
            [ref]$result
        ) | Out-Null
    }
    catch {
        Write-Warning "Could not broadcast the environment change notification."
    }
}

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------

if ($Action -eq "Install") {

    if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
        throw "Cannot register Xenapse: executable not found at '$Executable'."
    }

    $state = Get-ValidatedState

    if ($null -eq $state) {
        $state = New-IntegrationState
    }

    # Add Xenapse to the current user's PATH only when absent.

    $pathInfo = Read-UserPath

    if (-not (Test-PathContainsInstallDir $pathInfo.Value)) {

        $newPath = if ([string]::IsNullOrWhiteSpace($pathInfo.Value)) {
            $InstallDir
        }
        else {
            "$($pathInfo.Value.TrimEnd(';'));$InstallDir"
        }

        Write-UserPath $newPath $pathInfo.Kind

        $state.PathAdded = $true
        Save-State $state

        Write-Host "Added Xenapse to the current user's PATH."
    }
    else {
        Write-Host "Xenapse is already present in the current user's PATH."
    }

    # Register Xenapse using Windows App Paths.
    # Never modify another application's default registration.

    $appKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey(
        $AppPathsSubKey,
        $true
    )

    if ($null -eq $appKey) {
        $appKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey(
            $AppPathsSubKey
        )
    }

    if ($null -eq $appKey) {
        throw "Unable to create the Xenapse App Paths registry key."
    }

    try {
        $names = @($appKey.GetValueNames())
        $canRegisterPath = $false

        if ($names -notcontains "") {

            $appKey.SetValue(
                "",
                $Executable,
                [Microsoft.Win32.RegistryValueKind]::String
            )

            $state.AppPathsDefaultAdded = $true
            Save-State $state

            $canRegisterPath = $true

            Write-Host "Registered Xenapse executable in Windows App Paths."
        }
        else {
            $existing = [string]$appKey.GetValue(
                "",
                "",
                [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
            )

            if ($existing -ieq $Executable) {
                $canRegisterPath = $true
            }
            else {
                Write-Warning "A different application already owns this App Paths registration. Its registration was preserved."
            }
        }

        # The Path value is meaningful only when the default executable
        # belongs to Xenapse. Do not attach it to another application's key.

        if ($canRegisterPath) {
            $names = @($appKey.GetValueNames())

            if ($names -notcontains "Path") {
                $appKey.SetValue(
                    "Path",
                    $InstallDir,
                    [Microsoft.Win32.RegistryValueKind]::String
                )

                $state.AppPathsPathAdded = $true
                Save-State $state
            }
        }
    }
    finally {
        $appKey.Close()
    }

    Save-State $state
    Notify-EnvironmentChanged

    Write-Host "Xenapse integration installed."
    exit 0
}

# ---------------------------------------------------------------------------
# Uninstall
# ---------------------------------------------------------------------------

$state = Get-ValidatedState

if ($null -eq $state) {
    Write-Host "No Xenapse integration state found. Nothing to remove."
    exit 0
}

# Remove only the PATH entry recorded as Xenapse-owned.
# Preserve all unrelated entries and the original registry value type.

if ($state.PathAdded) {

    $pathInfo = Read-UserPath
    $remaining = @()
    $removed = $false

    foreach ($entry in ($pathInfo.Value -split ';')) {

        $matches = $false

        if (-not [string]::IsNullOrWhiteSpace($entry)) {
            try {
                $matches = (
                    Normalize-Path $entry.Trim('"')
                ) -ieq $InstallDir
            }
            catch {
                # Keep entries that cannot be normalized.
            }
        }

        if ($matches -and -not $removed) {
            $removed = $true
        }
        else {
            $remaining += $entry
        }
    }

    if ($removed) {
        Write-UserPath ($remaining -join ';') $pathInfo.Kind
        Write-Host "Removed Xenapse from the current user's PATH."
    }
}

# Remove only App Paths values explicitly owned by Xenapse.

$appKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey(
    $AppPathsSubKey,
    $true
)

if ($null -ne $appKey) {

    try {
        $names = @($appKey.GetValueNames())

        if (
            $state.AppPathsDefaultAdded -and
            ($names -contains "")
        ) {
            $current = [string]$appKey.GetValue(
                "",
                "",
                [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
            )

            if ($current -ieq $Executable) {
                $appKey.DeleteValue("", $false)
                Write-Host "Removed Xenapse App Paths executable registration."
            }
        }

        $names = @($appKey.GetValueNames())

        if (
            $state.AppPathsPathAdded -and
            ($names -contains "Path")
        ) {
            $currentPath = [string]$appKey.GetValue("Path", "")

            if ($currentPath -ieq $InstallDir) {
                $appKey.DeleteValue("Path", $false)
            }
        }

        # Remove the key only when completely empty.

        if (
            $appKey.SubKeyCount -eq 0 -and
            $appKey.ValueCount -eq 0
        ) {
            $appKey.Close()
            $appKey = $null

            [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKey(
                $AppPathsSubKey,
                $false
            )
        }
    }
    finally {
        if ($null -ne $appKey) {
            $appKey.Close()
        }
    }
}

# Remove only Xenapse's integration state.
# Keep the shared directory if file-association state or other files remain.

Remove-Item -LiteralPath $StateFile -Force `
    -ErrorAction SilentlyContinue

if (Test-Path -LiteralPath $StateDir -PathType Container) {

    $remaining = @(
        Get-ChildItem -LiteralPath $StateDir -Force `
            -ErrorAction SilentlyContinue
    )

    if ($remaining.Count -eq 0) {
        Remove-Item -LiteralPath $StateDir -Force `
            -ErrorAction SilentlyContinue
    }
}

Notify-EnvironmentChanged

Write-Host "Xenapse integration removed."
exit 0
