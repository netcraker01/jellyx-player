cask "jellyx-player" do
  version "0.2.1"

  # Architecture-specific URLs and checksums.
  # Replace SHA256 placeholders with actual values after the first DMG release.
  # Generate checksums with: shasum -a 256 <dmg-file>

  on_arm do
    url "https://github.com/netcraker01/jellyx-player/releases/download/v#{version}/Jellyx_#{version}_aarch64.dmg",
      verified: "github.com/netcraker01/jellyx-player/"
    sha256 "REPLACE_WITH_AARCH64_SHA256" # arm64 checksum placeholder
  end

  on_intel do
    url "https://github.com/netcraker01/jellyx-player/releases/download/v#{version}/Jellyx_#{version}_x64.dmg",
      verified: "github.com/netcraker01/jellyx-player/"
    sha256 "REPLACE_WITH_X64_SHA256" # Intel checksum placeholder
  end

  name "Jellyx Player"
  desc "Desktop background music player for long work sessions"
  homepage "https://github.com/netcraker01/jellyx-player"

  # The .dmg installs Jellyx.app into /Applications once productName is updated in PR 5.
  depends_on macos: ">= :ventura"

  app "Jellyx.app"

  # yt-dlp is auto-downloaded on first run, but users can also install via brew
  zap trash: [
    "~/Library/Application Support/com.jellyx.music",
    "~/Library/Caches/com.jellyx.music",
    "~/Library/Preferences/com.jellyx.music.plist",
    "~/Library/Saved Application State/com.jellyx.music.savedState",
    # Auto-downloaded yt-dlp binary
    "~/Library/Application Support/jellyx/bin",
  ]

  caveats <<~EOS
    Jellyx Player auto-downloads yt-dlp on first launch for streaming support.
    Alternatively, you can install yt-dlp manually:

      brew install yt-dlp

    The auto-downloaded binary is stored in:
      ~/Library/Application Support/jellyx/bin/
  EOS
end
