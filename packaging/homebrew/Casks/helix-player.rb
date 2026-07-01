cask "helix-player" do
  version "0.1.0"

  # Architecture-specific URLs and checksums.
  # Replace SHA256 placeholders with actual values after the first DMG release.
  # Generate checksums with: shasum -a 256 <dmg-file>

  on_arm do
    url "https://github.com/netcraker01/helix/releases/download/v#{version}/Helix_#{version}_aarch64.dmg",
      verified: "github.com/netcraker01/helix/"
    sha256 "REPLACE_WITH_AARCH64_SHA256" # arm64 checksum placeholder
  end

  on_intel do
    url "https://github.com/netcraker01/helix/releases/download/v#{version}/Helix_#{version}_x64.dmg",
      verified: "github.com/netcraker01/helix/"
    sha256 "REPLACE_WITH_X64_SHA256" # Intel checksum placeholder
  end

  name "Helix Player"
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
    Helix Player auto-downloads yt-dlp on first launch for streaming support.
    Alternatively, you can install yt-dlp manually:

      brew install yt-dlp

    The auto-downloaded binary is stored in:
      ~/Library/Application Support/helix/bin/
  EOS
end