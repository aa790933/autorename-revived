# AutoRename-Revived Context Menu Installer
# Run: powershell -ExecutionPolicy Bypass -File setup.ps1

$Host.UI.RawUI.WindowTitle = "AutoRename-Revived Setup"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$exePath = Join-Path $scriptDir "autorename-revived-cli.exe"
$installPaddle = $false

function Write-Banner {
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "   AutoRename-Revived v3.0.4 Setup" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
}

function Add-ContextMenu {
    param([string]$MenuText, [string]$Command, [string]$IconPath)

    $regPath = "HKCU:\Software\Classes\$Command"
    if (-not (Test-Path $regPath)) {
        New-Item -Path $regPath -Force | Out-Null
    }
    Set-ItemProperty -Path $regPath -Name "(Default)" -Value $MenuText
    Set-ItemProperty -Path $regPath -Name "Icon" -Value $IconPath

    $cmdPath = "$regPath\shell\open\command"
    if (-not (Test-Path $cmdPath)) {
        New-Item -Path $cmdPath -Force | Out-Null
    }
    Set-ItemProperty -Path $cmdPath -Name "(Default)" -Value $Command

    Write-Host "  [+] Added: '$MenuText'" -ForegroundColor Green
}

function Remove-ContextMenu {
    param([string]$Command)

    $regPath = "HKCU:\Software\Classes\$Command"
    if (Test-Path $regPath) {
        Remove-Item -Path $regPath -Recurse -Force
        Write-Host "  [-] Removed: '$Command'" -ForegroundColor Yellow
    }
}

function Install-PaddleOCR {
    Write-Host "`nInstalling PaddleOCR (~500MB download)..." -ForegroundColor Yellow
    $paddleVenv = Join-Path $scriptDir "venv_paddleocr"

    if (-not (Test-Path $paddleVenv)) {
        & python -m venv $paddleVenv
    }

    $pip = Join-Path $paddleVenv "Scripts\pip.exe"
    & $pip install --upgrade pip setuptools wheel
    & $pip install paddleocr paddlepaddle
    Write-Host "  [OK] PaddleOCR installed in $paddleVenv" -ForegroundColor Green
}

# ── Main ──────────────────────────────────────────────────────────────────

Write-Banner

$hasCli = Test-Path $exePath

if (-not $hasCli) {
    Write-Host "[!] CLI EXE not found. Place this script next to:" -ForegroundColor Red
    Write-Host "    - autorename-revived-cli.exe" -ForegroundColor Yellow
    Write-Host "  Download from: https://github.com/aa790933/autorename-revived/releases`n" -ForegroundColor Yellow
    $choice = Read-Host "Proceed anyway? (y/N)"
    if ($choice -ne "y") { exit 1 }
}

$hasPython = $null -ne (Get-Command "python" -ErrorAction SilentlyContinue)

Write-Host "Choose setup mode:" -ForegroundColor White
Write-Host "  1) Install context menu entries only" -ForegroundColor White
Write-Host "  2) Install context menu + PaddleOCR ($(if (-not $hasPython) { 'requires Python' } else { '~500MB' }))" -ForegroundColor White
Write-Host "  3) Remove all context menu entries" -ForegroundColor White
Write-Host "  4) Exit" -ForegroundColor White

$choice = Read-Host "`nChoice [1-4]"

switch ($choice) {
    "1" {
        if ($hasCli) {
            Add-ContextMenu -MenuText "Auto Rename PDF" -Command "*\shell\AutoRenamePDF" -IconPath $exePath
            Add-ContextMenu -MenuText "Auto Rename PDFs in Folder" -Command "Directory\shell\AutoRenamePDF" -IconPath $exePath
            Write-Host "`n[OK] Context menu installed." -ForegroundColor Green
        } else {
            Write-Host "[!] CLI EXE not found. Cannot install context menu." -ForegroundColor Red
        }
    }
    "2" {
        if ($hasCli) {
            Add-ContextMenu -MenuText "Auto Rename PDF" -Command "*\shell\AutoRenamePDF" -IconPath $exePath
            Add-ContextMenu -MenuText "Auto Rename PDFs in Folder" -Command "Directory\shell\AutoRenamePDF" -IconPath $exePath
            Write-Host "`n[OK] Context menu installed." -ForegroundColor Green
        }
        if ($hasPython) {
            Install-PaddleOCR
        } else {
            Write-Host "[!] Python not found. Skipping PaddleOCR." -ForegroundColor Yellow
        }
    }
    "3" {
        Remove-ContextMenu -Command "*\shell\AutoRenamePDF"
        Remove-ContextMenu -Command "Directory\shell\AutoRenamePDF"
        Write-Host "`n[OK] Context menu removed." -ForegroundColor Green
    }
    "4" { exit 0 }
    default { Write-Host "Invalid choice." -ForegroundColor Red; exit 1 }
}

Write-Host "`nDone." -ForegroundColor Cyan
