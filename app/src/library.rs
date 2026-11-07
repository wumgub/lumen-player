//! The library as the UI sees it: the shared database plus what's been read
//! out of it, refreshed whenever something changes, and folder scans running
//! on a thread of their own.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

use lumen_core::scanner::{self, Phase, Progress};
use lumen_core::{AlbumRow, ArtistRow, Counts, Library, Playlist, Query, Sort, Track};

/// Where in the library you are, as in the Electron app's `View`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    Songs,
    Artists,
    Albums,
    Stats,
    Artist(String),
    Album(String),
    Playlist(i64, String),
    Folder(String),
}

impl View {
    pub fn default_sort(&self) -> SortState {
        match self {
            View::Album(_) => SortState { key: Sort::Custom, descending: false },
            View::Artist(_) => SortState { key: Sort::Album, descending: false },
            _ => SortState { key: Sort::Title, descending: false },
        }
    }

}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortState {
    pub key: Sort,
    pub descending: bool,
}

pub fn sort_label(s: Sort) -> &'static str {
    match s {
        Sort::Title => "Title",
        Sort::Artist => "Artist",
        Sort::Album => "Album",
        Sort::AddedAt => "Date added",
        Sort::Year => "Year",
        Sort::Duration => "Duration",
        Sort::Custom => "Track order",
    }
}

pub const SORT_OPTIONS: [Sort; 5] = [Sort::Title, Sort::Artist, Sort::Album, Sort::AddedAt, Sort::Duration];

pub struct Scan {
    rx: Receiver<Result<Progress, String>>,
    pub progress: Progress,
}

pub struct LibraryState {
    pub db: Library,
    pub path: PathBuf,
    pub artists: Vec<ArtistRow>,
    pub albums: Vec<AlbumRow>,
    pub playlists: Vec<Playlist>,
    pub folders: Vec<String>,
    pub counts: Counts,
    /// Bumped on every change, so views know to re-query.
    pub version: u64,
    pub scan: Option<Scan>,
    cache: Option<(ListKey, Vec<Track>)>,
}

#[derive(Clone, PartialEq)]
struct ListKey {
    view: View,
    search: String,
    sort: SortState,
    version: u64,
}

impl LibraryState {
    pub fn open(path: PathBuf) -> lumen_core::db::Result<LibraryState> {
        let db = Library::open(&path)?;
        let mut s = LibraryState {
            db,
            path,
            artists: Vec::new(),
            albums: Vec::new(),
            playlists: Vec::new(),
            folders: Vec::new(),
            counts: Counts::default(),
            version: 0,
            scan: None,
            cache: None,
        };
        s.reload();
        Ok(s)
    }

    /// Re-read the lists after any change.
    pub fn reload(&mut self) {
        self.artists = self.db.artists().unwrap_or_default();
        self.albums = self.db.albums().unwrap_or_default();
        self.playlists = self.db.playlists().unwrap_or_default();
        self.folders = self.db.folders().unwrap_or_default();
        self.counts = self.db.counts().unwrap_or_default();
        self.version += 1;
    }

    /// The songs a view lists, cached until something changes.
    pub fn tracks(&mut self, view: &View, search: &str, sort: SortState) -> &[Track] {
        let key = ListKey { view: view.clone(), search: search.to_owned(), sort, version: self.version };
        if self.cache.as_ref().map(|(k, _)| k != &key).unwrap_or(true) {
            let rows = match view {
                View::Playlist(id, _) => self.db.playlist_tracks(*id).unwrap_or_default(),
                _ => {
                    let q = Query {
                        search: (!search.is_empty()).then(|| search.to_owned()),
                        artist: if let View::Artist(a) = view { Some(a.clone()) } else { None },
                        album: if let View::Album(a) = view { Some(a.clone()) } else { None },
                        folder: if let View::Folder(f) = view { Some(f.clone()) } else { None },
                        sort: sort.key,
                        descending: sort.descending,
                        ..Default::default()
                    };
                    self.db.query(&q).unwrap_or_default()
                }
            };
            self.cache = Some((key, rows));
        }
        &self.cache.as_ref().expect("just filled").1
    }

    /// Every track, for things like the Stats view and the song picker.
    pub fn all_tracks(&self) -> Vec<Track> {
        self.db.query(&Query { sort: Sort::Title, ..Default::default() }).unwrap_or_default()
    }

    pub fn start_scan(&mut self) {
        if self.scan.is_some() || self.folders.is_empty() {
            return;
        }
        let (tx, rx) = channel();
        let (path, folders) = (self.path.clone(), self.folders.clone());
        std::thread::Builder::new()
            .name("lumen-scan".into())
            .spawn(move || {
                let lib = match Library::open(&path) {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        return;
                    }
                };
                let res = scanner::scan(&lib, &folders, |p| {
                    let _ = tx.send(Ok(p));
                });
                if let Err(e) = res {
                    let _ = tx.send(Err(e.to_string()));
                }
            })
            .ok();
        self.scan = Some(Scan {
            rx,
            progress: Progress { phase: Phase::Discovering, processed: 0, total: 0, current: None },
        });
    }

    /// Follow a running scan. True when one just finished.
    pub fn poll_scan(&mut self) -> bool {
        let Some(scan) = self.scan.as_mut() else { return false };
        let mut finished = false;
        loop {
            match scan.rx.try_recv() {
                Ok(Ok(p)) => {
                    finished = p.phase == Phase::Done;
                    scan.progress = p;
                }
                Ok(Err(e)) => {
                    log::warn!("scan failed: {e}");
                    finished = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                // The scan thread is gone without saying it was done.
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    finished = true;
                    break;
                }
            }
        }
        if finished {
            self.scan = None;
            self.reload();
        }
        finished
    }
}
