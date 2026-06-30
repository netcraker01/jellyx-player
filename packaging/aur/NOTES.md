# AUR Publishing Notes
# =============================================================================
# Steps to publish helix-player to the Arch User Repository:
#
# 1. PREPARE THE PKGBUILD
#    - Replace sha256sums with the actual checksum of the release tarball:
#      updpkgsums
#    - Verify the build in a clean chroot:
#      extra-x86_64-build  (from devtools package)
#    - Run namcap on both PKGBUILD and the built package:
#      namcap PKGBUILD
#      namcap helix-player-*.pkg.tar.zst
#
# 2. GENERATE .SRCINFO
#    makepkg --printsrcinfo > .SRCINFO
#    .SRCINFO must be committed alongside PKGBUILD in the AUR repo.
#
# 3. CREATE AUR PACKAGE
#    # If this is a new package:
#    ssh aur@aur.archlinux.org setup-repo helix-player
#    # Or via the AUR web interface: https://aur.archlinux.org/packages/new/
#    # Then push:
#    git clone ssh://aur@aur.archlinux.org/helix-player.git aur-helix-player
#    cd aur-helix-player
#    cp ../packaging/aur/PKGBUILD .
#    cp ../packaging/aur/helix-player.install .
#    makepkg --printsrcinfo > .SRCINFO
#    git add PKGBUILD .SRCINFO helix-player.install
#    git commit -m "Initial upload: helix-player 0.1.0"
#    git push
#
# 4. MAINTENANCE
#    - Update PKGBUILD pkgver/pkgrel on each release
#    - Update sha256sums (updpkgsums or manual)
#    - Regenerate .SRCINFO
#    - Push changes to AUR
#
# 5. AUR ACCOUNT REQUIREMENTS
#    - Create an account at https://aur.archlinux.org/register/
#    - Upload your SSH public key at https://aur.archlinux.org/account/
#    - You need a GPG key for signing if using makepkg --sign
#
# NOTE: Helix is licensed under AGPL-3.0 — this is acceptable for AUR
# (AUR permits any OSI-approved license).