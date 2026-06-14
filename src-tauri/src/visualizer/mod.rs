//! Audio visualization engine.
//!
//! Takes FFT frequency data and renders real-time visualizations
//! using WGPU (cross-platform GPU: Vulkan, Metal, DX12).

pub mod renderer;

pub enum VisualizerMode {
    Spectrum,    // Classic bars
    Oscilloscope, // Waveform
    AlbumArt,    // Album art + glow
    Shader(String), // Custom GLSL shader
}

pub struct VisualizerConfig {
    pub mode: VisualizerMode,
    pub bar_count: usize,
    pub sensitivity: f32,
    pub color_scheme: ColorScheme,
    pub fps: u32,
}

pub struct ColorScheme {
    pub primary: [f32; 4],
    pub secondary: [f32; 4],
    pub background: [f32; 4],
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            mode: VisualizerMode::Spectrum,
            bar_count: 64,
            sensitivity: 1.0,
            color_scheme: ColorScheme {
                primary: [0.0, 0.8, 1.0, 1.0],   // Cyan
                secondary: [0.5, 0.0, 1.0, 1.0], // Purple
                background: [0.05, 0.05, 0.1, 1.0],
            },
            fps: 60,
        }
    }
}
