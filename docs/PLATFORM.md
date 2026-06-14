# Helix Platform Strategy

> Análisis detallado de viabilidad multiplataforma.

---

## 🎯 Estrategia general

**Desktop first. Mobile as v2.0.**

Helix comparte un **core en Rust** entre todas las plataformas, pero cada plataforma tiene su propio **audio backend** y **capa de UI**.

```
                ┌──────────────────────────────┐
                │      Rust Core (común)        │
                │  ─ búsqueda y resolución      │
                │  ─ gestión de playlists       │
                │  ─ metadatos (MusicBrainz)     │
                │  ─ FFT analysis               │
                │  ─ plugin runtime (WASM)       │
                │  ─ Last.fm scrobbling         │
                └──────┬──────┬──────┬──────────┘
                       │      │      │
        ┌──────────────┘      │      └──────────────┐
        ▼                     ▼                     ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│  Desktop UI    │   │   Android UI  │   │    iOS UI      │
│  (Tauri+Svelte)│   │ (Tauri+Svelte)│   │ (Tauri+Svelte) │
│               │   │               │   │               │
│ Audio: cpal   │   │ Audio: Oboe   │   │ Audio: AVAudio │
│ + symphonia   │   │ + symphonia   │   │ + symphonia    │
│               │   │               │   │               │
│ GPU: WGPU     │   │ GPU: WGPU     │   │ GPU: Metal     │
│               │   │               │   │               │
│ VID: yt-dlp   │   │ VID: API proxy│   │ VID: API proxy │
└───────────────┘   └───────────────┘   └───────────────┘
```

---

## 🖥️ Desktop (v1.0): Viabilidad COMPLETA

### Windows
| Componente | Estado | Notas |
|---|---|---|
| Tauri v2 | ✅ 100% | MSI/EXE installer. WebView2 (incluido en Win 11) |
| symphonia | ✅ 100% | Decodificación WASAPI via cpal |
| cpal | ✅ 100% | Host WASAPI. Baja latencia |
| WGPU | ✅ 100% | Backend DX12 |
| yt-dlp | ✅ 100% | CLI embebido o DLL |
| AppImage/MSI | ✅ | Tauri bundler lo soporta |

### macOS
| Componente | Estado | Notas |
|---|---|---|
| Tauri v2 | ✅ 100% | DMG installer. WebView nativo (WKWebView) |
| symphonia | ✅ 100% | Decodificación CoreAudio via cpal |
| cpal | ✅ 100% | Host CoreAudio |
| WGPU | ✅ 100% | Backend Metal. Apple Silicon native |
| yt-dlp | ✅ 100% | Funciona en macOS |
| Notarization | ✅ | Posible con cuenta de Apple ($99/yr) |

### Linux
| Componente | Estado | Notas |
|---|---|---|
| Tauri v2 | ✅ 100% | AppImage, .deb, .rpm, Flatpak |
| symphonia | ✅ 100% | Decodificación ALSA/PulseAudio/JACK via cpal |
| cpal | ✅ 100% | Host PulseAudio (default), ALSA, JACK |
| WGPU | ✅ 100% | Backend Vulkan |
| yt-dlp | ✅ 100% | Native. En repos de todas las distros |
| AppImage | ✅ | Universal Linux binary |

---

## 📱 Mobile (v2.0): Viabilidad CONDICIONADA

### Android

| Componente | Estado | Notas |
|---|---|---|
| Tauri v2 mobile | ⚠️ Posible | Tauri v2 tiene soporte Android. APK/AAB |
| symphonia | ✅ 100% | Rust puro. Compila a ARM64 |
| cpal | ⚠️ Soporte limitado | cpal tiene backend Android pero está verde |
| **Oboe** | ✅ Alternativa recomendada | Google Oboe + cxx bridge desde Rust |
| WGPU | ⚠️ Posible | Backend Vulkan. Funciona en Android 10+ |
| yt-dlp | ❌ **No funciona** | Requiere Python. No hay Python en Android sin termux |
| Stream resolution | ⚠️ Necesita proxy | Solución: servidor remoto o reimplementación |

**Problemas específicos de Android:**

1. **yt-dlp** — No hay Python runtime en Android nativo. Soluciones:
   - A: Servidor proxy (el usuario corre yt-dlp en otro lado) ❌ Mala UX
   - B: Reimplementar resolución en Rust (yt-dlp-api) ⚠️ Mucho trabajo
   - C: Usar APIs nativas de YouTube/rest. ⚠️ Rate limits, frágil
   - D: Bundlear Python + yt-dlp en el APK ❌ APK de 200+ MB

2. **Background playback** — Android mata procesos en background. Hay que usar `foreground service` con notificación persistente.

3. **Audio focus** — Android tiene un sistema de "audio focus" que hay que manejar.

4. **Visualizaciones en mobile** — La batería sufre con WGPU corriendo. Tal vez limitar FPS o desactivar en mobile.

### iOS

| Componente | Estado | Notas |
|---|---|---|
| Tauri v2 mobile | ⚠️ Posible | Tauri v2 tiene soporte iOS |
| symphonia | ✅ 100% | Rust puro. Compila a ARM64 |
| cpal | ⚠️ Limitado | Backend iOS existe pero limitado |
| **AVAudioEngine** | ✅ Alternativa recomendada | Bridge Rust→AVAudioEngine via objc |
| WGPU | ⚠️ Posible | Backend Metal |
| yt-dlp | ❌ **No funciona** | Mismo problema que Android |
| App Store | ⚠️ Revisión | Cuenta de Apple Developer ($99/yr) |

**Problemas específicos de iOS:**

1. **yt-dlp** — Mismo problema. Sin Python en iOS.
2. **App Store Review** — Apple revisa cada actualización. Política de streaming de terceros puede ser problema.
3. **Background playback** — Requiere configurar `audio` background mode. Apple puede rechazar si no hay "modo música real".
4. **Sin WGPU backend nativo** — iOS usa Metal. WGPU tiene backend Metal ✅ pero el rendimiento en dispositivos viejos es limitado.
5. **Sin sideloading fácil** — Solo TestFlight o App Store para distribución.

---

## 🔄 Recomendación de roadmap multiplataforma

```
Fase 0: Setup ─────── Desktop (todas) + CI/static analysis
            │
Fase 1: v0.1 ──────── Core player (Desktop)
            │
Fase 2: v0.2-v0.5 ─── Features completas (Desktop)
            │
Fase 3: v1.0 ──────── Production release (Desktop)
            │
Fase 4: Core refactor ─── Abstraer audio backend (trait AudioBackend)
            │               ─── Abstraer stream resolution (trait StreamResolver)
            │               ─── Abstraer UI (compartir stores/lógica)
            │
Fase 5: Android ────── Port (Oboe + proxy/resolver nativo)
            │
Fase 6: iOS ────────── Port (AVAudioEngine + resolver nativo)
```

---

## ⚠️ Riesgos del mobile

| Riesgo | Severidad | Mitigación |
|---|---|---|
| yt-dlp sin Python en mobile | 🔴 Alta | Invertir en reimplementación Rust de resolución de streams |
| Apple rechaza en App Store | 🟡 Media | No usar términos "YouTube downloader". Enfocar en streaming |
| Rendimiento visualizaciones mobile | 🟡 Media | Desactivar por defecto en mobile. GPU toggle |
| Background playback iOS | 🟢 Baja | Apple lo permite con audio background mode y sin abuso |
| Fragmentación Android | 🟡 Media | API 26+ minimum. Probar en 5 dispositivos diferentes |

---

## 🗣️ Veredicto

| Aspecto | Veredicto |
|---|---|
| **Desktop (Win/Mac/Linux)** | ✅ **Completamente viable.** Herramientas maduras. Stack probado. |
| **Android** | ⚠️ **Viable pero con trabajo.** Audio requiere Oboe. yt-dlp requiere reimplementación. |
| **iOS** | ⚠️ **Viable pero restrictivo.** App Store, background modes, sin yt-dlp. |
| **Compartir código** | ✅ 80%+ del Rust core se comparte. Solo cambia audio backend + UI adaptación. |
| **Esfuerzo mobile** | 📊 30-40% adicional del proyecto total. |

> **Conclusión:** Desktop es pan comido con Tauri v2. Mobile es posible pero requiere decisiones arquitectónicas desde el día 1 para no tener que reescribir. Recomiendo abstraer el audio backend como trait desde v0.1 aunque no se use mobile hasta v2.0.
