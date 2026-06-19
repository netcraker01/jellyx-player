//! Local file source module.
//!
//! Provides `LocalResolver` (implements `SourceResolver` trait)
//! and `ScannerService` (walks directories + extracts metadata).

pub mod resolver;
pub mod scanner;

pub use resolver::LocalResolver;
pub use scanner::{ScanResult, ScannerService};
