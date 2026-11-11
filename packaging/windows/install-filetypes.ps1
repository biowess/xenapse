<#
.SYNOPSIS
    Installs Xenapse file type registrations and icons for the current user.
.DESCRIPTION
    Registers .xnp and .xcss file extensions and ProgIDs under HKEY_CURRENT_USER\Software\Classes.
    Sets friendly display names, MIME content types, and icon locations without associating
    the Xenapse CLI as the default application opener. Tracks ownership and preserves foreign data.
#>
[CmdletBinding()]
param (
    [string]$IconDir = "$env:LOCALAPPDATA\Xenapse\icons"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SourceIconDir = Join-Path $ScriptDir "icons"

$StateDir = Split-Path -Parent $IconDir
if ([string]::IsNullOrEmpty($StateDir)) {
    $StateDir = "$env:LOCALAPPDATA\Xenapse"
}
$StateFile = Join-Path $StateDir "filetypes-state.json"
$JournalFile = Join-Path $StateDir "filetypes-journal.json"
$ClassesRoot = "HKCU:\Software\Classes"

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

# State management
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

function Write-JournalEntry($entry) {
    if (-not (Test-Path $StateDir)) {
        New-Item -ItemType Directory -Path $StateDir -Force | Out-Null
    }
    $tmpJ = "$JournalFile.tmp.$PID"
    try {
        $json = ConvertTo-Json $entry -Depth 4
        Set-Content -Path $tmpJ -Value $json -Encoding UTF8
        Move-Item -Path $tmpJ -Destination $JournalFile -Force
    } finally {
        if (Test-Path $tmpJ) {
            Remove-Item -Path $tmpJ -Force -ErrorAction SilentlyContinue
        }
    }
}

function Clear-Journal {
    if (Test-Path $JournalFile) {
        Remove-Item -Path $JournalFile -Force -ErrorAction Stop
    }
}

# Registry helper functions using .NET API
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

function Set-RegistryValueDirect([string]$keyPath, [string]$valueName, [string]$desiredValue) {
    $subKey = $keyPath -replace '^HKCU:\\?', '' -replace '^HKEY_CURRENT_USER\\?', ''
    $rk = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($subKey, $true)
    if ($null -eq $rk) {
        throw "Cannot open registry key '$keyPath' for writing."
    }
    try {
        $regName = if ($valueName -eq "(default)") { "" } else { $valueName }
        $rk.SetValue($regName, $desiredValue, [Microsoft.Win32.RegistryValueKind]::String)
    } finally {
        $rk.Close()
    }
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

# Reconcile any uncommitted journal entries from an interrupted previous run.
# Conservative policy: a journal records *intent before* the operation. We cannot
# distinguish a resource created by Xenapse from one created by another application
# after the journal was written but before Xenapse's operation completed.
# Therefore: never claim ownership on existence or matching content alone.
# Preserve the resource, warn about the ambiguity, and clear the journal.
# The next install run will re-evaluate: absent resources will be re-created with
# ownership; present unowned resources will be preserved as foreign.
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
        Write-Host "Error: Journal file at '$JournalFile' failed validation. Aborting without performing mutations."
        exit 1
    }

    # Ambiguous recovery: the resource at $j.Path may or may not have been created by
    # Xenapse. Do not add it to ownership state. Preserve it and clear the journal so
    # the next install run handles it conservatively.
    if ($j.Type -eq "Icon") {
        if (Test-Path -PathType Leaf $j.Path) {
            Write-Host "Notice: Ambiguous recovery for icon '$($j.Path)': resource exists but ownership cannot be confirmed. Preserving without claiming ownership."
        } else {
            Write-Host "Notice: Ambiguous recovery for icon '$($j.Path)': resource absent after interrupted install. It will be re-created on next install."
        }
    } elseif ($j.Type -eq "Key") {
        if (Test-Path -LiteralPath $j.Path) {
            Write-Host "Notice: Ambiguous recovery for registry key '$($j.Path)': resource exists but ownership cannot be confirmed. Preserving without claiming ownership."
        } else {
            Write-Host "Notice: Ambiguous recovery for registry key '$($j.Path)': resource absent after interrupted install. It will be re-created on next install."
        }
    } elseif ($j.Type -eq "Value") {
        if (Test-RegistryValueExists $j.Path $j.Name) {
            Write-Host "Notice: Ambiguous recovery for registry value '$($j.Name)' at '$($j.Path)': resource exists but ownership cannot be confirmed. Preserving without claiming ownership."
        } else {
            Write-Host "Notice: Ambiguous recovery for registry value '$($j.Name)' at '$($j.Path)': resource absent after interrupted install. It will be re-created on next install."
        }
    }

    # State is not modified. Clear only the journal; the existing state file is preserved as-is.
    try {
        Clear-Journal
    } catch {
        Write-Host "Error: Failed to clear journal after ambiguous recovery: $_. Aborting to preserve recovery information."
        exit 1
    }
}

# Load and validate prior state file
if (Test-Path $StateFile) {
    try {
        $raw = Get-Content -Path $StateFile -Raw -ErrorAction Stop
        if ([string]::IsNullOrWhiteSpace($raw)) {
            Write-Host "Error: State file at '$StateFile' exists but is empty or contains only whitespace. Aborting installation."
            exit 1
        }
        $parsed = ConvertFrom-Json $raw
        if (-not (Test-ValidXenapseState $parsed $IconDir $ClassesRoot)) {
            Write-Host "Error: Existing state file at '$StateFile' failed validation. Aborting installation."
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
        Write-Host "Error: Failed to read state file at '$StateFile': $_. Aborting installation."
        exit 1
    }
}

Recover-FromJournal

Write-Host "Installing Xenapse file type registrations and icons..."

# 1. Install icons with ownership tracking and transaction rollback
if (-not (Test-Path $IconDir)) {
    New-Item -ItemType Directory -Path $IconDir -Force | Out-Null
}

$Icons = @("xnp.ico", "xcss.ico", "xnp-mono.ico", "xcss-mono.ico")
foreach ($icon in $Icons) {
    $src = Join-Path $SourceIconDir $icon
    $dest = Join-Path $IconDir $icon
    if (Test-Path $src) {
        if (Test-Path $dest) {
            if ($State.installedIcons -contains $dest) {
                $tmpDest = "$dest.tmp.$PID.$([System.IO.Path]::GetRandomFileName())"
                try {
                    Copy-Item -Path $src -Destination $tmpDest -Force
                    Move-Item -Path $tmpDest -Destination $dest -Force
                } catch {
                    Write-Host "Error: Failed to update icon '$dest': $_"
                    throw
                } finally {
                    if (Test-Path $tmpDest) {
                        Remove-Item -Path $tmpDest -Force -ErrorAction SilentlyContinue
                    }
                }
            } else {
                Write-Host "Notice: Icon '${icon}' already exists in '$IconDir' and was not installed by Xenapse. Preserving existing file."
            }
        } else {
            Write-JournalEntry @{ Type = "Icon"; Path = $dest }
            $parentDir = Split-Path -Parent $dest
            if (-not (Test-Path $parentDir)) {
                New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
            }
            $tmpDest = "$dest.tmp.$PID.$([System.IO.Path]::GetRandomFileName())"
            try {
                Copy-Item -Path $src -Destination $tmpDest -Force
                Move-Item -Path $tmpDest -Destination $dest -Force
            } catch {
                Clear-Journal
                throw
            } finally {
                if (Test-Path $tmpDest) {
                    Remove-Item -Path $tmpDest -Force -ErrorAction SilentlyContinue
                }
            }
            $State.installedIcons += $dest
            try {
                Save-State
                Clear-Journal
            } catch {
                # Rollback: remove the icon we just created. Only clear the journal
                # once we have confirmed the icon is gone; retain it on failure so the
                # ambiguous state can be diagnosed on next run.
                $removed = $false
                try {
                    Remove-Item -Path $dest -Force -ErrorAction Stop
                    $removed = $true
                } catch {
                    Write-Host "Warning: Failed to remove '$dest' during rollback. Journal retained for recovery."
                }
                $State.installedIcons = @($State.installedIcons | Where-Object { $_ -ne $dest })
                if ($removed) {
                    Clear-Journal
                }
                throw
            }
        }
    }
}

$XnpIconPath = Join-Path $IconDir "xnp.ico"
$XcssIconPath = Join-Path $IconDir "xcss.ico"

# Ensure Classes root exists
if (-not (Test-Path $ClassesRoot)) {
    New-Item -Path $ClassesRoot -Force | Out-Null
}

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

function Ensure-KeyTracked([string]$keyPath) {
    if (-not (Test-Path $keyPath)) {
        Write-JournalEntry @{ Type = "Key"; Path = $keyPath }
        try {
            New-Item -Path $keyPath -Force | Out-Null
        } catch {
            Clear-Journal
            throw
        }
        $State.createdKeys += $keyPath
        try {
            Save-State
            Clear-Journal
        } catch {
            # Rollback: only remove the key if it is empty — another process may have
            # written values into it between our creation and now. Only clear the journal
            # once the key is confirmed gone; retain it on failure.
            $removed = $false
            $subKeyStr = $keyPath -replace '^HKCU:\\?', '' -replace '^HKEY_CURRENT_USER\\?', ''
            $rk = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($subKeyStr, $false)
            $isEmpty = $false
            if ($null -ne $rk) {
                try { $isEmpty = ($rk.SubKeyCount -eq 0 -and $rk.GetValueNames().Length -eq 0) } finally { $rk.Close() }
            }
            if ($isEmpty) {
                try {
                    Remove-Item -Path $keyPath -Force -ErrorAction Stop
                    $removed = $true
                } catch {
                    Write-Host "Warning: Failed to remove '$keyPath' during rollback. Journal retained for recovery."
                }
            } else {
                Write-Host "Warning: Key '$keyPath' is not empty after creation; not removing during rollback. Journal retained for recovery."
            }
            $State.createdKeys = @($State.createdKeys | Where-Object { $_ -ne $keyPath })
            if ($removed) {
                Clear-Journal
            }
            throw
        }
    }
}

function Set-ValueTracked([string]$keyPath, [string]$valueName, [string]$desiredValue) {
    Ensure-KeyTracked $keyPath

    $exists = Test-RegistryValueExists $keyPath $valueName
    $current = if ($exists) { Get-RegistryValueAndKind $keyPath $valueName } else { $null }

    $matchingEntry = $null
    foreach ($entry in $State.createdValues) {
        if ($entry.Path -eq $keyPath -and $entry.Name -eq $valueName) {
            $matchingEntry = $entry
            break
        }
    }

    if ($exists -and $null -eq $matchingEntry) {
        # Foreign pre-existing value: preserve even if empty or foreign type
        $dispVal = if ($null -ne $current) { $current.Value } else { "" }
        Write-Host "Preserving existing foreign value '$valueName' at '$keyPath': $dispVal"
        return
    }

    if ($null -ne $matchingEntry) {
        # Already owned by Xenapse: compare both content and type
        if ($exists -and $null -ne $current) {
            if ($current.Value -ne [string]$matchingEntry.Value -or $current.Kind -ne [string]$matchingEntry.Kind) {
                Write-Host "Preserving value '$valueName' at '$keyPath': content or type was modified externally."
                return
            }
            if ($current.Value -eq $desiredValue -and $current.Kind -eq "String" -and [string]$matchingEntry.Value -eq $desiredValue -and [string]$matchingEntry.Kind -eq "String") {
                return
            }
        }
        $prevVal = [string]$matchingEntry.Value
        $prevKind = if ($matchingEntry.Kind) { [string]$matchingEntry.Kind } else { "String" }

        Write-JournalEntry @{ Type = "Value"; Path = $keyPath; Name = $valueName; Value = $desiredValue; Kind = "String" }
        try {
            Set-RegistryValueDirect $keyPath $valueName $desiredValue
        } catch {
            Clear-Journal
            throw
        }
        $matchingEntry.Value = $desiredValue
        $matchingEntry.Kind = "String"
        try {
            Save-State
            Clear-Journal
        } catch {
            # Rollback the registry write. Only clear the journal if rollback succeeds.
            $rolledBack = $false
            try {
                if ($exists) {
                    Set-RegistryValueDirect $keyPath $valueName $prevVal
                } else {
                    Remove-RegistryValueSafe $keyPath $valueName
                }
                $rolledBack = $true
            } catch {
                Write-Host "Warning: Failed to restore value '$valueName' at '$keyPath' during rollback. Journal retained for recovery."
            }
            $matchingEntry.Value = $prevVal
            $matchingEntry.Kind = $prevKind
            if ($rolledBack) {
                Clear-Journal
            }
            throw
        }
    } else {
        # Value does not exist: journal intent, create, commit to state, and rollback on failure
        Write-JournalEntry @{ Type = "Value"; Path = $keyPath; Name = $valueName; Value = $desiredValue; Kind = "String" }
        try {
            Set-RegistryValueDirect $keyPath $valueName $desiredValue
        } catch {
            Clear-Journal
            throw
        }
        $newValEntry = @{ Path = $keyPath; Name = $valueName; Value = $desiredValue; Kind = "String" }
        $State.createdValues += $newValEntry
        try {
            Save-State
            Clear-Journal
        } catch {
            # Rollback: only clear the journal if the value is confirmed removed.
            $rolledBack = $false
            try {
                Remove-RegistryValueSafe $keyPath $valueName
                $rolledBack = $true
            } catch {
                Write-Host "Warning: Failed to remove value '$valueName' at '$keyPath' during rollback. Journal retained for recovery."
            }
            $State.createdValues = @($State.createdValues | Where-Object { $_.Path -ne $keyPath -or $_.Name -ne $valueName })
            if ($rolledBack) {
                Clear-Journal
            }
            throw
        }
    }
}

# 2. Register ProgIDs
# Xenapse.Diagram
$DiagramProgId = "$ClassesRoot\Xenapse.Diagram"
if (Has-OpenCommandHandler $DiagramProgId) {
    Write-Host "Preserving existing application handler for Xenapse.Diagram."
} else {
    Ensure-KeyTracked $DiagramProgId
    Set-ValueTracked $DiagramProgId "(default)" "Xenapse Diagram"
    Set-ValueTracked $DiagramProgId "FriendlyTypeName" "Xenapse Diagram"

    $DiagramIconKey = "$DiagramProgId\DefaultIcon"
    Ensure-KeyTracked $DiagramIconKey
    Set-ValueTracked $DiagramIconKey "(default)" "$XnpIconPath,0"
}

# Xenapse.Stylesheet
$StylesheetProgId = "$ClassesRoot\Xenapse.Stylesheet"
if (Has-OpenCommandHandler $StylesheetProgId) {
    Write-Host "Preserving existing application handler for Xenapse.Stylesheet."
} else {
    Ensure-KeyTracked $StylesheetProgId
    Set-ValueTracked $StylesheetProgId "(default)" "Xenapse Stylesheet"
    Set-ValueTracked $StylesheetProgId "FriendlyTypeName" "Xenapse Stylesheet"

    $StylesheetIconKey = "$StylesheetProgId\DefaultIcon"
    Ensure-KeyTracked $StylesheetIconKey
    Set-ValueTracked $StylesheetIconKey "(default)" "$XcssIconPath,0"
}

# 3. Register File Extensions
# .xnp
$XnpExtKey = "$ClassesRoot\.xnp"
Ensure-KeyTracked $XnpExtKey

# Preserve existing foreign default opener association without forcing Xenapse CLI as default opener
$CurrentXnpProgId = (Get-ItemProperty -Path $XnpExtKey -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
if (-not [string]::IsNullOrEmpty($CurrentXnpProgId) -and $CurrentXnpProgId -ne "Xenapse.Diagram") {
    Write-Host "Preserving existing default association for .xnp: $CurrentXnpProgId"
}

Set-ValueTracked $XnpExtKey "Content Type" "application/vnd.xenapse.diagram"
Set-ValueTracked $XnpExtKey "PerceivedType" "document"

# Register in OpenWithProgids without forcing default opener
$XnpOpenWithKey = "$XnpExtKey\OpenWithProgids"
Ensure-KeyTracked $XnpOpenWithKey
Set-ValueTracked $XnpOpenWithKey "Xenapse.Diagram" ""

# .xcss
$XcssExtKey = "$ClassesRoot\.xcss"
Ensure-KeyTracked $XcssExtKey

# Preserve existing foreign default opener association without forcing Xenapse CLI as default opener
$CurrentXcssProgId = (Get-ItemProperty -Path $XcssExtKey -Name "(default)" -ErrorAction SilentlyContinue)."(default)"
if (-not [string]::IsNullOrEmpty($CurrentXcssProgId) -and $CurrentXcssProgId -ne "Xenapse.Stylesheet") {
    Write-Host "Preserving existing default association for .xcss: $CurrentXcssProgId"
}

Set-ValueTracked $XcssExtKey "Content Type" "text/vnd.xenapse.stylesheet"
Set-ValueTracked $XcssExtKey "PerceivedType" "text"

# Register in OpenWithProgids without forcing default opener
$XcssOpenWithKey = "$XcssExtKey\OpenWithProgids"
Ensure-KeyTracked $XcssOpenWithKey
Set-ValueTracked $XcssOpenWithKey "Xenapse.Stylesheet" ""

# 4. Notify Windows Explorer of association changes
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
    # Ignore if execution environment restricts P/Invoke
}

Write-Host "Installation complete."
Write-Host "Registered extensions: .xnp (Xenapse.Diagram), .xcss (Xenapse.Stylesheet)"
Write-Host "Icons installed to: $IconDir"
