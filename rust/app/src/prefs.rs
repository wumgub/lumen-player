//! The UI's own preferences (theme, visualizer, volume and so on). The
//! Electron app kept these in localStorage; here they live in the library's
//! `settings` table as one JSON value, so they sit with the rest of the data.

use lumen_core::Library;
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
        }
    }
}

impl UiPrefs {
    pub fn load(lib: &Library) -> UiPrefs {
        lib.setting(KEY)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str::<UiPrefs>(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, lib: &Library) {
        if let Ok(s) = serde_json::to_string(self) {
            let _ = lib.set_setting(KEY, &s);
        }
    }
}
