# Diseño de Interfaz y Sistema Visual — Helix Player

Este documento define las guías visuales, el layout y el sistema de diseño para Helix Player, asegurando una implementación coherente en el frontend (Svelte + Tauri).

## 1. División de la Ventana (Layout)

Helix Player utiliza un **Layout Clásico** (tipo Spotify) para garantizar una curva de aprendizaje casi nula y priorizar la intuición del usuario.

El espacio se divide en tres áreas principales:

1. **Sidebar (Barra lateral izquierda)**
   - Fija, no colapsable por defecto.
   - Contiene la navegación principal: Inicio, Buscar, Favoritos.
   - Se utiliza para moverse entre contextos sin perder el estado de reproducción.

2. **Bottom Bar (Barra inferior de controles)**
   - Fija, ocupa el 100% del ancho de la ventana inferior.
   - Contiene los controles principales: Play/Pause, Siguiente/Anterior, Barra de progreso, Volumen.
   - Aquí también estará el acceso para desplegar la vista de "Ahora Suena" y "Cola".

3. **Área Central (Contenido Principal)**
   - Es el espacio dinámico donde se renderizan las vistas (Inicio, Resultados de búsqueda, Detalle de Artista/Álbum).
   - Ocupa el espacio restante entre el Sidebar y la Bottom Bar.

## 2. Temas y Paleta de Colores

Para la `v0.1`, Helix Player utilizará exclusivamente un **Tema Oscuro Estricto (Dark Mode only)**. 

Justificación:
- Mejora drásticamente el contraste para las visualizaciones (FFT, osciloscopios) y las carátulas de los álbumes.
- Reduce la fatiga visual, alineándose con el perfil de usuarios técnicos y entusiastas.

### Estructura de Paleta Base:
- **Fondos (Backgrounds):** Escala de grises muy oscuros (casi negros) para maximizar el contraste. Diferentes elevaciones (Sidebar vs Main vs Bottom Bar) se separarán por sutiles variaciones de gris o bordes finos.
- **Texto:** Blancos rotos (off-white) para texto principal y grises claros para texto secundario (metadatos, tiempos).
- **Acento (Primary Color):** Un color vibrante (ej. verde cyan, morado neón o azul eléctrico) que indique interactividad, estados activos (play) y destaques visuales.

## 3. Tipografía

Helix utilizará un **sistema tipográfico mixto**, combinando fuentes sans-serif para la legibilidad principal y fuentes monoespaciadas para los metadatos técnicos.

- **Fuente principal (Sans-serif):** Se usará para títulos de canciones, nombres de artistas, navegación y textos de interfaz. Debe ser neutra y altamente legible (ej: Inter, Roboto o sistema nativo).
- **Fuente secundaria (Monospace):** Se usará exclusivamente para duraciones de tiempo (ej: 02:34), metadatos técnicos, contadores de frames (FPS) de las visualizaciones o datos del espectrómetro de audio. Esto le otorga a Helix un sutil carácter técnico y de precisión.

## 4. Iconografía

La iconografía utilizará un sistema **Mixto funcional** (outline/solid):

- **Estado inactivo/por defecto:** Se usarán iconos tipo *outline* (líneas o bordes). Esto mantiene la interfaz visualmente ligera y permite que las carátulas y visualizaciones resalten.
- **Estado activo/seleccionado:** Cuando el usuario interactúa (ej. añadir a favoritos, sección de navegación actual, botón de repeat activado), el icono cambiará a su variante *solid* (relleno) y tomará el color de acento.

Se recomienda usar librerías consistentes y modernas como Lucide o Phosphor Icons para Svelte.

## 5. Componentes Clave: Visualizaciones (El "Wow Factor")

Las visualizaciones en tiempo real (impulsadas por `rustfft`) son el diferenciador principal de Helix. En `v0.1`, se integrarán de forma **Mixta (Contextual y Dedicada)**:

1. **Modo Contextual (Ambient Blur / Aurora):**
   - Durante la navegación normal por la app, el color dominante de la carátula o una visualización de baja frecuencia se renderizará de fondo, muy desenfocado. 
   - Aporta una sensación premium y viva a la interfaz sin competir con la legibilidad del texto o las listas de canciones.

2. **Vista Dedicada (Modo Inmersivo / Cine):**
   - Un botón en los controles ("Expandir Visualización") permitirá ocultar el Sidebar y los menús, llevando las visualizaciones (espectrómetro, osciloscopio) a pantalla completa.
   - Es el estado ideal para "dejar el reproductor de fondo" en un monitor secundario, dando todo el protagonismo a la música y al renderizado en tiempo real (OpenGL/WGPU).
