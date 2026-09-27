# AUR publishing checklist for the Arch package recipe in this repository.
# The package currently repacks the official x86_64 .deb release; it does not
# compile Jellyx from source and is not yet published to the AUR.
#
# Before each release update:
#   1. Set pkgver and the official .deb and LICENSE SHA256 values in PKGBUILD.
#   2. Build and inspect in a clean chroot:
#        extra-x86_64-build  # from devtools
#        namcap PKGBUILD jellyx-player-*.pkg.tar.zst
#   3. Regenerate and commit .SRCINFO:
#        makepkg --printsrcinfo > .SRCINFO
#   4. Push PKGBUILD, .SRCINFO, and jellyx-player.install to the AUR Git repo.
#
# AUR publication requires an AUR account and an uploaded SSH key:
#   https://aur.archlinux.org/register/
#   https://aur.archlinux.org/account/
