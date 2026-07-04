[English](README.md) | [Español](README.es.md)

<p align="center">
  <img src="assets/brand/banner-1eng.png" alt="Banner de Helix Player" width="960">
</p>

<p align="center">
  <b>Reproductor de música de fondo de escritorio para personas que trabajan con música puesta.</b><br>
  Escucha YouTube, SoundCloud y archivos locales sin cuentas, sin suscripciones y sin arrastrar reproducción de vídeo innecesaria.
  <br>
  <small>Software alpha. Primero está hecho para mi flujo diario de trabajo y lo comparto por si también le sirve a alguien más.</small>
</p>

<p align="center">
  <a href="https://github.com/netcraker01/helix/releases"><img src="https://img.shields.io/github/release/netcraker01/helix?style=flat-square&color=00E5FF&label=release" alt="Última versión"></a>
  <a href="https://github.com/netcraker01/helix/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/netcraker01/helix/release.yml?style=flat-square&label=build&color=8A5CFF" alt="Estado del build"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0-D946FF?style=flat-square" alt="Licencia"></a>
  <img src="https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-00E5FF?style=flat-square" alt="Plataformas">
</p>

<p align="center">
  <a href="#por-que-existe-helix"><strong>Por qué existe</strong></a> ·
  <a href="#que-hace-helix"><strong>Qué hace</strong></a> ·
  <a href="#casos-de-uso"><strong>Casos de uso</strong></a> ·
  <a href="#descarga"><strong>Descarga</strong></a> ·
  <a href="#compilar"><strong>Compilar</strong></a>
</p>

---

## Helix

Helix es un reproductor de música de fondo para escritorio pensado para personas que trabajan con música puesta.

Muchos abrimos YouTube solo para escuchar sesiones largas de ambient, focus, lofi, jazz, electrónica o directos mientras trabajamos. El vídeo suele estar oculto, minimizado o detrás del IDE, pero el navegador sigue cargando una experiencia pesada pensada para vídeo.

Helix existe exactamente para ese flujo. Se centra en el audio: YouTube, SoundCloud y archivos locales en una app nativa de escritorio, sin cuentas, sin suscripciones y sin reproducción de vídeo innecesaria.

<a id="por-que-existe-helix"></a>

## Por Qué Existe Helix

Construí Helix porque quería una forma más tranquila de tener música de fondo durante sesiones largas de trabajo.

- No quería tener YouTube abierto solo para escuchar.
- No quería otra suscripción solo para música de fondo.
- Quería YouTube, SoundCloud y archivos locales en un mismo flujo de escritorio.
- Quería algo simple, honesto y centrado en el audio en vez de la distracción.

Helix no intenta ser una plataforma musical universal. Es una herramienta práctica para un problema cotidiano muy concreto: escuchar música de fondo mientras trabajas.

<a id="que-hace-helix"></a>

## Qué Hace Helix

- Reproduce audio desde YouTube.
- Reproduce streams de SoundCloud.
- Reproduce archivos locales.
- Mantiene la reproducción centrada en el audio en vez del vídeo.
- Te da un flujo de escritorio con cola, playlists, favoritos e historial.
- Incluye visualizadores en tiempo real y un modo ambiental cinematográfico si quieres una vista más expresiva del reproductor.

## Casos De Uso

- Programar con sesiones largas de ambient o focus desde YouTube.
- Trabajar en diseño con mezclas de lofi, jazz o electrónica de fondo.
- Escuchar mixes largos de SoundCloud durante sesiones de escritura o investigación.
- Usar archivos locales cuando trabajas sin conexión.
- Poner música de fondo en oficina sin dejar una pestaña del navegador reproduciendo vídeo innecesario.

## Para Quién Es

- Desarrolladores
- Diseñadores
- Escritores
- Sysadmins
- Makers
- Personas que trabajan durante horas con música de fondo
- Personas que quieren YouTube, SoundCloud y archivos locales en un mismo lugar
- Personas que no quieren otra suscripción musical solo para música de concentración

## Qué No Es Helix

- No es un clon de Spotify
- No es una app profesional para DJ
- No es un media center completo
- No es un producto que quiera reemplazar todos los reproductores de música
- No está terminado ni es todavía software de grado producción

Herramientas distintas resuelven trabajos distintos. Helix está hecho para escuchar música de fondo mientras trabajas, especialmente cuando la fuente suele ser video-first pero la necesidad real es audio-first.

## Privacidad Y Cuentas

La privacidad sigue importando, pero no es toda la historia.

- No hace falta una cuenta para el uso básico.
- Helix no añade su propio sistema de tracking.
- Helix no requiere suscripción.
- Helix no añade sus propios anuncios.
- Las fuentes externas pueden tener su propio comportamiento, límites o cambios de disponibilidad.

## Desarrollo Asistido Por IA

Helix está desarrollado con ayuda de IA, pero dirigido y revisado por humanos.

La dirección del producto, los requisitos, la arquitectura y las decisiones de release están lideradas por humanos. La IA ayuda en partes de implementación y documentación, pero el proyecto se mantiene bajo revisión y verificación explícitas.

Si quieres más detalle, mira [docs/ENGINEERING-PROCESS.md](docs/ENGINEERING-PROCESS.md).

---

## Véelo En Acción

<video src="docs/videos/demo.mp4" controls width="100%" poster="docs/screenshots/now-playing.png">
  <img src="docs/videos/demo.gif" alt="Animación demo de Helix">
</video>

Una demo corta de búsqueda, reproducción e interfaz del reproductor.

---

## Capturas

<table>
  <tr>
    <td width="50%">
      <img src="docs/screenshots/home.png" alt="Pantalla principal de Helix">
      <p align="center"><b>Inicio</b> - Empieza rápido y vuelve a lo que ya estabas escuchando.</p>
    </td>
    <td width="50%">
      <img src="docs/screenshots/search-results.png" alt="Resultados de búsqueda de Helix">
      <p align="center"><b>Búsqueda</b> - Busca en YouTube y SoundCloud desde un solo lugar.</p>
    </td>
  </tr>
  <tr>
    <td width="50%">
      <img src="docs/screenshots/now-playing.png" alt="Pantalla de reproducción actual de Helix">
      <p align="center"><b>Ahora suena</b> - Mantén visibles la cola, los controles y el contexto de la pista.</p>
    </td>
    <td width="50%">
      <img src="docs/screenshots/playlists.png" alt="Playlists de Helix">
      <p align="center"><b>Tu biblioteca</b> - Guarda favoritos, playlists e importaciones para más tarde.</p>
    </td>
  </tr>
</table>

---

## Descarga

Elige tu plataforma y prueba la alpha actual:

| Plataforma | Recomendado | Alternativa |
|---|---|---|
| **Linux** | [`.deb` / `.rpm`](https://github.com/netcraker01/helix/releases) | `AppImage`, `.tar.gz` |
| **macOS** | [DMG para Apple Silicon](https://github.com/netcraker01/helix/releases) | El soporte Intel sigue limitado en alpha |
| **Windows** | [NSIS setup.exe](https://github.com/netcraker01/helix/releases) | `.msi` o `.exe` portable |

> **Nota de Windows:** Los instaladores aún no están firmados. Windows 11 puede mostrar una advertencia de SmartScreen. Haz clic en "More info -> Run anyway" para instalar.

> **Nota de Linux:** En esta alpha, `.deb` y `.rpm` son los paquetes Linux recomendados. AppImage está disponible, pero puede tener problemas gráficos o de runtime en algunos entornos Wayland.

Todas las descargas, checksums y notas de versión están en la página de [Releases](https://github.com/netcraker01/helix/releases).

## Compilar

Si quieres compilar Helix tú mismo:

```bash
git clone https://github.com/netcraker01/helix
cd helix
cargo tauri dev
```

Para instrucciones completas de build y empaquetado, mira [docs/BUILDING.md](docs/BUILDING.md).

## Contribuir

Helix es open source y se mantiene en abierto.

- [Reportar un bug](https://github.com/netcraker01/helix/issues/new?template=bug_report.md)
- [Sugerir una feature](https://github.com/netcraker01/helix/issues/new?template=feature_request.md)
- [Leer la guía de contribución](CONTRIBUTING.md)
- [Ver los design tokens](assets/brand/design-tokens.json)

Todos los contribuidores mantienen la propiedad de su trabajo y quedan acreditados en [AUTHORS.md](AUTHORS.md).

## Documentación Para Desarrolladores

- [Compilar desde código fuente](docs/BUILDING.md)
- [Resumen de arquitectura](docs/ARCHITECTURE.md)
- [Estrategia de plataforma](docs/PLATFORM.md)
- [Diseño de UI](docs/UI_DESIGN.md)
- [Guía de empaquetado y releases](docs/packaging.md)
- [Convenciones de release](docs/release-conventions.md)

## Licencia

Helix tiene doble licencia:

- **Open source:** [AGPL-3.0](LICENSE)
- **Comercial:** Disponible para organizaciones que no puedan cumplir AGPL-3.0. Contacta con el propietario del proyecto para más detalles.

Al contribuir, aceptas la [CLA](CLA.md).

---

<p align="center">
  Built with Rust + Tauri v2 + Svelte · Powered by yt-dlp, Symphonia and rustfft
</p>
