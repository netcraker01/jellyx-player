# winget Publishing Notes
# =============================================================================
# Steps to publish helix-player to the winget package manager:
#
# 1. VALIDATE MANIFESTS
#    Install the winget CLI and validate locally:
#      winget validate manifests\
#
#    Or use the winget-pkgs repo validation:
#      https://github.com/microsoft/winget-pkgs
#
# 2. GET PRODUCT AND UPGRADE CODES
#    After building the MSI, extract the product code:
#      # PowerShell
#      $msi = "Helix_0.1.0_x64_en-US.msi"
#      (Get-Item $msi).Name  # just to confirm the file
#      # Use lessmsi or dark.exe to extract MSI metadata:
#      lessmsi l $msi
#      # Or install the MSI and check:
#      Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*" |
#        Where-Object { $_.DisplayName -like "*Helix*" } |
#        Select-Object PSChildName, DisplayName, DisplayVersion
#
#    Replace {REPLACE_WITH_WIX_PRODUCT_CODE} and {REPLACE_WITH_WIX_UPGRADE_CODE}
#    with the actual GUIDs from the MSI.
#
# 3. GENERATE SHA256 CHECKSUMS
#    Get-FileHash .\Helix_0.1.0_x64_en-US.msi -Algorithm SHA256
#    Replace REPLACE_WITH_ACTUAL_SHA256_X64 with the output.
#
# 4. SUBMIT TO WINGET-PAKS
#    Fork https://github.com/microsoft/winget-pkgs
#    Create directory: manifests/n/netcraker01/helix-player/0.1.0/
#    Copy the three YAML files there:
#      - netcraker01.helix-player.yaml          (main manifest pointing to version)
#      - netcraker01.helix-player.installer.yaml
#      - netcraker01.helix-player.locale.en-US.yaml
#    Also create the top-level pointer manifest:
#      manifests/n/netcraker01/helix-player.yaml
#    Open a PR against microsoft/winget-pkgs
#
# 5. AUTOMATED UPDATES
#    Consider using https://github.com/vedantmgoyal2009/winget-releaser
#    with GitHub Actions to auto-create winget PRs on release.
#
# 6. WINGET ACCOUNT REQUIREMENTS
#    - A GitHub account (to fork winget-pkgs)
#    - No special Microsoft account needed for submission
#    - First submission may take a few days for review
#
# NOTE: The WebView2 dependency is declared as a PackageDependency.
# Windows 10 1809+ and Windows 11 ship WebView2 by default.
# For older Windows 10, the installer will prompt to install it.