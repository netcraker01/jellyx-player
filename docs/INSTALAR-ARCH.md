# Instalar Jellyx en Arch Linux

En Arch Linux y sus derivados, instala Jellyx usando la receta de paquete Arch del repositorio. Esta receta empaqueta el ejecutable oficial del `.deb` como un paquete nativo de pacman, de modo que Jellyx usa WebKitGTK y los plugins GStreamer del sistema en lugar del runtime de la AppImage.

## Instalación

Instala las herramientas de compilación, clona el repositorio y construye/instala el paquete:

```bash
sudo pacman -S --needed base-devel git
git clone https://github.com/netcraker01/jellyx-player.git
cd jellyx-player/packaging/aur
makepkg -si
```

`makepkg` verifica los checksums del release, pide a pacman instalar las dependencias runtime que falten, construye el paquete y lo instala. El paquete incluye la entrada del menú y el comando `jellyx` para la terminal.

Por ahora, la receta se mantiene en este repositorio de GitHub; todavía no está publicada en AUR.

## Dependencias runtime

El paquete declara estas dependencias obligatorias:

- `gtk3` y `webkit2gtk-4.1`: ventana de escritorio y runtime WebKit nativos.
- `gst-plugins-base-libs` y `gst-plugins-good`: elementos de reproducción de WebKit, incluidos `appsrc` y `autoaudiosink`.

Para reproducir fuentes en línea, Jellyx puede descargar `yt-dlp` al primer uso; instalar el paquete `yt-dlp` por separado es opcional. En sistemas con PipeWire, debe estar instalado y activo `pipewire-pulse` para la salida de audio.

## Actualizar y desinstalar

Para actualizar cuando el repositorio publique una versión nueva de la receta:

```bash
cd jellyx-player
git pull --ff-only
cd packaging/aur
makepkg -si
```

Para desinstalar Jellyx y conservar sus datos de usuario:

```bash
sudo pacman -R jellyx-player
```

Los datos del usuario se guardan por separado en `~/.local/share/jellyx/` y este comando no los elimina.
