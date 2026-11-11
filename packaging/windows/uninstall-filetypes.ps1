<#
.SYNOPSIS
    Uninstalls Xenapse file type registrations and removes icons for the current user.
.DESCRIPTION
    Removes only Xenapse-owned ProgIDs, extension entries, and installed icons.
    Preserves foreign data, application handlers, and pre-existing files.
#>
[CmdletBinding()]
param (
    [string]$IconDir = "$env:LOCALAPPDATA\Xenapse\icons"
)

$ErrorActionPreference = "Continue"

Write-Host "Uninstalling Xenapse file type registrations..."

$ClassesRoot = "HKCU:\Software\Classes"

$StateDir = Split-Path -Parent $IconDir
if ([string]::IsNullOrEmpty($StateDir)) {
    $StateDir = "$env:LOCALAPPDATA\Xenapse"
}
$StateFile = Join-Path $StateDir "filetypes-state.json"
$JournalFile = Join-Path $StateDir "filetypes-journal.json"

if (-not (Test-Path $StateFile) -and -not (Test-Path $JournalFile)) {
    Write-Host "Notice: No Xenapse installation state found at $StateFile. Preserving all unverified registrations and files."
    exit 0
}

function Test-ValidXenapseState($s, [string]$expectedIconDir, [string]$classesRoot) {
    if ($null -eq $s) { return $false }

    $allowedIcons = @(
        (Join-Path $expectedIconDir "xnp.ico"),
        (Join-Path $expectedIconDir "xcss.ico"),
        (Join-Path $expectedIconDir "xnp-mono.ico"),
        (Join-Path $expectedIconDir "xcss-mono.ico")
    )

    $allowedKeys = @(
        "$classesRoot\Xenapse.Diagram",
        "$classesRoot\Xenapse.Diagram\DefaultIcon",
        "$classesRoot\Xenapse.Stylesheet",
        "$classesRoot\Xenapse.Stylesheet\DefaultIcon",
        "$classesRoot\.xnp",
        "$classesRoot\.xnp\OpenWithProgids",
        "$classesRoot\.xcss",
        "$classesRoot\.xcss\OpenWithProgids"
    )

    $allowedValues = @{
        "$classesRoot\Xenapse.Diagram" = @("(default)", "FriendlyTypeName")
        "$classesRoot\Xenapse.Diagram\DefaultIcon" = @("(default)")
        "$classesRoot\Xenapse.Stylesheet" = @("(default)", "FriendlyTypeName")
        "$classesRoot\Xenapse.Stylesheet\DefaultIcon" = @("(default)")
        "$classesRoot\.xnp" = @("Content Type", "PerceivedType")
        "$classesRoot\.xnp\OpenWithProgids" = @("Xenapse.Diagram")
        "$classesRoot\.xcss" = @("Content Type", "PerceivedType")
        "$classesRoot\.xcss\OpenWithProgids" = @("Xenapse.Stylesheet")
    }

    # Required top-level properties must be present and must be actual arrays (null and scalars are invalid)
    $propNames = @($s.PSObject.Properties.Name)
    foreach ($req in @("installedIcons", "createdKeys", "createdValues")) {
        if ($propNames -notcontains $req) {
            Write-Host "Validation failed: missing required property '$req' in state"
            return $false
        }
        $val = $s.$req
        if ($null -eq $val -or $val -isnot [System.Array]) {
            Write-Host "Validation failed: property '$req' must be an array in state"
            return $false
        }
    }

    $seenIcons = @{}
    foreach ($icon in $s.installedIcons) {
        if (-not ($icon -is [string]) -or ($allowedIcons -notcontains $icon)) {
            Write-Host "Validation failed: out-of-scope icon path in state: $icon"
            return $false
        }
        if ($seenIcons.ContainsKey($icon)) {
            Write-Host "Validation failed: duplicate icon entry in state: $icon"
            return $false
        }
        $seenIcons[$icon] = $true
    }

    $seenKeys = @{}
    foreach ($key in $s.createdKeys) {
        if (-not ($key -is [string]) -or ($allowedKeys -notcontains $key)) {
            Write-Host "Validation failed: out-of-scope registry key in state: $key"
            return $false
        }
        if ($seenKeys.ContainsKey($key)) {
            Write-Host "Validation failed: duplicate key entry in state: $key"
            return $false
        }
        $seenKeys[$key] = $true
    }

    $seenValues = @{}
    foreach ($v in $s.createdValues) {
        if ($null -eq $v.Path -or $null -eq $v.Name -or $null -eq $v.Value) {
            Write-Host "Validation failed: malformed value entry in state (missing Path, Name, or Value)"
            return $false
        }
        $p = [string]$v.Path
        $n = [string]$v.Name
        if (-not $allowedValues.ContainsKey($p)) {
            Write-Host "Validation failed: out-of-scope key for value in state: $p"
            return $false
        }
        $validNames = $allowedValues[$p]
        if ($validNames -notcontains $n) {
            Write-Host "Validation failed: out-of-scope value name in state: $n under $p"
            return $false
        }
        if (-not ($v.Value -is [string])) {
            Write-Host "Validation failed: Value field must be a string in state entry: $n under $p"
            return $false
        }
        if ($null -eq $v.Kind -or -not ($v.Kind -is [string]) -or $v.Kind -ne "String") {
            Write-Host "Validation failed: Kind field must be the string 'String' in state entry: $n under $p"
            return $false
        }
        $dedupKey = "$p||$n"
        if ($seenValues.ContainsKey($dedupKey)) {
            Write-Host "Validation failed: duplicate value entry in state: $n under $p"
            return $false
        }
        $seenValues[$dedupKey] = $true
    }

    return $true
}

$State = @{
    installedIcons = @()
    createdKeys = @()
    createdValues = @()
}

function Save-State {
    if (-not (Test-Path $StateDir)) {
        New-Item -ItemType Directory -Path $StateDir -Force | Out-Null
    }
    $tmpFile = "$StateFile.tmp.$PID"
    try {
        $json = ConvertTo-Json $State -Depth 4
        Set-Content -Path $tmpFile -Value $json -Encoding UTF8
        Move-Item -Path $tmpFile -Destination $StateFile -Force
    } finally {
        if (Test-Path $tmpFile) {
            Remove-Item -Path $tmpFile -Force -ErrorAction SilentlyContinue
        }
    }
}

function Clear-Journal {
    if (Test-Path $JournalFile) {
        Remove-Item -Path $JournalFile -Force -ErrorAction Stop
    }
}

function Test-RegistryValueExists([string]$keyPath, [string]$valueName) {
    if (-not (Test-Path $keyPath)) { return $false }
    $subKey = $keyPath -replace '^HKCU:\\?', '' -replace '^HKEY_CURRENT_USER\\?', ''
    $rk = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($subKey, $false)
    if ($null -eq $rk) { return $false }
    try {
        $regName = if ($valueName -eq "(default)") { "" } else { $valueName }
        return ($rk.GetValueNames() -contains $regName)
    } finally {
        $rk.Close()
    }
}

function Get-RegistryValueAndKind([string]$keyPath, [string]$valueName) {
    if (-not (Test-Path $keyPath)) { return $null }
    $subKey = $keyPath -replace '^HKCU:\\?', '' -replace '^HKEY_CURRENT_USER\\?', ''
    $rk = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($subKey, $false)
    if ($null -eq $rk) { return $null }
    try {
        $regName = if ($valueName -eq "(default)") { "" } else { $valueName }
        if ($rk.GetValueNames() -contains $regName) {
            $val = $rk.GetValue($regName, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            $kind = $rk.GetValueKind($regName).ToString()
            return @{ Value = [string]$val; Kind = $kind }
        }
    } finally {
        $rk.Close()
    }
    return $null
}

function Remove-RegistryValueSafe([string]$keyPath, [string]$valueName) {
    if (-not (Test-Path $keyPath)) { return }
    $subKey = $keyPath -replace '^HKCU:\\?', '' -replace '^HKEY_CURRENT_USER\\?', ''
    $rk = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($subKey, $true)
    if ($null -eq $rk) { return }
    try {
        $regName = if ($valueName -eq "(default)") { "" } else { $valueName }
        $rk.DeleteValue($regName, $false)
    } finally {
        $rk.Close()
    }
}

function Test-ValidJournalEntry($j, [string]$expectedIconDir, [string]$classesRoot) {
    if ($null -eq $j) { return $false }
    if ($j -isnot [PSCustomObject] -and $j -isnot [hashtable]) { return $false }

    $propNames = if ($j -is [hashtable]) { @($j.Keys) } else { @($j.PSObject.Properties.Name) }
    if ($propNames -notcontains "Type" -or $propNames -notcontains "Path") { return $false }
    if ($null -eq $j.Type -or $null -eq $j.Path) { return $false }
    $type = [string]$j.Type
    $path = [string]$j.Path

    $allowedIcons = @(
        (Join-Path $expectedIconDir "xnp.ico"),
        (Join-Path $expectedIconDir "xcss.ico"),
        (Join-Path $expectedIconDir "xnp-mono.ico"),
        (Join-Path $expectedIconDir "xcss-mono.ico")
    )

    $allowedKeys = @(
        "$classesRoot\Xenapse.Diagram",
        "$classesRoot\Xenapse.Diagram\DefaultIcon",
        "$classesRoot\Xenapse.Stylesheet",
        "$classesRoot\Xenapse.Stylesheet\DefaultIcon",
        "$classesRoot\.xnp",
        "$classesRoot\.xnp\OpenWithProgids",
        "$classesRoot\.xcss",
        "$classesRoot\.xcss\OpenWithProgids"
    )

    $allowedValues = @{
        "$classesRoot\Xenapse.Diagram" = @("(default)", "FriendlyTypeName")
        "$classesRoot\Xenapse.Diagram\DefaultIcon" = @("(default)")
        "$classesRoot\Xenapse.Stylesheet" = @("(default)", "FriendlyTypeName")
        "$classesRoot\Xenapse.Stylesheet\DefaultIcon" = @("(default)")
        "$classesRoot\.xnp" = @("Content Type", "PerceivedType")
        "$classesRoot\.xnp\OpenWithProgids" = @("Xenapse.Diagram")
        "$classesRoot\.xcss" = @("Content Type", "PerceivedType")
        "$classesRoot\.xcss\OpenWithProgids" = @("Xenapse.Stylesheet")
    }

    if ($type -eq "Icon") {
        if ($allowedIcons -notcontains $path) {
            Write-Host "Journal validation failed: out-of-scope icon path: $path"
            return $false
        }
        return $true
    } elseif ($type -eq "Key") {
        if ($allowedKeys -notcontains $path) {
            Write-Host "Journal validation failed: out-of-scope registry key: $path"
            return $false
        }
        return $true
    } elseif ($type -eq "Value") {
        if ($propNames -notcontains "Name" -or $propNames -notcontains "Value") {
            Write-Host "Journal validation failed: incomplete value entry in journal"
            return $false
        }
        if ($null -eq $j.Name -or $null -eq $j.Value) {
            Write-Host "Journal validation failed: null Name or Value in journal entry"
            return $false
        }
        $name = [string]$j.Name
        if (-not $allowedValues.ContainsKey($path)) {
            Write-Host "Journal validation failed: out-of-scope key for value in journal: $path"
            return $false
        }
        $validNames = $allowedValues[$path]
        if ($validNames -notcontains $name) {
            Write-Host "Journal validation failed: out-of-scope value name in journal: $name under $path"
            return $false
        }
        if ($propNames -contains "Kind" -and $j.Kind -and [string]$j.Kind -ne "String") {
            Write-Host "Journal validation failed: unexpected value kind in journal: $($j.Kind)"
            return $false
        }
        return $true
    } else {
        Write-Host "Journal validation failed: unknown journal type: $type"
        return $false
    }
}

function Recover-FromJournal {
    if (-not (Test-Path $JournalFile)) { return }
    $raw = $null
    try {
        $raw = Get-Content -Path $JournalFile -Raw -ErrorAction Stop
    } catch {
        Write-Host "Error: Unable to read journal file at '$JournalFile': $_. Aborting."
        exit 1
    }

    if ([string]::IsNullOrWhiteSpace($raw)) {
        Write-Host "Error: Journal file at '$JournalFile' is empty or incomplete. Aborting."
        exit 1
    }

    $j = $null
    try {
        $j = ConvertFrom-Json $raw -ErrorAction Stop
    } catch {
        Write-Host "Error: Journal file at '$JournalFile' contains invalid JSON: $_. Aborting."
        exit 1
    }

    if (-not (Test-ValidJournalEntry $j $IconDir $ClassesRoot)) {
        Write-Host "Error: Journal file at '$JournalFile' failed validation. Aborting without performing uninstallation mutations."
        exit 1
    }

    # Conservative policy: a journal records intent before an installer operation that was
    # interrupted before completing. We cannot determine how far the interrupted install
    # progressed. Do not modify ownership state. Preserve the resource and clear the journal.
    # The existing state file, loaded before this function, already reflects prior confirmed ownership.
    if ($j.Type -eq "Icon") {
        if (Test-Path -PathType Leaf $j.Path) {
            Write-Host "Notice: Ambiguous recovery for icon '$($j.Path)': resource exists; uninstall completion status unknown. Preserving without altering ownership state."
        } else {
            Write-Host "Notice: Ambiguous recovery for icon '$($j.Path)': resource absent; it was likely removed before the interruption."
        }
    } elseif ($j.Type -eq "Key") {
        if (Test-Path -LiteralPath $j.Path) {
            Write-Host "Notice: Ambiguous recovery for registry key '$($j.Path)': resource exists; uninstall completion status unknown. Preserving without altering ownership state."
        } else {
            Write-Host "Notice: Ambiguous recovery for registry key '$($j.Path)': resource absent; it was likely removed before the interruption."
        }
    } elseif ($j.Type -eq "Value") {
        if (Test-RegistryValueExists $j.Path $j.Name) {
            Write-Host "Notice: Ambiguous recovery for registry value '$($j.Name)' at '$($j.Path)': resource exists; uninstall completion status unknown. Preserving without altering ownership state."
        } else {
            Write-Host "Notice: Ambiguous recovery for registry value '$($j.Name)' at '$($j.Path)': resource absent; it was likely removed before the interruption."
        }
    }

    # State is not modified. Clear only the journal.
    try {
        Clear-Journal
    } catch {
        Write-Host "Error: Failed to clear journal after ambiguous recovery: $_. Aborting to preserve recovery information."
        exit 1
    }
}

if (Test-Path $StateFile) {
    try {
        $raw = Get-Content -Path $StateFile -Raw -ErrorAction Stop
        if ([string]::IsNullOrWhiteSpace($raw)) {
            Write-Host "Error: State file at '$StateFile' exists but is empty or contains only whitespace. Aborting uninstallation."
            exit 1
        }
        $parsed = ConvertFrom-Json $raw
        if (-not (Test-ValidXenapseState $parsed $IconDir $ClassesRoot)) {
            Write-Host "Error: State file at '$StateFile' failed validation. Aborting without performing registry mutations."
            exit 1
        }
        if ($parsed.installedIcons) { $State.installedIcons = @($parsed.installedIcons) }
        if ($parsed.createdKeys) { $State.createdKeys = @($parsed.createdKeys) }
        if ($parsed.createdValues) {
            foreach ($cv in $parsed.createdValues) {
                $kind = if ($cv.Kind) { [string]$cv.Kind } else { "String" }
                $State.createdValues += @{ Path = [string]$cv.Path; Name = [string]$cv.Name; Value = [string]$cv.Value; Kind = $kind }
            }
        }
    } catch {
        Write-Host "Warning: Failed to parse Xenapse state file at $StateFile. Preserving files to prevent accidental data loss."
        exit 1
    }
}

Recover-FromJournal

function Test-KeyIsEmpty([string]$keyPath) {
    if (-not (Test-Path $keyPath)) { return $false }
    $subKey = $keyPath -replace '^HKCU:\\?', '' -replace '^HKEY_CURRENT_USER\\?', ''
    $rk = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($subKey, $false)
    if ($null -eq $rk) { return $false }
    try {
        if ($rk.SubKeyCount -gt 0) { return $false }
        $valNames = $rk.GetValueNames()
        if ($null -ne $valNames -and $valNames.Length -gt 0) {
            return $false
        }
        return $true
    } finally {
        $rk.Close()
    }
}

# Check whether an actual application handler (shell\open\command) is registered on the ProgIDs
function Has-OpenCommandHandler([string]$progIdPath) {
    $cmdKey = "$progIdPath\shell\open\command"
    if (Test-Path $cmdKey) {
        $val = (Get-ItemProperty -Path $cmdKey -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
        if (-not [string]::IsNullOrWhiteSpace($val)) {
            return $true
        }
        $cmdItem = Get-Item $cmdKey -ErrorAction SilentlyContinue
        if ($null -ne $cmdItem -and ($cmdItem.ValueCount -gt 0 -or $cmdItem.SubKeyCount -gt 0)) {
            return $true
        }
    }
    return $false
}

$DiagramHasApp = Has-OpenCommandHandler "$ClassesRoot\Xenapse.Diagram"
$StylesheetHasApp = Has-OpenCommandHandler "$ClassesRoot\Xenapse.Stylesheet"

# 1. Remove values created by Xenapse, preserving any values modified by other applications
$remainingValues = @()
if ($null -ne $State.createdValues) {
    foreach ($entry in $State.createdValues) {
        $p = [string]$entry.Path
        $n = [string]$entry.Name
        $expectedVal = [string]$entry.Value
        $expectedKind = if ($entry.Kind) { [string]$entry.Kind } else { "String" }

        if ($p -like "*Xenapse.Diagram*" -and $DiagramHasApp) {
            Write-Host "Preserving created value '$n' at '$p': active application handler detected."
            $remainingValues += $entry
            continue
        }
        if ($p -like "*Xenapse.Stylesheet*" -and $StylesheetHasApp) {
            Write-Host "Preserving created value '$n' at '$p': active application handler detected."
            $remainingValues += $entry
            continue
        }

        if (Test-Path $p) {
            if (Test-RegistryValueExists $p $n) {
                $current = Get-RegistryValueAndKind $p $n
                if ($null -ne $current -and ($current.Value -ne $expectedVal -or $current.Kind -ne $expectedKind)) {
                    Write-Host "Preserving value '$n' at '$p': content or type was modified externally."
                    continue
                }
                Remove-RegistryValueSafe $p $n
                if (Test-RegistryValueExists $p $n) {
                    Write-Host "Warning: Failed to remove value '$n' at '$p'."
                    $remainingValues += $entry
                }
            }
        }
    }
}
$State.createdValues = @($remainingValues)

# 2. Remove registry keys created by Xenapse (deepest subkeys first)
$remainingKeys = @()
if ($null -ne $State.createdKeys) {
    $sortedKeys = $State.createdKeys | Sort-Object -Descending { $_.Length }
    foreach ($k in $sortedKeys) {
        if ($k -like "*Xenapse.Diagram*" -and $DiagramHasApp) {
            Write-Host "Preserving key '$k': active application handler detected."
            $remainingKeys += $k
            continue
        }
        if ($k -like "*Xenapse.Stylesheet*" -and $StylesheetHasApp) {
            Write-Host "Preserving key '$k': active application handler detected."
            $remainingKeys += $k
            continue
        }
        if (Test-Path $k) {
            if (Test-KeyIsEmpty $k) {
                Remove-Item -Path $k -Force -ErrorAction SilentlyContinue
                if (Test-Path $k) {
                    Write-Host "Warning: Failed to remove key '$k'."
                    $remainingKeys += $k
                }
            } else {
                Write-Host "Preserving key '$k': foreign values or subkeys remain."
                $remainingKeys += $k
            }
        }
    }
}
$State.createdKeys = @($remainingKeys)

# 3. Clean up icons (only if installed by Xenapse and not referenced by preserved registrations)
function Test-IconReferenced([string]$iconFileName, [string]$iconFullPath) {
    if (Test-Path "$ClassesRoot\Xenapse.Diagram") {
        if ($DiagramHasApp -and $iconFileName -like "*xnp*") {
            return $true
        }
        $diagIcon = (Get-ItemProperty -Path "$ClassesRoot\Xenapse.Diagram\DefaultIcon" -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
        if (-not [string]::IsNullOrEmpty($diagIcon) -and ($diagIcon -like "*$iconFileName*" -or $diagIcon -like "*$iconFullPath*")) {
            return $true
        }
    }
    if (Test-Path "$ClassesRoot\Xenapse.Stylesheet") {
        if ($StylesheetHasApp -and $iconFileName -like "*xcss*") {
            return $true
        }
        $styleIcon = (Get-ItemProperty -Path "$ClassesRoot\Xenapse.Stylesheet\DefaultIcon" -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
        if (-not [string]::IsNullOrEmpty($styleIcon) -and ($styleIcon -like "*$iconFileName*" -or $styleIcon -like "*$iconFullPath*")) {
            return $true
        }
    }
    return $false
}

$remainingIcons = @()
$Icons = @("xnp.ico", "xcss.ico", "xnp-mono.ico", "xcss-mono.ico")
foreach ($icon in $Icons) {
    $p = Join-Path $IconDir $icon
    if (Test-Path $p) {
        if ($null -eq $State.installedIcons -or ($State.installedIcons -notcontains $p)) {
            Write-Host "Preserving icon ${icon}: was not installed by Xenapse."
            continue
        }
        if (Test-IconReferenced $icon $p) {
            Write-Host "Preserving icon ${icon}: referenced by preserved application registration."
            $remainingIcons += $p
        } else {
            Remove-Item -Path $p -Force -ErrorAction SilentlyContinue
            if (Test-Path $p) {
                Write-Host "Warning: Failed to remove icon '$p'."
                $remainingIcons += $p
            }
        }
    }
}
$State.installedIcons = @($remainingIcons)

if (Test-Path $IconDir) {
    $remaining = Get-ChildItem -Path $IconDir -Force -ErrorAction SilentlyContinue
    if ($null -eq $remaining -or $remaining.Count -eq 0) {
        Remove-Item -Path $IconDir -Force -ErrorAction SilentlyContinue
    } else {
        Write-Host "Preserving icon directory: referenced or unowned icon files remain."
    }
}

# 4. State file retention or cleanup
$hasRemaining = ($State.installedIcons.Count -gt 0) -or ($State.createdKeys.Count -gt 0) -or ($State.createdValues.Count -gt 0)

if ($hasRemaining) {
    Save-State
    Write-Host "Notice: Some Xenapse-owned resources were preserved and recorded in $StateFile for future uninstallation."
} else {
    Remove-Item -Path $StateFile -Force -ErrorAction SilentlyContinue
    Clear-Journal
    if (Test-Path $StateDir) {
        $stateRemaining = Get-ChildItem -Path $StateDir -Force -ErrorAction SilentlyContinue
        if ($null -eq $stateRemaining -or $stateRemaining.Count -eq 0) {
            Remove-Item -Path $StateDir -Force -ErrorAction SilentlyContinue
        }
    }
}

# 5. Notify Windows Explorer of association changes
$code = @'
using System;
using System.Runtime.InteropServices;
public class ShellNotifier {
    [DllImport("shell32.dll", SetLastError = true)]
    public static extern void SHChangeNotify(int wEventId, uint uFlags, IntPtr dwItem1, IntPtr dwItem2);
}
'@
try {
    Add-Type -TypeDefinition $code -ErrorAction SilentlyContinue
    [ShellNotifier]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)
} catch {
    # Ignore if restricted
}

Write-Host "Uninstallation complete."
