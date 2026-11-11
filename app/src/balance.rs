//! Balance volume's measuring: a thread that works out every song's loudness
//! once (lumen_playback::loudness) and saves it with the track, so the player
//! can turn each song up or down to the same level. Songs about to play go
//! first. A song is measured again only if its file changes.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::UNIX_EPOCH;

use eframe::egui;
use lumen_core::{Library, Track};
use lumen_playback::loudness;

enum Cmd {
    /// Look for unmeasured songs and measure them.
    Run,
    /// Measure these first if they still need it.
    First(Vec<i64>),
}

pub enum Msg {
    Measured { track_id: i64, gain: f64 },
    Progress { done: usize, total: usize },
}

pub struct Balance {
    tx: Sender<Cmd>,
    rx: Receiver<Msg>,
    /// (done, total) while measuring.
    pub progress: Option<(usize, usize)>,
}

/// The player's gain for a measured track.
pub fn gain_of(t: &Track) -> Option<f64> {
    Some(loudness::gain(t.loudness?, t.peak.unwrap_or(1.0)))
}

/// What a file looked like when it was measured: size and modified time.
fn key(path: &Path) -> Option<String> {
    let m = std::fs::metadata(path).ok()?;
    let modified = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(format!("{}:{modified}", m.len()))
}

impl Balance {
    pub fn new(db: PathBuf, ctx: egui::Context) -> Balance {
        let (tx, cmds) = channel();
        let (out, rx) = channel();
        std::thread::Builder::new()
            .name("lumen-balance".into())
            .spawn(move || worker(&db, &cmds, &out, &ctx))
            .ok();
        Balance { tx, rx, progress: None }
    }

    pub fn run(&self) {
        let _ = self.tx.send(Cmd::Run);
    }

    pub fn first(&self, ids: Vec<i64>) {
        let _ = self.tx.send(Cmd::First(ids));
    }

    /// New gains since the last call, as (track id, gain).
    pub fn poll(&mut self) -> Vec<(i64, f64)> {
        let mut gains = Vec::new();
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Measured { track_id, gain } => gains.push((track_id, gain)),
                Msg::Progress { done, total } => self.progress = (done < total).then_some((done, total)),
            }
        }
        gains
    }
}

fn worker(db: &Path, cmds: &Receiver<Cmd>, out: &Sender<Msg>, ctx: &egui::Context) {
    let Ok(lib) = Library::open(db) else { return };
    let mut first: Vec<i64> = Vec::new();
    while let Ok(cmd) = cmds.recv() {
        match cmd {
            Cmd::First(ids) => {
                first = ids;
                continue;
            }
            Cmd::Run => {}
        }
        // Everything whose file doesn't match what was measured.
        let mut todo: Vec<(i64, String, String)> = lib
            .loudness_keys()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(id, path, measured)| {
                let now = key(Path::new(&path))?;
                (measured.as_deref() != Some(now.as_str())).then_some((id, path, now))
            })
            .collect();
        let total = todo.len();
        let mut done = 0;
        while !todo.is_empty() {
            for cmd in cmds.try_iter() {
                if let Cmd::First(ids) = cmd {
                    first = ids;
                }
            }
            let i = first.iter().find_map(|f| todo.iter().position(|t| t.0 == *f)).unwrap_or(0);
            let (id, path, key) = todo.remove(i);
            let measured = loudness::measure(Path::new(&path));
            let _ = lib.set_loudness(id, measured.map(|l| l.lufs), measured.map(|l| l.peak), &key);
            if let Some(l) = measured {
                let _ = out.send(Msg::Measured { track_id: id, gain: loudness::gain(l.lufs, l.peak) });
            }
            done += 1;
            let _ = out.send(Msg::Progress { done, total });
            ctx.request_repaint();
        }
    }
}
