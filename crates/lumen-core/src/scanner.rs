//! Folder scanning, like src/main/scanner.ts: walk the folders, read tags with
//! lofty, upsert everything, then prune tracks whose files are gone. Tags are
//! read on all cores; database writes happen in batches on the calling thread.

use std::collections::{HashMap, HashSet};
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
    /// Tracks whose file turned up somewhere else and kept their history.
    pub moved: usize,
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

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// How many folder names two paths share, counting up from the file.
fn shared_folders(a: &str, b: &str) -> usize {
    let parts = |p: &str| p.split(['/', '\\']).rev().skip(1).map(str::to_owned).collect::<Vec<_>>();
    parts(a).iter().zip(parts(b).iter()).take_while(|(x, y)| x == y).count()
}

/// Move track `id` to the one candidate in the most similarly named folders, if
/// there is exactly one.
fn pick<'a>(lib: &Library, id: i64, old: &str, candidates: Vec<&'a str>, taken: &mut HashSet<&'a str>) -> Result<bool> {
    let best = candidates.iter().map(|c| shared_folders(old, c)).max();
    let top: Vec<&str> = candidates.into_iter().filter(|c| Some(shared_folders(old, c)) == best).collect();
    if let [only] = top[..] {
        lib.move_track(id, only)?;
        taken.insert(only);
        return Ok(true);
    }
    Ok(false)
}

fn same_length(known: f64, found: f64) -> bool {
    known <= 0.0 || found <= 0.0 || (known - found).abs() < 1.0
}

/// Moving or renaming folders in Finder would otherwise look like some songs
/// vanished and new ones appeared, and deleting the old rows takes their plays
/// and playlist spots with them. Before the upsert, pair each track whose file
/// is gone with a newly found file: the same file name (and length, when both
/// are known), or failing that the same title and length. Ties go to the file
/// in the most similarly named folders; a tie there is left alone. Paired
/// tracks just get their new path. Files parsed here go into `parsed`.
fn match_moves(lib: &Library, files: &[String], parsed: &mut HashMap<String, UpsertTrack>) -> Result<usize> {
    if files.is_empty() {
        return Ok(0);
    }
    let index = lib.path_index()?;
    let known: HashSet<&str> = index.iter().map(|(_, p, _, _)| p.as_str()).collect();
    let gone: Vec<_> = index.iter().filter(|(_, p, _, _)| !Path::new(p).exists()).collect();
    if gone.is_empty() {
        return Ok(0);
    }
    let fresh: Vec<&String> = files.iter().filter(|f| !known.contains(f.as_str())).collect();
    if fresh.is_empty() {
        return Ok(0);
    }
    parsed.extend(fresh.par_iter().map(|f| ((*f).clone(), read_track(f))).collect::<Vec<_>>());

    let mut taken: HashSet<&str> = HashSet::new();
    let mut moved = 0;
    let mut left = Vec::new();
    for &&(id, ref old, ref title, duration) in &gone {
        let name = file_name(old);
        let candidates: Vec<&str> = fresh
            .iter()
            .map(|f| f.as_str())
            .filter(|f| !taken.contains(f) && file_name(f) == name && same_length(duration, parsed[*f].duration))
            .collect();
        if pick(lib, id, old, candidates, &mut taken)? {
            moved += 1;
        } else {
            left.push((id, old, title, duration));
        }
    }
    // Renamed files: only when the length is known, so two untitled clips can't swap.
    for (id, old, title, duration) in left {
        if duration <= 0.0 {
            continue;
        }
        let candidates: Vec<&str> = fresh
            .iter()
            .map(|f| f.as_str())
            .filter(|f| {
                let t = &parsed[*f];
                !taken.contains(f) && t.duration > 0.0 && (t.duration - duration).abs() < 1.0
                    && t.title.to_lowercase() == title.to_lowercase()
            })
            .collect();
        if pick(lib, id, old, candidates, &mut taken)? {
            moved += 1;
        }
    }
    if moved > 0 {
        log::info!("scan: {moved} moved file(s) kept their history");
    }
    Ok(moved)
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
    let mut parsed_early = HashMap::new();
    result.moved = match_moves(lib, &files, &mut parsed_early)?;

    for (n, chunk) in files.chunks(64).enumerate() {
        let parsed: Vec<UpsertTrack> =
            chunk.par_iter().map(|p| parsed_early.get(p).cloned().unwrap_or_else(|| read_track(p))).collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moved_files_keep_their_plays_and_playlists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Music");
        let write = |rel: &str| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, b"not really audio").unwrap();
        };
        write("A/song.mp3");
        write("B/song.mp3");
        write("C/other.mp3");
        let lib = Library::open_in_memory().unwrap();
        let folders = vec![root.to_string_lossy().into_owned()];
        assert_eq!(scan(&lib, &folders, |_| {}).unwrap().added, 3);
        let id_of = |rel: &str| {
            let p = root.join(rel).to_string_lossy().into_owned();
            lib.path_index().unwrap().into_iter().find(|t| t.1 == p).map(|t| t.0)
        };
        let a = id_of("A/song.mp3").unwrap();
        let b = id_of("B/song.mp3").unwrap();
        let c = id_of("C/other.mp3").unwrap();
        lib.record_play(a, 100.0, true).unwrap();
        let mix = lib.create_playlist("Mix").unwrap();
        lib.add_to_playlist(mix, &[b, c]).unwrap();

        // Everything goes one folder deeper, and one file is renamed (unknown
        // length, so it can't be matched by title and starts fresh).
        std::fs::rename(&root, dir.path().join("tmp")).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::rename(dir.path().join("tmp"), root.join("Music")).unwrap();
        std::fs::rename(root.join("Music/C/other.mp3"), root.join("Music/C/renamed.mp3")).unwrap();

        let r = scan(&lib, &folders, |_| {}).unwrap();
        assert_eq!((r.moved, r.added), (2, 1));
        assert_eq!(id_of("Music/A/song.mp3"), Some(a));
        assert_eq!(id_of("Music/B/song.mp3"), Some(b));
        assert_eq!(lib.play_history().unwrap().len(), 1);
        let ids: Vec<i64> = lib.playlist_tracks(mix).unwrap().iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![b]);
    }
}
