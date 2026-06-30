# Homebrew Cask for Helix Player
# =============================================================================
# This is a PREPARATION scaffold. Before publishing to a Homebrew tap:
#   1. Replace placeholder values with actual release artifacts and checksums.
#   2. The GitHub release must produce a macOS .dmg artifact.
#   3. Test with: brew install --cask ./packaging/homebrew/Casks/helix-player.rb
#   4. Publish to a custom tap: https://github.com/netcraker01/homebrew-helix
#
# To generate the SHA256 checksum from a release .dmg:
#   shasum -a 256 Helix_0.1.0_aarch64.dmg
#   shasum -a 256 Helix_0.1.0_x64.dmg
# =============================================================================

cask "helix-player" do
  version "0.1.0"
  sha256 :no_check  # REPLACE with actual checksum per architecture

  # REPLACE with actual GitHub release URLs when available
  url "https://github.com/netcraker01/helix/releases/download/v#{version}/Helix_#{version}_aarch64.dmg",
    verified: "github.com/netcraker01/helix/"

  # Uncomment and adjust for Intel Macs if you ship a universal or separate x64 binary:
  # on_intel do
  #   url "https://github.com/netcraker01/helix/releases/download/v#{version}/Helix_#{version}_x64.dmg",
  #     verified: "github.com/netcraker01/helix/"
  #   sha256 "REPLACE_WITH_X64_SHA256"
  # end

  # on_arm do
  #   sha256 "REPLACE_WITH_ARM64_SHA256"
  # end

  name "Helix"
  desc "Privacy-first music player — stream, visualize, discover"
  homepage "https://github.com/netcraker01/helix"

  # The .dmg installs Helix.app into /Applications
  depends_on macos: ">= :ventura"

  app "Helix.app"

  # yt-dlp is auto-downloaded on first run, but users can also install via brew
  zap trash: [
    "~/Library/Application Support/com.helix.music",
    "~/Library/Caches/com.helix.music",
    "~/Library/Preferences/com.helix.music.plist",
    "~/Library/Saved Application State/com.helix.music.savedState",
    # Auto-downloaded yt-dlp binary
    "~/Library/Application Support/helix/bin",
  ]

  caveats <<~EOS
    Helix auto-downloads yt-dlp on first launch for streaming support.
    Alternatively, you can install yt-dlp manually:

      brew install yt-dlp

    The auto-downloaded binary is stored in:
      ~/Library/Application Support/helix/bin/
  EOS
end