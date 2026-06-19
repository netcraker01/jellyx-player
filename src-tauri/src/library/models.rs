//! Library internal models — re-exports persistence entry types for IPC.

pub use crate::persistence::models::{FavoriteEntry, HistoryEntry};

#[allow(unused_imports)]
mod ipc_reexports {
    pub use super::{FavoriteEntry, HistoryEntry};
}
