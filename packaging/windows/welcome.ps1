$ErrorActionPreference = "Stop"

$InstallDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Executable = Join-Path $InstallDir "xenapse.exe"

if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
    Write-Host "Xenapse could not be found." -ForegroundColor Red
    Write-Host "Expected: $Executable"
    Read-Host "Press Enter to close"
    exit 1
}

# Make Xenapse available in this terminal session.
$env:PATH = "$InstallDir;$env:PATH"

$Version = (& $Executable --version 2>$null | Select-Object -First 1)
if (-not $Version) {
    $Version = "Version unavailable"
}

Clear-Host

Write-Host ""
Write-Host "  X E N A P S E" -ForegroundColor Cyan
Write-Host "  Diagram Language  |  SVG Compiler" -ForegroundColor DarkGray
Write-Host ""
Write-Host "  Welcome to Xenapse." -ForegroundColor White
Write-Host ""
Write-Host "  A lightweight, text-first diagramming language." -ForegroundColor Gray
Write-Host "  Write structured source files and compile them into" -ForegroundColor Gray
Write-Host "  deterministic, self-contained SVG diagrams." -ForegroundColor Gray
Write-Host ""
Write-Host "  VERSION" -ForegroundColor Yellow
Write-Host "  $Version" -ForegroundColor White
Write-Host ""
Write-Host "  GETTING STARTED" -ForegroundColor Yellow
Write-Host "  xenapse --help" -ForegroundColor White
Write-Host "  xenapse --version" -ForegroundColor White
Write-Host "  xenapse check diagram.xnp" -ForegroundColor White
Write-Host "  xenapse compile diagram.xnp" -ForegroundColor White
Write-Host ""
Write-Host "  RESOURCES" -ForegroundColor Yellow
Write-Host "  README.md    Project overview and CLI usage" -ForegroundColor Gray
Write-Host "  docs\        Technical documentation" -ForegroundColor Gray
Write-Host "  examples\    Example diagrams and stylesheets" -ForegroundColor Gray
Write-Host ""
Write-Host "  LICENSE" -ForegroundColor Yellow
Write-Host "  Apache License 2.0" -ForegroundColor Gray
Write-Host ""
Write-Host "  The terminal is ready. Type a command to begin." -ForegroundColor Cyan
Write-Host "  Type 'exit' to close this window." -ForegroundColor Gray
Write-Host ""
