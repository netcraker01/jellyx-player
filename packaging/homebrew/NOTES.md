# Homebrew Tap Publishing Notes
# =============================================================================
# Steps to create and publish a Homebrew tap for Helix:
#
# 1. CREATE THE TAP REPO
#    Create a GitHub repo named: homebrew-helix
#    (Homebrew taps follow the naming convention: homebrew-<name>)
#    URL: https://github.com/netcraker01/homebrew-helix
#
# 2. ADD THE CASK
#    mkdir -p Casks
#    cp packaging/homebrew/Casks/helix-player.rb Casks/
#    # Update the version, SHA256, and URL with actual release values
#    git add Casks/helix-player.rb
#    git commit -m "Add helix-player cask v0.1.0"
#    git push
#
# 3. TEST LOCALLY
#    brew install --cask ./packaging/homebrew/Casks/helix-player.rb
#    # Or from the tap:
#    brew tap netcraker01/helix
#    brew install --cask helix-player
#
# 4. USERS INSTALL VIA
#    brew tap netcraker01/helix
#    brew install --cask helix-player
#
# 5. UPDATING ON EACH RELEASE
#    - Update version, sha256, and URL in the cask file
#    - Generate SHA256: shasum -a 256 Helix_0.1.0_aarch64.dmg
#    - Commit and push
#
# NOTE: If Helix gains enough popularity, consider submitting to homebrew/cask
# (the official Homebrew cask repo) for `brew install --cask helix-player`
# without needing a custom tap. See: https://docs.brew.sh/Adding-Software-to-Homebrew#casks