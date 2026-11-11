//! The UI's own preferences (theme, visualizer, volume and so on). The
//! Electron app kept these in localStorage; here they live in the library's
//! `settings` table as one JSON value, so they sit with the rest of the data.

use std::collections::HashMap;
use std::path::Path;

use lumen_core::Library;
use rusty_leveldb::LdbIterator;
use lumen_core::theme::{self, Tokens};
use serde::{Deserialize, Serialize};

const KEY: &str = "uiPrefs";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UiPrefs {
    pub theme: String,
    pub custom: Tokens,
    /// Glass (and Blackout) take their accents from the album art.
    pub adaptive: bool,
    pub viz: String,
    pub viz_on: bool,
    /// 0 means as fast as the screen refreshes.
    pub viz_fps: u32,
    /// 0 to 1.5.
    pub volume: f64,
    /// Show a music video's picture rather than its art.
    pub video_mode: bool,
    /// The CPU / GPU / memory readout in the corner.
    pub debug_overlay: bool,
    /// Play every song at about the same loudness.
    pub balance_volume: bool,
    /// Write edits made in Lumen into the songs' files too.
    pub edit_metadata: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        UiPrefs {
            theme: "neon".into(),
            custom: theme::preset("neon"),
            adaptive: true,
            viz: "bars".into(),
            viz_on: true,
            viz_fps: 0,
            volume: 0.9,
            video_mode: true,
            debug_overlay: false,
            balance_volume: true,
            edit_metadata: false,
        }
    }
}

impl UiPrefs {
    pub fn load(lib: &Library) -> UiPrefs {
        if let Some(s) = lib.setting(KEY).ok().flatten() {
            return serde_json::from_str::<UiPrefs>(&s).unwrap_or_default();
        }
        // First run after the Electron app: take its settings along.
        let prefs = lumen_core::data_dir()
            .and_then(|d| from_electron(&d.join("Local Storage").join("leveldb")))
            .unwrap_or_default();
        prefs.save(lib);
        prefs
    }

    pub fn save(&self, lib: &Library) {
        if let Ok(s) = serde_json::to_string(self) {
            let _ = lib.set_setting(KEY, &s);
        }
    }
}

/// Read the Electron app's localStorage (Chromium keeps it in LevelDB). Works
/// on a copy, so the original is never opened for writing.
fn from_electron(dir: &Path) -> Option<UiPrefs> {
    if !dir.is_dir() {
        return None;
    }
    let tmp = std::env::temp_dir().join(format!("lumen-ls-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).ok()?;
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        if e.file_name() != "LOCK" {
            let _ = std::fs::copy(e.path(), tmp.join(e.file_name()));
        }
    }
    // The installed app runs from file://; a dev build from a localhost
    // server has its own, usually stale, copy.
    let (mut installed, mut other) = (HashMap::new(), HashMap::new());
    if let Ok(mut db) = rusty_leveldb::DB::open(&tmp, rusty_leveldb::Options::default())
        && let Ok(mut it) = db.new_iter()
    {
        while it.advance() {
            if let Some((k, v)) = it.current()
                && let Some((name, value)) = local_storage_entry(&k, &v)
            {
                let into = if k.starts_with(b"_file://\0") { &mut installed } else { &mut other };
                into.insert(name, value);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    let items = if installed.is_empty() { other } else { installed };
    (!items.is_empty()).then(|| from_items(&items))
}

/// Keys look like `_<origin>\0\x01<name>`; values start with 0 (UTF-16LE)
/// or 1 (Latin-1).
fn local_storage_entry(key: &[u8], value: &[u8]) -> Option<(String, String)> {
    let at = key.windows(2).position(|w| w == [0, 1])?;
    let name = String::from_utf8(key[at + 2..].to_vec()).ok()?;
    if !key.starts_with(b"_") || !name.starts_with("lumen.") {
        return None;
    }
    let text = match value.split_first()? {
        (0, rest) => String::from_utf16_lossy(&rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>()),
        (1, rest) => rest.iter().map(|&b| b as char).collect(),
        _ => return None,
    };
    Some((name, text))
}

/// The same reading rules the Electron app used.
fn from_items(items: &HashMap<String, String>) -> UiPrefs {
    let mut p = UiPrefs::default();
    let get = |k: &str| items.get(k).map(String::as_str);
    if let Some(t) = get("lumen.theme").filter(|t| !t.is_empty()) {
        p.theme = t.to_string();
    }
    p.adaptive = get("lumen.adaptive") != Some("false");
    if let Some(v) = get("lumen.viz").filter(|v| matches!(*v, "bars" | "waveform" | "radial")) {
        p.viz = v.to_string();
    }
    p.viz_on = get("lumen.viz-off") != Some("1");
    p.viz_fps = match get("lumen.viz-fps").and_then(|n| n.parse::<u32>().ok()) {
        Some(n @ (15 | 30)) => n,
        _ => 0,
    };
    if let Some(v) = get("lumen.volume").and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite() && *v > 0.0) {
        p.volume = v.min(1.5);
    }
    p.video_mode = get("lumen.video-mode") != Some("0");
    if let Some(saved) = get("lumen.custom").and_then(|s| serde_json::from_str::<HashMap<String, String>>(s).ok()) {
        p.custom.extend(saved);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_electron_values() {
        let mut items = HashMap::new();
        for (k, v) in [
            ("lumen.theme", "custom"),
            ("lumen.adaptive", "false"),
            ("lumen.viz", "waveform"),
            ("lumen.viz-fps", "30"),
            ("lumen.volume", "0.65"),
            ("lumen.video-mode", "0"),
            ("lumen.custom", r##"{"accent":"#ff0000"}"##),
        ] {
            items.insert(k.to_string(), v.to_string());
        }
        let p = from_items(&items);
        assert_eq!(p.theme, "custom");
        assert!(!p.adaptive && p.viz_on && !p.video_mode);
        assert_eq!((p.viz.as_str(), p.viz_fps, p.volume), ("waveform", 30, 0.65));
        assert_eq!(p.custom.get("accent").map(String::as_str), Some("#ff0000"));
        assert!(p.custom.len() > 1);
    }

    #[test]
    fn decodes_keys_and_values() {
        let key = b"_file://\x00\x01lumen.theme";
        assert_eq!(local_storage_entry(key, b"\x01glass"), Some(("lumen.theme".into(), "glass".into())));
        let utf16: Vec<u8> = std::iter::once(0).chain("retro".encode_utf16().flat_map(u16::to_le_bytes)).collect();
        assert_eq!(local_storage_entry(key, &utf16).unwrap().1, "retro");
        assert_eq!(local_storage_entry(b"META:file://", b"x"), None);
    }
}

#[cfg(test)]
mod real {
    /// `LUMEN_LS=<a copy of Local Storage/leveldb> cargo test -p lumen real -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn reads_a_real_profile() {
        let dir = std::env::var("LUMEN_LS").expect("LUMEN_LS");
        let p = super::from_electron(std::path::Path::new(&dir)).expect("nothing read");
        println!("{p:#?}");
    }
}
