# Arquitectura Técnica — Helix Player

Este documento define las decisiones técnicas core y el diseño de sistemas para el MVP (v0.1) de Helix Player (Rust + Tauri + Svelte).

La arquitectura está orientada al caso de uso principal del producto: sesiones largas de música de fondo en escritorio, con reproducción estable, foco en audio y tolerancia a fallos de fuentes externas.

---

## 1. Gestión del Estado (State Management)

Helix Player utiliza un patrón de **Estado Centralizado en Backend (Rust)**.

El núcleo del reproductor (Rust) es la *Fuente de la Verdad (Source of Truth)* absoluta para todo lo relacionado con la reproducción.

- **Rust controla:** El estado de reproducción (Play/Pause), la pista actual, la cola de reproducción completa (Queue), el progreso de la pista y el volumen.
- **Svelte actúa como Cliente "Tonto":** El frontend se limita a suscribirse a los eventos emitidos por Rust. Si el usuario cierra o recarga la ventana del frontend, la música no debe detenerse a menos que el demonio de Rust se cierre. Svelte envía comandos ("play", "pause", "next", "add_to_queue") pero no confía en su propio estado local hasta que Rust emite el evento de confirmación de vuelta.
- **Resiliencia:** Esto evita cortes de audio si el hilo principal de JavaScript (UI) se satura renderizando vistas complejas o bloqueos temporales, algo clave cuando Helix se usa durante horas como reproductor de fondo.

---

## 2. Comunicación Frontend - Backend (Tauri IPC)

### IPC Dual: Comandos + Eventos + Binario

Helix usa un modelo de comunicación **dual** entre Rust y Svelte:

| Tipo | Dirección | Uso | Formato |
|------|-----------|-----|---------|
| **Comandos (invoke)** | Svelte → Rust | Acciones del usuario: play, pause, next, search, add_to_queue, set_volume | JSON (Tauri estándar) |
| **Eventos (emit/listen)** | Rust → Svelte | Estado de reproducción: track_changed, state_changed, queue_updated, progress_tick | JSON (Tauri estándar) |
| **IPC Binario** | Rust → Svelte | Datos FFT para visualizaciones a 60fps | TypedArrays / Uint8Array |

### IPC Binario para FFT

La transferencia de datos FFT/audio entre Rust y Svelte se hace por **IPC binario** usando `TypedArrays`/`Uint8Array`.

**Razón:** Enviar FFT en binario evita el coste de serialización JSON a 60fps. Esto mantiene la UI fluida y permite integrar visualizaciones en Svelte sin romper la arquitectura visual del producto.

**Implementación:**
- Rust escribe datos FFT en un buffer compartido o los envía como bytes crudos vía Tauri IPC binario.
- Svelte recibe los bytes como `Uint8Array` y los interpreta directamente en el canvas WebGL/SVG.
- No hay paso intermedio por JSON — los bytes van directo del motor de audio al renderer visual.

---

## 3. Pipeline de Audio

### Arquitectura con Bus Interno de PCM

Helix usa un pipeline de audio con **bus interno de PCM** para desacoplar la salida de audio y el análisis FFT. La prioridad es que la reproducción siga siendo estable aunque la UI, las visualizaciones o una fuente externa fallen.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────────────┐
│ Source      │────▶│ Decoder     │────▶│ PCM Bus (interno)    │
│ Resolver    │     │ (symphonia) │     │                     │
└─────────────┘     └─────────────┘     ├──────────┬──────────┤
                                        │          │          │
                                        ▼          ▼          ▼
                                   ┌────────┐ ┌────────┐ ┌──────────┐
                                   │ Audio  │ │ FFT    │ │ Futuro:  │
                                   │ Output │ │ Engine │ │ Plugins  │
                                   │ (cpal) │ │        │ │ WASM     │
                                   └────────┘ └────────┘ └──────────┘
                                                     │
                                                     ▼
                                              ┌────────────┐
                                              │ IPC Binario │
                                              │ → Svelte    │
                                              └────────────┘
```

**Ventajas del bus interno de PCM:**
- Una sola decodificación alimenta a la vez la salida de audio, las visualizaciones FFT y futuras extensiones sin duplicar trabajo.
- Extensible: se pueden agregar consumidores al bus sin modificar el pipeline existente.
- Sin la complejidad de un grafo tipo DAW — es un bus simple de pub/sub interno.

**Flujo detallado:**
1. **Source Resolver** determina la fuente (YouTube, SoundCloud, archivo local) y obtiene el stream de audio.
2. **Decoder** (symphonia) decodifica el stream a PCM crudo.
3. **PCM Bus** distribuye los frames PCM a todos los suscriptores registrados.
4. **Audio Output** (cpal) consume PCM y reproduce por el dispositivo de audio del sistema.
5. **FFT Engine** consume PCM, calcula la transformada y envía los datos por IPC binario a Svelte.

---

## 4. Modelos de Datos Core

### 4.1 Track — Modelo Unificado Rico

Helix usa un modelo **unificado rico** para `Track`, con campos comunes y metadata opcional por fuente.

```rust
struct Track {
    id: String,              // UUID interno de Helix
    source: Source,           // Enum: YouTube | SoundCloud | Local
    source_id: String,        // ID en la fuente original (video ID, track ID, path)
    title: String,
    artist: String,
    album: Option<String>,
    duration: Option<f64>,   // Segundos
    thumbnail: Option<String>,// URL o path local
    stream_url: Option<String>,// Resuelto por source al reproducir
    local_path: Option<String>,// Solo para archivos locales
    metadata: HashMap<String, String>, // Metadata adicional por fuente
}
```

**Razón:** Un `Track` unificado simplifica cola, favoritos, búsqueda y vistas de contenido, mientras deja espacio para enriquecer metadata sin reventar el modelo base. Soporta YouTube, SoundCloud y archivos locales sin caer en tipos específicos por fuente demasiado pronto.

### 4.2 Modelos Complementarios

```rust
struct Artist {
    id: String,
    name: String,
    thumbnail: Option<String>,
    source: Source,
    source_id: String,
}

struct Album {
    id: String,
    title: String,
    artist: String,          // ID o nombre del artista
    cover: Option<String>,
    year: Option<u32>,
    source: Source,
    source_id: String,
    tracks: Vec<String>,     // IDs de Tracks
}

enum Source {
    YouTube,
    SoundCloud,
    Local,
}
```

### 4.3 Estrategia de Modelos Híbrida

- **Modelos globales:** `Track`, `Artist`, `Album`, `Source` — compartidos por playback, library, search y UI bridge.
- **DTOs internos por dominio:** Cada módulo (playback, sources, library) define sus propios tipos de datos internos para estado, eventos y operaciones específicas.
- **Razón:** Equilibrar simplicidad inicial y limpieza a largo plazo. Las entidades musicales nucleares son compartidas; el estado interno y DTOs técnicos se quedan dentro de su módulo.

---

## 5. Estructura de Carpetas

### 5.1 Backend (Rust / jellyx-desktop/)

Estructura **híbrida**: dominios principales arriba, submódulos técnicos dentro.

> La lógica de dominio pura (`models/`, `shared/utils`) se extrajo a
> `jellyx-core/` en el workspace split. Ver [`ARCHITECTURE.md`](../ARCHITECTURE.md)
> raíz para el layout del workspace y el boundary del core.

```
jellyx-desktop/
├── src/
│   ├── app/                    # Inicialización y configuración de la app
│   │   ├── mod.rs
│   │   └── setup.rs
│   ├── ipc/                    # Handlers de comandos Tauri (bridge Svelte ↔ Rust)
│   │   ├── mod.rs
│   │   ├── commands.rs         # Comandos invocables desde Svelte
│   │   └── events.rs           # Eventos emitidos hacia Svelte
│   ├── playback/               # Dominio: motor de reproducción
│   │   ├── mod.rs
│   │   ├── service.rs          # Orquestación de reproducción
│   │   ├── state.rs            # Estado de reproducción (Source of Truth)
│   │   ├── events.rs           # Eventos de playback
│   │   └── models.rs           # DTOs internos de playback
│   ├── audio/                  # Pipeline de audio
│   │   ├── mod.rs
│   │   ├── decoder.rs          # Decodificación (symphonia)
│   │   ├── output.rs           # Salida de audio (cpal)
│   │   ├── pipeline.rs        # PCM Bus y orquestación del pipeline
│   │   └── fft.rs              # Motor FFT
│   ├── visualizer/             # Procesamiento de datos para visualizaciones
│   │   ├── mod.rs
│   │   └── fft_bridge.rs       # Envío de FFT por IPC binario
│   ├── sources/                # Fuentes de contenido
│   │   ├── mod.rs              # Registry de fuentes + SourceResolver
│   │   ├── youtube/
│   │   │   ├── mod.rs
│   │   │   ├── resolver.rs     # Resolución de streams
│   │   │   └── search.rs       # Búsqueda en YouTube
│   │   ├── soundcloud/
│   │   │   ├── mod.rs
│   │   │   ├── resolver.rs
│   │   │   └── search.rs
│   │   └── local/
│   │       ├── mod.rs
│   │       ├── resolver.rs     # Lectura de archivos locales
│   │       └── scanner.rs      # Indexación de carpetas
│   ├── library/                # Biblioteca del usuario
│   │   ├── mod.rs
│   │   ├── service.rs          # CRUD de favoritos, historial, playlists
│   │   ├── state.rs
│   │   └── models.rs
│   ├── persistence/            # Almacenamiento persistente
│   │   ├── mod.rs
│   │   └── db.rs               # SQLite o similar
│   ├── errors/                 # Tipos de error centralizados
│   │   ├── mod.rs
│   │   └── types.rs
│   └── main.rs                 # Entry point
├── Cargo.toml
└── tauri.conf.json
```

> **Nota:** `models/` y `shared/` se movieron a `jellyx-core/` en el workspace
> split (PR 3). El árbol arriba refleja solo lo que permanece en
> `jellyx-desktop`. Para los módulos extraídos, ver
> [`jellyx-core/src/`](../jellyx-core/src/).

### 5.2 Frontend (Svelte / ui/)

Estructura **híbrida**: `routes/` para páginas, `features/` para lógica por dominio, `shared/` para piezas reutilizables.

```
ui/
├── src/
│   ├── app/                    # Layout raíz, router, providers
│   │   ├── App.svelte
│   │   └── layout/
│   ├── routes/                 # Páginas (una por sección de navegación)
│   │   ├── Home/
│   │   │   └── +page.svelte
│   │   ├── Search/
│   │   │   └── +page.svelte
│   │   ├── Favorites/
│   │   │   └── +page.svelte
│   │   └── NowPlaying/
│   │       └── +page.svelte
│   ├── features/               # Lógica por dominio
│   │   ├── player/             # Reproductor + visualizador
│   │   │   ├── components/
│   │   │   │   ├── Controls.svelte
│   │   │   │   ├── ProgressBar.svelte
│   │   │   │   ├── Queue.svelte
│   │   │   │   ├── NowPlayingInfo.svelte
│   │   │   │   └── Visualizer.svelte   # Visualizador vive dentro de player
│   │   │   ├── stores/
│   │   │   │   └── player.ts
│   │   │   └── types/
│   │   ├── search/
│   │   │   ├── components/
│   │   │   ├── stores/
│   │   │   └── types/
│   │   ├── favorites/
│   │   │   ├── components/
│   │   │   ├── stores/
│   │   │   └── types/
│   │   └── library/
│   │       ├── components/
│   │       ├── stores/
│   │       └── types/
│   ├── shared/                 # Componentes y utilidades reutilizables
│   │   ├── components/
│   │   │   ├── TrackList.svelte
│   │   │   ├── AlbumCard.svelte
│   │   │   ├── ArtistCard.svelte
│   │   │   └── Sidebar.svelte
│   │   ├── stores/
│   │   │   └── theme.ts
│   │   ├── types/
│   │   │   └── models.ts       # Tipos TypeScript espejo de modelos Rust
│   │   ├── utils/
│   │   ├── constants/
│   │   └── icons/
│   ├── services/               # Comunicación con Tauri/Rust
│   │   ├── tauri.ts            # Bridge Tauri (invoke, listen)
│   │   ├── events.ts           # Suscripciones a eventos de Rust
│   │   └── commands.ts        # Comandos tipados hacia Rust
│   ├── styles/                 # Estilos globales, tema oscuro, tokens
│   │   ├── global.css
│   │   ├── tokens.css
│   │   └── animations.css
│   └── main.ts                 # Entry point Svelte
├── package.json
├── svelte.config.js
├── vite.config.ts
└── tsconfig.json
```

**Nota sobre el Visualizer:** El visualizador vive dentro de `features/player/` (no es feature separada) porque depende totalmente del contexto de reproducción. Esto mantiene el MVP cohesionado.

---

## 6. Flujo End-to-End: Buscar → Reproducir → Visualizar

### 6.1 Búsqueda

1. Usuario escribe consulta en **Search**.
2. Svelte envía comando `search(query)` → Rust.
3. **SourceResolver** consulta todas las fuentes registradas (YouTube, SoundCloud, Local) en paralelo.
4. Cada fuente normaliza sus resultados al modelo `Track` unificado.
5. Rust emite evento `search_results` → Svelte renderiza resultados.

### 6.2 Reproducción desde Búsqueda (Play from Search)

1. Usuario hace clic en "Reproducir" en un resultado.
2. Svelte envía comando `play(track_id)` → Rust.
3. **Rust inicia la reproducción INMEDIATAMENTE** y construye una **cola contextual** con el resto de resultados relevantes.
4. **SourceResolver** resuelve el stream de la pista seleccionada.
5. **Decoder** decodifica a PCM.
6. **PCM Bus** distribuye a **Audio Output** + **FFT Engine**.
7. Rust emite eventos: `track_changed`, `state_changed`, `queue_updated`.
8. Svelte actualiza UI: barra de reproducción, carátula, controles, cola.

**Comportamiento clave:** La reproducción es inmediata. La cola contextual se construye con el resto de resultados de búsqueda para que la escucha continúe naturalmente durante sesiones largas sin exigir interacción constante.

### 6.3 Visualización

1. **FFT Engine** calcula la transformada en tiempo real desde el PCM Bus.
2. Datos FFT se envían por **IPC Binario** → Svelte.
3. **Visualizer.svelte** recibe `Uint8Array` y renderiza en canvas.
4. Dos modos: **Ambient Blur** (durante navegación) y **Modo Cine** (inmersivo, expansivo).

### 6.4 Diagrama de Flujo Completo

```
[Svelte UI]
    │
    ├─ search(query) ────────────────────────────────────────────┐
    │                                                             ▼
    │                                              ┌─────────────────────┐
    │                                              │   Source Resolver    │
    │                                              │ (YouTube/SoundCloud/ │
    │                                              │  Local)              │
    │                                              └────────┬────────────┘
    │                                                       │
    │                                                       ▼
    │                                              ┌─────────────────────┐
    │  ◀── search_results ──────────────────────────│  Normaliza a Track  │
    │                                              └─────────────────────┘
    │
    ├─ play(track_id) ────────────────────────────┐
    │                                               │
    │                                               ▼
    │                                    ┌─────────────────────┐
    │                                    │ Playback Service     │
    │                                    │ • Reproduce inmediato│
    │                                    │ • Cola contextual    │
    │                                    └────────┬────────────┘
    │                                             │
    │                                             ▼
    │                                    ┌─────────────────────┐
    │                                    │ Source Resolver      │
    │                                    │ (resuelve stream)   │
    │                                    └────────┬────────────┘
    │                                             │
    │                                             ▼
    │                                    ┌─────────────────────┐
    │                                    │ Decoder (symphonia)  │
    │                                    └────────┬────────────┘
    │                                             │
    │                                             ▼
    │                                    ┌─────────────────────┐
    │                                    │ PCM Bus              │
    │                                    └──────┬────────┬─────┘
    │                                           │        │
    │                                           ▼        ▼
    │                                    ┌──────────┐ ┌──────────┐
    │                                    │ cpal     │ │ FFT      │
    │                                    │ (audio)  │ │ Engine   │
    │                                    └──────────┘ └────┬─────┘
    │                                                      │
    │  ◀── events (track_changed, ─────────────────────────┤
    │      state_changed, queue_updated)                   │
    │                                                      │
    │  ◀── IPC Binario (Uint8Array FFT) ◀─────────────────┘
    │
    ▼
[UI actualizada]
```

---

## 7. Comportamientos Clave de la Arquitectura

### Resiliencia de Reproducción
- El motor de audio corre en un hilo separado en Rust.
- Si la UI de Svelte se satura, cuelga o se recarga, el audio **no se detiene**.
- Svelte se reconecta a los eventos de Rust al reiniciar.

### Sincronización de Estado
- Svelte **nunca** asume un estado local como definitivo.
- Cada acción del usuario es un comando a Rust; la confirmación viene como evento de vuelta.
- Esto previene inconsistencias entre UI y motor de audio.

### Extensibilidad del Pipeline
- Nuevos consumidores del PCM Bus se agregan sin modificar el pipeline existente.
- Nuevas fuentes se agregan implementando el trait `SourceResolver`.
- El modelo `Track` unificado con `metadata` flexible permite fuentes futuras sin romper la interfaz.

### Plugins (Preparación Arquitectónica, no visibles en v0.1)
- Base para modelo futuro de permisos declarativos tipo extensiones de navegador.
- El bus de PCM ya es un punto de extensión natural para plugins de audio.
- El SourceResolver es un punto de extensión natural para fuentes adicionales.
