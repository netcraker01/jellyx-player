# Install Jellyx on Arch Linux

On Arch Linux and derivatives, install Jellyx from the repository's Arch package recipe. It repackages the official Linux `.deb` executable as a native pacman package, so Jellyx uses the system WebKitGTK and GStreamer plugins instead of the AppImage runtime.

## Install

Install the build tools, clone the repository, and build/install the package:

```bash
sudo pacman -S --needed base-devel git
git clone https://github.com/netcraker01/jellyx-player.git
cd jellyx-player/packaging/aur
makepkg -si
```

`makepkg` verifies the release asset checksums, asks pacman to install missing runtime dependencies, builds the package, and installs it. The desktop entry and `jellyx` terminal command are included.

The recipe is currently maintained in this GitHub repository; it is not yet published to the AUR.

## Runtime dependencies

The package declares these required dependencies:

- `gtk3` and `webkit2gtk-4.1` — native desktop window and WebKit runtime.
- `gst-plugins-base-libs` and `gst-plugins-good` — WebKit playback elements, including `appsrc` and `autoaudiosink`.

For network playback, Jellyx can download `yt-dlp` on first use; installing the `yt-dlp` package separately is optional. PipeWire users should have `pipewire-pulse` installed and running for audio output.

## Update and remove

To update after the repository publishes a newer package recipe:

```bash
cd jellyx-player
git pull --ff-only
cd packaging/aur
makepkg -si
```

To remove Jellyx while keeping its user data:

```bash
sudo pacman -R jellyx-player
```

User data is stored separately under `~/.local/share/jellyx/` and is not removed by this command.
