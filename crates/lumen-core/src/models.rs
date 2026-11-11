//! The shapes the library hands out, matching src/shared/types.ts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<i64>,
    pub track_no: Option<i64>,
    pub disc_no: Option<i64>,
    /// Seconds.
    pub duration: f64,
    pub has_art: bool,
    /// Epoch ms.
    pub added_at: i64,
    /// The user's free-text note; search matches it too.
    pub note: String,
    /// Backed by a video container, e.g. an .mp4 music video.
    pub is_video: bool,
    /// Integrated loudness in LUFS, once Balance volume has measured it.
    pub loudness: Option<f64>,
    /// Sample peak, linear (1.0 = full scale), measured with `loudness`.
    pub peak: Option<f64>,
}

impl Track {
    /// The artist the Artists view groups by: the album artist when there is
    /// one, otherwise the track artist.
    pub fn effective_artist(&self) -> &str {
        if self.album_artist.is_empty() { &self.artist } else { &self.album_artist }
    }
}

/// One listening session for a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayEvent {
    pub track_id: i64,
    /// Epoch ms.
    pub played_at: i64,
    /// Whole seconds actually listened.
    pub seconds: i64,
    /// Crossed the "counts as a play" mark.
    pub counted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistRow {
    pub artist: String,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumRow {
    pub album: String,
    pub artist: String,
    pub count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub tracks: i64,
    pub artists: i64,
    pub albums: i64,
}

/// Image bytes as stored (embedded art or a custom album cover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Art {
    pub mime: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    Title,
    #[default]
    Artist,
    Album,
    AddedAt,
    Year,
    Duration,
    /// The user's manual order within an album.
    Custom,
}

impl Sort {
    fn column(self) -> &'static str {
        match self {
            Sort::Title => "title",
            Sort::Artist | Sort::Custom => "artist",
            Sort::Album => "album",
            Sort::AddedAt => "addedAt",
            Sort::Year => "year",
            Sort::Duration => "duration",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Query {
    pub search: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    /// Tracks whose file is under this folder.
    pub folder: Option<String>,
    pub sort: Sort,
    pub descending: bool,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Query {
    pub(crate) fn order_by(&self) -> String {
        let dir = if self.descending { "DESC" } else { "ASC" };
        match self.sort {
            // User-positioned tracks first (NULL = unset, sorts last).
            Sort::Custom => format!(
                "albumPos IS NULL {dir}, albumPos {dir}, discNo ASC, trackNo ASC, title ASC"
            ),
            s => format!("{} {dir}, album ASC, discNo ASC, trackNo ASC, title ASC", s.column()),
        }
    }
}

/// App-wide settings stored in the `settings` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSettings {
    pub stats_enabled: bool,
    pub discord_enabled: bool,
    pub discord_client_id: String,
    /// Only needed to register external cover art with Discord.
    pub discord_client_secret: String,
    /// Asset key used as the fallback large image.
    pub discord_large_image_key: String,
    /// Look up public cover art online (iTunes) for the presence image.
    pub discord_cover_art: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            stats_enabled: true,
            discord_enabled: false,
            discord_client_id: String::new(),
            discord_client_secret: String::new(),
            discord_large_image_key: "lumen".into(),
            discord_cover_art: true,
        }
    }
}

pub const AUDIO_EXTENSIONS: &[&str] =
    &["mp3", "flac", "wav", "aif", "aiff", "m4a", "aac", "ogg", "oga", "opus"];
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "webm", "mov"];

fn extension(path: &str) -> Option<String> {
    std::path::Path::new(path).extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase())
}

pub fn is_media_path(path: &str) -> bool {
    extension(path).is_some_and(|e| {
        AUDIO_EXTENSIONS.contains(&e.as_str()) || VIDEO_EXTENSIONS.contains(&e.as_str())
    })
}

pub fn is_video_path(path: &str) -> bool {
    extension(path).is_some_and(|e| VIDEO_EXTENSIONS.contains(&e.as_str()))
}
