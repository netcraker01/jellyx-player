# =============================================================================
# inspect-msi.ps1 — Extract winget metadata from a built MSI
# =============================================================================
# Usage:
#   .\scripts\inspect-msi.ps1 -MsiPath .\Helix_0.1.0_x64_en-US.msi
#
# Outputs:
#   - SHA256 hash of the MSI
#   - ProductCode (from MSI database Property table)
#   - UpgradeCode (from MSI database Upgrade table)
#   - A ready-to-paste snippet for winget installer manifests
#
# Prerequisites:
#   - PowerShell 5.1+ (Windows)
#   - WiX toolset OR lessmsi (for ProductCode extraction)
#   - If neither is available, the script falls back to registry-based extraction
#     after installing the MSI
# =============================================================================

param(
    [Parameter(Mandatory = $true)]
    [string]$MsiPath
)

$ErrorActionPreference = "Stop"

# Resolve full path
$MsiPath = (Resolve-Path -Path $MsiPath).Path

if (-not (Test-Path -Path $MsiPath)) {
    Write-Error "MSI file not found: $MsiPath"
    exit 1
}

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  Helix Player — MSI Metadata Inspector" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# --- SHA256 ---
Write-Host "SHA256:" -ForegroundColor Yellow
$sha256 = (Get-FileHash -Path $MsiPath -Algorithm SHA256).Hash
Write-Host "  $sha256" -ForegroundColor White
Write-Host ""

# --- File info ---
$fileInfo = Get-Item -Path $MsiPath
Write-Host "File:" -ForegroundColor Yellow
Write-Host "  Name:  $($fileInfo.Name)" -ForegroundColor White
Write-Host "  Size:  $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor White
Write-Host ""

# --- ProductCode and UpgradeCode from MSI database ---
Write-Host "MSI Database Properties:" -ForegroundColor Yellow

try {
    # Use Windows Installer COM object to read MSI properties
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $database = $installer.GetType().InvokeMember("OpenDatabase", [System.Reflection.BindingFlags]::InvokeMethod, $null, $installer, @($MsiPath, 0))

    # ProductCode from Property table
    $view = $database.GetType().InvokeMember("OpenView", [System.Reflection.BindingFlags]::InvokeMethod, $null, $database, @("SELECT `Value` FROM `Property` WHERE `Property` = 'ProductCode'"))
    $view.GetType().InvokeMember("Execute", [System.Reflection.BindingFlags]::InvokeMethod, $null, $view, $null)
    $record = $view.GetType().InvokeMember("Fetch", [System.Reflection.BindingFlags]::InvokeMethod, $null, $view, $null)
    $productCode = $record.GetType().InvokeMember("StringData", [System.Reflection.BindingFlags]::GetProperty, $null, $record, @(1))
    Write-Host "  ProductCode:  $productCode" -ForegroundColor Green

    # UpgradeCode from Property table
    $view2 = $database.GetType().InvokeMember("OpenView", [System.Reflection.BindingFlags]::InvokeMethod, $null, $database, @("SELECT `Value` FROM `Property` WHERE `Property` = 'UpgradeCode'"))
    $view2.GetType().InvokeMember("Execute", [System.Reflection.BindingFlags]::InvokeMethod, $null, $view2, $null)
    $record2 = $view2.GetType().InvokeMember("Fetch", [System.Reflection.BindingFlags]::InvokeMethod, $null, $view2, $null)
    $upgradeCode = $record2.GetType().InvokeMember("StringData", [System.Reflection.BindingFlags]::GetProperty, $null, $record2, @(1))
    Write-Host "  UpgradeCode:  $upgradeCode" -ForegroundColor Green

    # ProductVersion
    $view3 = $database.GetType().InvokeMember("OpenView", [System.Reflection.BindingFlags]::InvokeMethod, $null, $database, @("SELECT `Value` FROM `Property` WHERE `Property` = 'ProductVersion'"))
    $view3.GetType().InvokeMember("Execute", [System.Reflection.BindingFlags]::InvokeMethod, $null, $view3, $null)
    $record3 = $view3.GetType().InvokeMember("Fetch", [System.Reflection.BindingFlags]::InvokeMethod, $null, $view3, $null)
    $productVersion = $record3.GetType().InvokeMember("StringData", [System.Reflection.BindingFlags]::GetProperty, $null, $record3, @(1))
    Write-Host "  ProductVersion:  $productVersion" -ForegroundColor Green

    # ProductName
    $view4 = $database.GetType().InvokeMember("OpenView", [System.Reflection.BindingFlags]::InvokeMethod, $null, $database, @("SELECT `Value` FROM `Property` WHERE `Property` = 'ProductName'"))
    $view4.GetType().InvokeMember("Execute", [System.Reflection.BindingFlags]::InvokeMethod, $null, $view4, $null)
    $record4 = $view4.GetType().InvokeMember("Fetch", [System.Reflection.BindingFlags]::InvokeMethod, $null, $view4, $null)
    $productName = $record4.GetType().InvokeMember("StringData", [System.Reflection.BindingFlags]::GetProperty, $null, $record4, @(1))
    Write-Host "  ProductName:  $productName" -ForegroundColor Green

} catch {
    Write-Host "  Could not read MSI database directly." -ForegroundColor Red
    Write-Host "  Install the MSI first, then run:" -ForegroundColor Red
    Write-Host '  Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*" |' -ForegroundColor White
    Write-Host '    Where-Object { $_.DisplayName -like "*Helix*" } |' -ForegroundColor White
    Write-Host '    Select-Object PSChildName, DisplayName, DisplayVersion' -ForegroundColor White
    Write-Host ""
    Write-Host "  For UpgradeCode, run:" -ForegroundColor Red
    Write-Host "  cargo tauri inspect wix-upgrade-code" -ForegroundColor White
    $productCode = "{REPLACE_WITH_WIX_PRODUCT_CODE}"
    $upgradeCode = "{REPLACE_WITH_WIX_UPGRADE_CODE}"
    $productVersion = "0.1.0"
    $productName = "Helix Player"
}

Write-Host ""

# --- Winget manifest snippet ---
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  winget Manifest Snippet" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Copy these values into packaging/winget/manifests/netcraker01.helix-player.installer.yaml:"
Write-Host ""
Write-Host "  InstallerSha256: $sha256"
Write-Host "  ProductCode: $productCode"
Write-Host "  UpgradeCode: $upgradeCode"
Write-Host ""
Write-Host "  InstallerUrl (for GitHub release):"
$version = $productVersion
Write-Host "    https://github.com/netcraker01/helix/releases/download/v${version}/$($fileInfo.Name)"
Write-Host ""