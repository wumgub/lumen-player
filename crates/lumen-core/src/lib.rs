//! Lumen's library: the SQLite database (the same file the Electron app used), folder
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

/// Lumen's data folder. It's where the Electron app kept its data
/// (`app.getPath('userData')`), so an upgrade finds the library in place.
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
