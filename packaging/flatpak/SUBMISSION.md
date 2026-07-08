# Flathub Submission Notes
# =============================================================================
# Before submitting to Flathub, complete these steps:
#
# 1. GENERATE CARGO SOURCES
#    pip install flatpak-cargo-generator
#    flatpak-cargo-generator Cargo.lock -o packaging/flatpak/cargo-sources.json
#    Then uncomment the cargo-sources.json line in com.helix.music.yml
#
# 2. ADD SCREENSHOTS
#    Take 2-5 screenshots of the app running and add them to the
#    <screenshots> section of com.helix.music.metainfo.xml
#    Screenshots should be 16:9, at least 1280x720, PNG or WebP.
#
# 3. UPDATE VERSION AND URL
#    In com.helix.music.yml, replace `type: dir` sources with:
#      - type: archive
#        url: https://github.com/netcraker01/helix/archive/refs/tags/v0.1.0.tar.gz
#        sha256: <actual-sha256-of-tarball>
#    Also update the release entry in com.helix.music.metainfo.xml
#
# 4. TEST LOCALLY
#    flatpak-builder --repo=repo --force-clean build-dir packaging/flatpak/com.helix.music.yml
#    flatpak --user remote-add --no-gpg-check helix-repo repo
#    flatpak --user install helix-repo com.helix.music
#    flatpak run com.helix.music
#
# 5. SUBMIT TO FLATHUB
#    Fork https://github.com/flathub/flathub
#    Add com.helix.music.yml (and cargo-sources.json if generated)
#    Open a PR against flathub/flathub
#    Review process: https://docs.flathub.org/docs/for-app-authors/submission/
#
# 6. POST-SUBMISSION
#    After approval, updates are published by pushing to the flathub repo.
#    Use flatpak-external-data-checker for automated version updates.
#
# IMPORTANT: The app auto-downloads yt-dlp on first run into the user's data
# directory. Flatpak's --share=network permission is already included in the
# manifest. No yt-dlp bundling is needed.