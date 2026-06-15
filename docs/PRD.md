# PRD — Helix Player

Helix es un reproductor de música open-source para escritorio, orientado a privacidad, libertad del usuario y una experiencia de escucha moderna. Este PRD es un documento vivo: consolida las decisiones ya tomadas y deja visibles los puntos que siguen abiertos.

## Estado

- **Estado**: borrador consolidado
- **Fuente base verificada**: README del repositorio oficial `netcraker01/helix`
- **Inspiración de referencia**: Nuclear
- **Dirección acordada**: Helix no será “solo un Nuclear mejor”, sino una plataforma musical abierta con mejor experiencia base de escucha en el MVP

## Resumen ejecutivo

Helix busca cubrir un hueco claro: una aplicación de escritorio para escuchar música gratis, sin suscripción, sin tracking y sin anuncios propios, con soporte para múltiples fuentes y una experiencia visual atractiva.

Para `v0.1`, el foco NO será ganar por plugins ni por ambición funcional, sino por una **experiencia base de escucha más simple e intuitiva** que Nuclear. El MVP debe validar que existe demanda para una app que combine:

- escucha rápida y estable,
- interfaz clara,
- control real de reproducción,
- soporte inicial para streaming abierto y archivos locales,
- identidad visual propia.

## Problema

El usuario quiere escuchar música de forma cómoda, gratuita y sin anuncios, pero las alternativas actuales suelen imponer al menos uno de estos costes:

- suscripción mensual,
- publicidad,
- rastreo del comportamiento,
- dependencia de una plataforma cerrada.

Además, el mercado deja huecos claros:

- muchos reproductores open-source no ofrecen una experiencia moderna de streaming,
- las visualizaciones en tiempo real casi han desaparecido,
- los sistemas extensibles suelen ser complejos o poco maduros.

## Visión del producto

Construir una plataforma musical abierta para escritorio que permita:

1. escuchar música desde múltiples fuentes,
2. descubrir nueva música,
3. disfrutar visualizaciones en tiempo real,
4. mantener privacidad y control,
5. evolucionar a futuro hacia extensibilidad mediante plugins.

## Propuesta de valor

Helix quiere ser la forma más simple y agradable de escuchar música gratis en escritorio, sin suscripción, sin tracking y sin anuncios propios.

### Posicionamiento del producto

Helix se posiciona como una **plataforma musical abierta** centrada en:

- libertad del usuario,
- privacidad,
- experiencia de escucha,
- extensibilidad futura.

No debe comunicarse como un simple “Spotify killer” ni como “solo un Nuclear mejor”, aunque Nuclear sea una referencia clara de partida.

### Definición operativa de “sin publicidad”

En Helix, “sin publicidad” significa:

- la app no mostrará anuncios propios,
- no habrá banners ni formatos promocionales internos,
- no se promete control absoluto sobre limitaciones o comportamientos de servicios externos.

## Benchmark e identidad frente a Nuclear

Nuclear es la referencia principal de inspiración, pero Helix quiere mejorarla en:

- experiencia base de escucha,
- claridad de interfaz,
- descubrimiento progresivo,
- componente visual,
- apertura futura de la arquitectura.

### Gancho principal del MVP

La mejora principal frente a Nuclear en `v0.1` será:

- **una interfaz más simple e intuitiva para escuchar música**.

## Usuario objetivo

### Usuario principal de `v0.1`

Persona a la que le gusta la música, no quiere pagar una suscripción tipo Spotify, no quiere ser rastreada y quiere escuchar música de forma simple desde una app sin anuncios propios.

### Público primario

- personas que quieren una alternativa sin cuota mensual,
- usuarios que valoran privacidad,
- personas que quieren escuchar música sin fricción,
- usuarios que quieren streaming sin depender de una sola plataforma.

### Público secundario

- usuarios que disfrutan una experiencia visual más expresiva,
- power users que a medio plazo querrán personalización,
- desarrolladores interesados en la extensibilidad futura.

## Objetivos

### Objetivos de `v0.1`

- validar demanda para una experiencia base de escucha mejor resuelta,
- entregar un core player usable, estable y claro,
- ofrecer una UX más simple e intuitiva que Nuclear,
- soportar un set inicial de fuentes atractivo pero controlado,
- sentar una base técnica que permita crecer sin bloquear visualizaciones ni plugins futuros.

### Objetivos a medio plazo

- biblioteca y playlists más completas,
- radio y discovery más profundo,
- visualizaciones avanzadas,
- sistema de plugins visible para usuario final.

## No objetivos de `v0.1`

Quedan fuera del MVP:

- app móvil,
- sincronización multiplataforma compleja,
- red social interna,
- plugin store o instalación de plugins visible,
- radio online,
- Bandcamp,
- sistema avanzado de librería local tipo music manager completo.

## Alcance del MVP (`v0.1`)

### Prioridad principal

Helix debe ganar primero por:

- **experiencia base de escucha simple, rápida y estable**.

### Principios UX del MVP

- lo que está sonando debe estar siempre claro,
- buscar debe ser inmediato,
- reproducir debe requerir el mínimo esfuerzo,
- la navegación debe ser familiar para usuario común,
- la app debe evitar sobrecarga visual innecesaria.

## Navegación y experiencia principal

### Estructura general

El MVP seguirá un patrón **tipo Spotify** con vistas separadas y reproductor persistente.

### Secciones principales

1. **Inicio**
2. **Buscar**
3. **Ahora suena**
4. **Favoritos**

Quedan fuera del primer nivel:

- Cola
- Playlists
- Descubrir
- Radio
- Ajustes

### Happy path principal

1. El usuario abre Helix.
2. Ve inmediatamente qué está sonando.
3. Busca una canción, artista o álbum sin fricción.
4. Entiende los resultados con claridad.
5. Reproduce con un clic.
6. Controla la reproducción y la cola.
7. Puede activar la visualización en tiempo real.

## Definición funcional del MVP

### Inicio

`Inicio` debe mostrar:

1. **Reproducido recientemente**
2. **Recomendaciones / descubrir**
3. **Accesos rápidos por géneros o moods**

Objetivo: servir como punto de retorno musical, no como una copia de `Buscar`.

### Ahora suena

`Ahora suena` debe incluir:

1. **Controles completos de reproducción**
2. **Carátula grande + metadata**
3. **Acciones rápidas**: favorito, compartir, abrir artista/álbum
4. **Visualización en tiempo real**
5. **Cola integrada en la propia vista**

Objetivo: ser el centro visual y operativo del reproductor.

### Cola

La cola no tendrá vista principal propia en `v0.1`; vivirá dentro de `Ahora suena`.

Operaciones iniciales:

- ver próximo tema,
- reordenar manualmente,
- eliminar canciones individuales,
- guardar cola como playlist,
- shuffle,
- repeat.

### Buscar

Capacidades prioritarias:

- buscar por canción,
- buscar por artista,
- buscar por álbum,
- filtrar resultados por tipo.

Presentación de resultados:

- bloques separados por **canciones**, **artistas** y **álbumes**.

Acciones directas sobre resultados:

- reproducir,
- añadir a favoritos,
- añadir a cola.

### Vistas propias de contenido

#### Vista `Artista`

Debe incluir:

- nombre + imagen,
- top canciones,
- álbumes del artista.

#### Vista `Álbum`

Debe incluir:

- portada + título + artista,
- listado de canciones,
- reproducir álbum completo.

## Fuentes y reproducción

### Fuentes iniciales de `v0.1`

Entran en el MVP:

1. **YouTube**
2. **SoundCloud**
3. **Archivos locales**

Quedan fuera, por ahora:

- radio online,
- Bandcamp.

### Archivos locales

El soporte local tendrá un alcance **medio**:

- indexación básica,
- organización mínima por artistas y álbumes,
- integración real con la experiencia general.

### Modelo de indexación local

El usuario seleccionará explícitamente las carpetas que Helix debe escanear.

Esto evita:

- escaneos agresivos del disco o del `home`,
- sorpresas para el usuario,
- complejidad innecesaria en el MVP.

## Plugins

Los plugins **no forman parte de la funcionalidad visible de `v0.1`**.

En el MVP solo se asume:

- base arquitectónica futura,
- separación razonable de responsabilidades,
- decisiones que no bloqueen el sistema de plugins posterior.

### Modelo futuro de permisos

Aunque no entren en `v0.1`, la arquitectura debe prepararse para un modelo de **permisos declarativos (tipo extensiones de navegador)**:

- los plugins declararán qué permisos necesitan (ej: acceso a red para un dominio concreto),
- el usuario deberá aprobarlos explícitamente al instalarlos,
- proporciona un equilibrio óptimo entre seguridad privacy-first y capacidad técnica para extensiones.

El roadmap previsto mantiene plugins como línea fuerte a futuro, no como parte del alcance inmediato.

## Requisitos no funcionales

| Área | Decisión inicial |
|---|---|
| Rendimiento | Reproducción fluida y UI responsiva |
| Audio | Pipeline nativo estable y de baja latencia |
| Privacidad | Sin tracking ni publicidad propia |
| Extensibilidad | Arquitectura que no bloquee plugins futuros |
| Portabilidad | Base para Linux, Windows y macOS |

## Métricas principales del MVP

Las métricas prioritarias para evaluar `v0.1` serán:

1. **Tiempo hasta primera reproducción**
2. **Tasa de reproducción exitosa**
3. **Consumo de CPU/RAM**

### Umbrales iniciales

- **Tiempo hasta primera reproducción**: menos de **5 segundos** en condiciones normales.
- **Tasa de reproducción exitosa**: al menos **90%**.
- **CPU durante reproducción normal con visualización activa**: menos de **20%** en un equipo medio.
- **RAM durante reproducción normal con visualización activa**: menos de **500 MB** en un equipo medio.

### Criterio de eficiencia

Helix priorizará un **equilibrio entre rendimiento y visualización**. No debe vaciar la experiencia visual para ahorrar recursos, pero tampoco degradar la escucha por efectos demasiado costosos.

## Arquitectura de referencia

### Stack acordado para el MVP

| Capa | Tecnología |
|---|---|
| Shell | Tauri v2 |
| Backend | Rust |
| Frontend | Svelte |
| Playback | `symphonia` + `cpal` |
| FFT | `rustfft` |
| Visualización | OpenGL o WGPU |
| Resolución de streams | `yt-dlp` |
| Plugins futuros | WASM runtime |

### Decisiones técnicas clave

- **Svelte** se elige para iterar UX más rápido en el MVP.
- El backend **nativo en Rust** habilita más control sobre reproducción, FFT y rendimiento.
- La arquitectura debe preservar futura extensibilidad, aunque plugins no entren todavía en `v0.1`.

### Diferenciador técnico principal

Helix quiere evitar depender del navegador para el pipeline de audio. Eso habilita:

- FFT real para visualizaciones,
- menor latencia,
- mejor control del pipeline,
- soporte más sólido para formatos y reproducción.

## Roadmap de producto

| Versión | Enfoque |
|---|---|
| `v0.1` | Core player + UX base + búsqueda + cola + favoritos + local files básicos |
| `v0.2` | Biblioteca, playlists, historial y mejoras de organización |
| `v0.3` | Visualizaciones avanzadas |
| `v0.4` | Radio y discovery enriquecido |
| `v0.5` | Sistema de plugins |
| `v1.0` | Empaquetado multiplataforma, auto-updates, i18n, temas más maduros |

## Riesgos principales

- complejidad del pipeline nativo de audio + FFT + visualización,
- dependencia funcional de fuentes externas,
- riesgos legales o de compatibilidad con ciertos servicios,
- sobrecarga de alcance si se adelantan features futuras,
- tensión entre identidad visual y eficiencia de recursos.

### Riesgo detallado: fuentes externas

Helix asume desde el inicio que las fuentes externas representan un riesgo **mixto: técnico y legal**.

#### Riesgo técnico

- cambios en APIs, estructura o disponibilidad de servicios externos,
- rotura de resolutores o degradación de estabilidad,
- diferencias de comportamiento entre fuentes que afecten la experiencia del usuario.

#### Riesgo legal

- posibles tensiones con términos de servicio de terceros,
- ambigüedad sobre usos permitidos en determinados contextos,
- necesidad de comunicar el producto con cuidado para no prometer más de lo que controla.

### Postura de producto frente al riesgo de fuentes externas

Helix adoptará una **postura equilibrada**.

Esto implica:

- comunicar una propuesta de valor fuerte sin lenguaje agresivo innecesario,
- reconocer límites técnicos y dependencias de terceros,
- evitar promesas absolutas sobre comportamiento o disponibilidad de fuentes externas,
- sostener una narrativa clara de libertad del usuario y privacidad sin convertirla en confrontación vacía.

### Mitigaciones iniciales para fuentes externas

Helix debe reflejar al menos estas mitigaciones iniciales:

1. **Diseñar conectores/fuentes desacoplados**
2. **Registrar y monitorizar fallos por fuente**
3. **Poder desactivar una fuente problemática sin romper toda la app**

Estas mitigaciones reducen el riesgo operativo del MVP y mejoran la resiliencia del producto ante cambios o roturas en servicios externos.

### Redacción base de límites del producto

Helix debe comunicar estos límites con un tono **claro para usuario final**.

Mensaje base recomendado para producto/documentación:

> Helix te ofrece una forma abierta y privada de escuchar música desde distintas fuentes, pero algunas funciones pueden depender de servicios externos que pueden cambiar, fallar o dejar de estar disponibles con el tiempo.

Esta línea mantiene honestidad con el usuario sin convertir la comunicación del producto en un texto legalista o defensivo.

## Dependencias relevantes

- Tauri v2
- ecosystem Rust (`symphonia`, `cpal`, `rustfft`)
- `yt-dlp`
- APIs futuras potenciales: Last.fm, MusicBrainz, directorios de radio

## Decisiones abiertas

Quedan abiertas para próximas iteraciones:

1. completar criterios de aceptación por feature del MVP,
2. aterrizar mejor la estrategia de monetización/licenciamiento dentro del PRD,
3. aterrizar mejor riesgos legales y técnicos de fuentes externas.

## Monetización y licenciamiento

### Enfoque inicial de monetización

Helix se orientará inicialmente a un modelo **open-source + donaciones/patrocinios**.

Esto implica que, en esta fase:

- la prioridad es construir producto y comunidad,
- no se fuerza una monetización agresiva dentro de la experiencia del usuario,
- la sostenibilidad económica se apoya primero en apoyo voluntario y patrocinio.

### Principio de producto asociado

La monetización no debe contaminar la propuesta central de Helix:

- sin publicidad propia,
- sin romper privacidad,
- sin degradar la experiencia base de escucha.

### Postura de licenciamiento en el PRD

El PRD debe presentar a Helix como un proyecto **open-source** y, al mismo tiempo, dejar claro que la sostenibilidad futura puede apoyarse en vías compatibles con esa identidad.

Esto implica:

- comunicar la naturaleza abierta del proyecto como parte central de su propuesta,
- separar la experiencia del usuario de la estrategia de sostenibilidad,
- dejar espacio para evolución futura sin convertir el PRD en un documento financiero o legal detallado.

### Visibilidad de donaciones y patrocinios

En esta fase, las donaciones y patrocinios deben comunicarse **fuera de la app**.

Canales apropiados:

- README,
- web del proyecto,
- documentación,
- GitHub Sponsors u otros canales equivalentes.

Esto protege la experiencia principal del usuario y evita contaminar el MVP con elementos de monetización dentro del producto.

## Criterios de aceptación iniciales del MVP

Estos criterios cubren los flujos principales ya definidos del MVP. Aún faltan criterios más detallados para áreas futuras o transversales.

### Feature: Buscar y reproducir

Se considerará aceptado si:

1. la búsqueda devuelve resultados de **canción**, **artista** y **álbum**,
2. los resultados aparecen separados por bloques por tipo,
3. una canción puede reproducirse con un clic,
4. un resultado de artista abre su vista propia.

### Feature: Cola dentro de `Ahora suena`

Se considerará aceptado si:

1. la cola muestra el próximo tema,
2. las canciones pueden reordenarse manualmente,
3. una canción puede eliminarse de la cola,
4. la cola puede guardarse como playlist.

### Feature: `Ahora suena`

Se considerará aceptado si:

1. muestra carátula grande y metadata,
2. permite controles completos de reproducción,
3. muestra visualización en tiempo real,
4. mantiene visible el contexto de reproducción actual.

### Feature: `Inicio`

Se considerará aceptado si:

1. muestra reproducido recientemente,
2. muestra recomendaciones / descubrir,
3. permite retomar reproducción rápidamente,
4. sirve como entrada clara al producto al abrir la app.

### Feature: archivos locales

Se considerará aceptado si:

1. el usuario puede seleccionar carpetas a escanear,
2. Helix indexa solo las carpetas elegidas,
3. la librería local se organiza por artista y álbum,
4. los archivos locales aparecen integrados con la experiencia general.

### Feature: Favoritos

Se considerará aceptado si:

1. una canción puede añadirse a favoritos,
2. Favoritos aparece como sección principal,
3. Favoritos muestra el contenido guardado de forma clara,
4. desde Favoritos puede reproducirse directamente.

## Checklist de completitud

- [x] usuario objetivo priorizado
- [x] problema principal definido
- [x] propuesta de valor resumida
- [x] alcance base del MVP
- [x] navegación principal del MVP
- [x] definición funcional de Inicio, Buscar, Ahora suena y Cola
- [x] fuentes iniciales del MVP
- [x] definición operativa de “sin publicidad”
- [x] frontend del MVP
- [x] métricas principales del MVP
- [~] criterios de aceptación por funcionalidad
- [x] riesgos legales/técnicos más detallados
- [x] monetización/licenciamiento dentro del PRD
- [x] definición futura del sistema de plugins

## Referencias

- Repositorio oficial: `https://github.com/netcraker01/helix`
- README público del repositorio
- Benchmark conceptual: Nuclear
