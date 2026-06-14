//! Plugin system using WASM runtime.
//!
//! Plugins are compiled to WASM and executed in a sandboxed runtime.
//! The plugin SDK exposes a limited set of APIs for extension.

pub mod runtime;

/// Plugin manifest (declared in plugin's wasm binary).
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub permissions: Vec<String>,
}

/// A loaded and validated plugin instance.
pub struct Plugin {
    pub manifest: PluginManifest,
    // TODO: wasm instance
}

impl Plugin {
    pub fn load(path: &str) -> Result<Self, PluginError> {
        // TODO: read WASM binary, validate, instantiate
        let _ = path;
        Err(PluginError::NotImplemented)
    }
}

#[derive(Debug)]
pub enum PluginError {
    LoadError(String),
    ValidationError(String),
    RuntimeError(String),
    NotImplemented,
}
