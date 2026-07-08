//! `helix-ffi` — skeleton crate exposing `helix-core` through a Foreign
//! Function Interface so other languages can integrate Helix logic.
//!
//! In PR 2 this crate is intentionally empty of logic: it only validates that
//! the `cdylib` / `staticlib` targets build as part of the workspace. It MUST
//! NOT expose any user-facing feature commitments in this slice (spec:
//! consumer-scaffolding). Real FFI surface will be added in a future change.
//!
//! (UniFFI scaffolding, if added later, belongs in a subsequent PR — not in
//! this skeleton slice.)