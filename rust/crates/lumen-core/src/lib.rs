//! Lumen's library: the SQLite database shared with the Electron app, folder
//! scanning, search folding, listening stats and theme tokens.

pub mod db;
pub mod fold;
pub mod models;
pub mod scanner;
pub mod stats;
pub mod theme;

pub use db::Library;
pub use models::*;

use std::path::PathBuf;

/// Where the Electron app keeps its data (`app.getPath('userData')`), which
/// this app shares so the library carries straight over.
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"));
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map(|b| b.join("Lumen"))
}

pub fn library_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("library.db"))
}
