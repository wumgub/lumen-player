//! Edit Metadata: when it's on, edits made in Lumen (titles, artists, albums,
//! covers) are also written into the songs' own files, so other players and
//! devices see them too. A thread does the writing so a big rename never
//! stalls the window. Files that can't be written (say, one Windows has
//! locked because it's playing) are tried again for a while.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

use eframe::egui;
use lumen_core::tags::{self, FileTags};
use lumen_core::{Library, Query, Track};

/// Which songs an edit touched.
pub enum Target {
    Tracks(Vec<i64>),
    Album(String),
    Artist(String),
}

pub struct Metadata {
    tx: Sender<Target>,
    rx: Receiver<Option<String>>,
    /// Why some files haven't been updated, while any are still waiting.
    pub problem: Option<String>,
}

const RETRY: Duration = Duration::from_secs(15);
const TRIES: u32 = 20;

impl Metadata {
    pub fn new(db: PathBuf, ctx: egui::Context) -> Metadata {
        let (tx, targets) = channel();
        let (out, rx) = channel();
        std::thread::Builder::new()
            .name("lumen-metadata".into())
            .spawn(move || worker(&db, &targets, &out, &ctx))
            .ok();
        Metadata { tx, rx, problem: None }
    }

    pub fn write(&self, target: Target) {
        let _ = self.tx.send(target);
    }

    pub fn poll(&mut self) {
        while let Ok(p) = self.rx.try_recv() {
            self.problem = p;
        }
    }
}

fn tracks(lib: &Library, target: Target) -> Vec<Track> {
    match target {
        Target::Tracks(ids) => ids.into_iter().filter_map(|id| lib.track(id).ok().flatten()).collect(),
        Target::Album(album) => lib.query(&Query { album: Some(album), ..Default::default() }).unwrap_or_default(),
        Target::Artist(artist) => lib.query(&Query { artist: Some(artist), ..Default::default() }).unwrap_or_default(),
    }
}

fn file_tags(lib: &Library, t: &Track) -> FileTags {
    // Only an album's own chosen cover is written; one that's just its first
    // song's art is already in that song.
    let cover = if !t.album.is_empty() && lib.album_has_cover(&t.album).unwrap_or(false) {
        lib.album_art(&t.album).ok().flatten()
    } else {
        None
    };
    FileTags {
        title: t.title.clone(),
        artist: t.artist.clone(),
        album_artist: t.album_artist.clone(),
        album: t.album.clone(),
        cover,
    }
}

fn worker(db: &Path, targets: &Receiver<Target>, out: &Sender<Option<String>>, ctx: &egui::Context) {
    let Ok(lib) = Library::open(db) else { return };
    // (track id, tries so far)
    let mut waiting: Vec<(i64, u32)> = Vec::new();
    loop {
        let target = match targets.recv_timeout(if waiting.is_empty() { Duration::MAX } else { RETRY }) {
            Ok(t) => Some(t),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let mut todo: Vec<(i64, u32)> = std::mem::take(&mut waiting);
        if let Some(t) = target {
            for track in tracks(&lib, t) {
                if !todo.iter().any(|w| w.0 == track.id) {
                    todo.push((track.id, 0));
                }
            }
        }
        let mut problem = None;
        for (id, tries) in todo {
            // Read again: a later edit may have changed it since.
            let Some(t) = lib.track(id).ok().flatten() else { continue };
            if t.is_video {
                continue;
            }
            let path = Path::new(&t.path);
            let before = crate::balance::key(path);
            match tags::write(path, &file_tags(&lib, &t)) {
                Ok(()) => {
                    // The sound didn't change, so neither did its loudness.
                    if let (Some(old), Some(new)) = (before, crate::balance::key(path)) {
                        let _ = lib.rekey_loudness(id, &old, &new);
                    }
                }
                Err(e) => {
                    log::warn!("couldn't write tags to {}: {e}", t.path);
                    if tries + 1 < TRIES {
                        waiting.push((id, tries + 1));
                    }
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    problem = Some(format!("{name}: {e}"));
                }
            }
        }
        let problem = problem.map(|p| match waiting.len() {
            0 => format!("Couldn't update a file. {p}"),
            1 => format!("1 file is waiting to be updated. {p}"),
            n => format!("{n} files are waiting to be updated. {p}"),
        });
        let _ = out.send(problem);
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumen_core::Art;
    use lumen_core::db::UpsertTrack;
    use lumen_core::scanner::read_track;

    /// A tenth of a second of silence as a WAV, which lofty tags like an MP3.
    fn wav(path: &Path) {
        let data = 8820u32;
        let mut b = Vec::new();
        for part in [&b"RIFF"[..], &(36 + data).to_le_bytes(), b"WAVEfmt ", &16u32.to_le_bytes()] {
            b.extend_from_slice(part);
        }
        for v in [1u16, 1] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&44100u32.to_le_bytes());
        b.extend_from_slice(&88200u32.to_le_bytes());
        for v in [2u16, 16] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data.to_le_bytes());
        b.resize(b.len() + data as usize, 0);
        std::fs::write(path, b).unwrap();
    }

    #[test]
    fn a_renamed_album_and_its_cover_reach_the_files() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("library.db");
        let lib = Library::open(&db).unwrap();
        for name in ["a", "b"] {
            let path = dir.path().join(format!("{name}.wav"));
            wav(&path);
            let path = path.to_string_lossy().into_owned();
            let (id, _) = lib
                .upsert_track(&UpsertTrack { path: path.clone(), title: name.into(), artist: "Someone".into(), album: "Old".into(), ..Default::default() })
                .unwrap();
            let key = crate::balance::key(Path::new(&path)).unwrap();
            lib.set_loudness(id, Some(-10.0), Some(0.5), &key).unwrap();
        }
        lib.rename_album("Old", "New").unwrap();
        lib.set_album_cover_custom("New", "image/png", b"\x89PNG\r\n\x1a\ncover").unwrap();

        let m = Metadata::new(db.clone(), egui::Context::default());
        m.write(Target::Album("New".into()));
        let problem = m.rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(problem, None);

        for (id, path, key) in lib.loudness_keys().unwrap() {
            let back = read_track(&path);
            assert_eq!(back.album, "New");
            assert_eq!(back.artist, "Someone");
            assert_eq!(back.art, Some(Art { mime: "image/png".into(), data: b"\x89PNG\r\n\x1a\ncover".to_vec() }));
            // Still counts as measured.
            assert_eq!(key, crate::balance::key(Path::new(&path)), "track {id}");
        }
    }
}
