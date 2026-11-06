//! Folder scanning, like src/main/scanner.ts: walk the folders, read tags with
//! lofty, upsert everything, then prune tracks whose files are gone. Tags are
//! read on all cores; database writes happen in batches on the calling thread.

use std::collections::HashSet;
use std::path::Path;

use lofty::picture::PictureType;
use lofty::prelude::*;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::db::{Library, Result, UpsertTrack};
use crate::models::{Art, is_media_path, is_video_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Discovering,
    Parsing,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub phase: Phase,
    pub processed: usize,
    pub total: usize,
    pub current: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScanResult {
    pub added: usize,
    pub updated: usize,
    pub total: usize,
}

/// Every media file under `folder`, skipping dot-files and dot-folders.
pub fn discover(folder: &Path, out: &mut Vec<String>) {
    let walker = WalkDir::new(folder).into_iter().filter_entry(|e| {
        e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.')
    });
    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            let p = entry.path().to_string_lossy().into_owned();
            if is_media_path(&p) {
                out.push(p);
            }
        }
    }
}

fn stem(path: &str) -> String {
    Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn clean(s: Option<std::borrow::Cow<'_, str>>) -> String {
    s.map(|v| v.trim().to_owned()).unwrap_or_default()
}

/// Read one file's tags. Unreadable files still get a minimal entry so they
/// show up in the library.
pub fn read_track(path: &str) -> UpsertTrack {
    let fallback = UpsertTrack {
        path: path.to_owned(),
        title: stem(path),
        artist: "Unknown Artist".into(),
        is_video: is_video_path(path),
        ..Default::default()
    };
    let Ok(file) = lofty::read_from_path(path) else { return fallback };
    let duration = file.properties().duration().as_secs_f64();
    let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) else {
        return UpsertTrack { duration, ..fallback };
    };
    let title = clean(tag.title());
    let artist = clean(tag.artist());
    let art = tag
        .pictures()
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| tag.pictures().first())
        .filter(|p| !p.data().is_empty())
        .map(|p| Art {
            mime: p.mime_type().map(|m| m.as_str().to_owned()).unwrap_or_else(|| "image/jpeg".into()),
            data: p.data().to_vec(),
        });
    UpsertTrack {
        path: path.to_owned(),
        title: if title.is_empty() { stem(path) } else { title },
        artist: if artist.is_empty() { "Unknown Artist".into() } else { artist },
        album_artist: tag.get_string(ItemKey::AlbumArtist).map(|s| s.trim().to_owned()).unwrap_or_default(),
        album: clean(tag.album()),
        genre: clean(tag.genre()),
        year: tag.date().map(|d| d.year as i64),
        track_no: tag.track().map(i64::from),
        disc_no: tag.disk().map(i64::from),
        duration,
        is_video: is_video_path(path),
        art,
    }
}

/// Scan `folders` and bring the library up to date.
pub fn scan(lib: &Library, folders: &[String], mut progress: impl FnMut(Progress)) -> Result<ScanResult> {
    progress(Progress { phase: Phase::Discovering, processed: 0, total: 0, current: None });
    let mut files = Vec::new();
    for f in folders {
        discover(Path::new(f), &mut files);
    }
    let total = files.len();
    let mut result = ScanResult { total, ..Default::default() };

    for (n, chunk) in files.chunks(64).enumerate() {
        let parsed: Vec<UpsertTrack> = chunk.par_iter().map(|p| read_track(p)).collect();
        for (_, new) in lib.upsert_tracks(&parsed)? {
            if new { result.added += 1 } else { result.updated += 1 }
        }
        let processed = (n * 64 + chunk.len()).min(total);
        progress(Progress {
            phase: Phase::Parsing,
            processed,
            total,
            current: chunk.last().map(|p| Path::new(p).file_name().unwrap_or_default().to_string_lossy().into_owned()),
        });
    }

    // Prune rows whose files are really gone (not just outside the scanned
    // folders). If nothing at all was found, skip it, so an unmounted drive
    // can't wipe the library.
    if !files.is_empty() {
        let seen: HashSet<String> = files.into_iter().collect();
        let mut keep = seen.clone();
        for p in lib.all_paths()? {
            if !seen.contains(&p) && Path::new(&p).exists() {
                keep.insert(p);
            }
        }
        lib.remove_missing(&keep)?;
    }
    progress(Progress { phase: Phase::Done, processed: total, total, current: None });
    Ok(result)
}
