//! Stream source resolution module.
//!
//! The `SourceResolver` trait and `SourceRegistry` live in `jellyx_engine`.
//! Desktop registers concrete implementations (yt-dlp, local scanner, etc.)
//! at startup.

pub mod local;
pub mod soundcloud;
pub mod youtube;
pub mod yt_dlp;

pub use jellyx_engine::source_resolver::{SourceError, SourceRegistry, SourceResolver};
