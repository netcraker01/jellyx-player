//! WASM runtime for plugin execution.
//!
//! Uses wasmtime (or wasmi) for sandboxed execution of WASM plugins.
//! Each plugin runs in its own linear memory space.

// use wasmtime::*;

pub struct WasmRuntime {
    // TODO: engine, store, linker
}

impl WasmRuntime {
    pub fn new() -> Self {
        Self {}
    }

    // TODO: instantiate plugin
    // TODO: call plugin hooks (on_track_change, on_render, etc.)
    // TODO: resource limits (memory, CPU time)
}
