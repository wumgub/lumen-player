//! Album art as textures. Images are decoded and shrunk on a worker thread
//! (with its own database connection), so scrolling a big library never waits
//! on a JPEG. Textures are cached per size and the least recently used go once
//! the cache is full.
//!
//! The worker also makes what the Glass theme needs from the playing song's
//! art: a heavily blurred copy for the window backdrop, and two accent colours.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use lumen_core::Library;

use crate::palette::{ArtAccents, extract_accents};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ArtKey {
    Track(i64),
    Album(String),
}

enum Job {
    Thumb { key: ArtKey, px: u32 },
    Backdrop { track: i64 },
}

enum Done {
    Thumb { key: ArtKey, px: u32, image: Option<ColorImage> },
    Backdrop { track: i64, image: Option<ColorImage>, accents: ArtAccents },
}

enum Slot {
    Loading,
    Ready(TextureHandle),
    Missing,
}

struct Entry {
    slot: Slot,
    used: u64,
}

pub struct Backdrop {
    pub track: i64,
    pub texture: Option<TextureHandle>,
    pub accents: ArtAccents,
}

pub struct ArtCache {
    entries: HashMap<(ArtKey, u32), Entry>,
    jobs: Sender<Job>,
    done: Receiver<Done>,
    frame: u64,
    pub backdrop: Option<Backdrop>,
    wanted_backdrop: Option<i64>,
}

const MAX_TEXTURES: usize = 700;

/// The texture size to request for something shown `points` wide.
fn bucket(points: f32, ppp: f32) -> u32 {
    let px = (points * ppp).ceil().max(16.0) as u32;
    px.next_power_of_two().min(1024)
}

impl ArtCache {
    pub fn new(db: PathBuf, ctx: egui::Context) -> ArtCache {
        let (jobs, rx) = channel::<Job>();
        let (tx, done) = channel::<Done>();
        std::thread::Builder::new()
            .name("lumen-art".into())
            .spawn(move || worker(db, rx, tx, ctx))
            .ok();
        ArtCache { entries: HashMap::new(), jobs, done, frame: 0, backdrop: None, wanted_backdrop: None }
    }

    /// Take finished work. Call once per frame; true if anything new arrived.
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        self.frame += 1;
        let mut any = false;
        while let Ok(d) = self.done.try_recv() {
            any = true;
            match d {
                Done::Thumb { key, px, image } => {
                    let slot = match image {
                        Some(img) => Slot::Ready(ctx.load_texture(format!("art-{key:?}-{px}"), img, TextureOptions::LINEAR)),
                        None => Slot::Missing,
                    };
                    self.entries.insert((key, px), Entry { slot, used: self.frame });
                }
                Done::Backdrop { track, image, accents } => {
                    if self.wanted_backdrop == Some(track) {
                        let texture = image.map(|img| ctx.load_texture("backdrop", img, TextureOptions::LINEAR));
                        self.backdrop = Some(Backdrop { track, texture, accents });
                    }
                }
            }
        }
        if self.entries.len() > MAX_TEXTURES {
            let mut by_age: Vec<_> = self.entries.iter().map(|(k, e)| (e.used, k.clone())).collect();
            by_age.sort_by_key(|(used, _)| *used);
            for (_, k) in by_age.into_iter().take(self.entries.len() - MAX_TEXTURES * 3 / 4) {
                self.entries.remove(&k);
            }
        }
        any
    }

    /// The texture for `key` at about `points` wide, or None while it loads
    /// (or if there's no art; see [`ArtCache::missing`]).
    pub fn get(&mut self, key: &ArtKey, points: f32, ppp: f32) -> Option<TextureHandle> {
        let px = bucket(points, ppp);
        let frame = self.frame;
        if let Some(e) = self.entries.get_mut(&(key.clone(), px)) {
            e.used = frame;
            return match &e.slot {
                Slot::Ready(t) => Some(t.clone()),
                _ => None,
            };
        }
        self.entries.insert((key.clone(), px), Entry { slot: Slot::Loading, used: frame });
        let _ = self.jobs.send(Job::Thumb { key: key.clone(), px });
        None
    }

    /// Known to have no art (so draw the placeholder rather than wait).
    pub fn missing(&self, key: &ArtKey, points: f32, ppp: f32) -> bool {
        matches!(self.entries.get(&(key.clone(), bucket(points, ppp))), Some(Entry { slot: Slot::Missing, .. }))
    }

    /// Ask for the playing song's backdrop and accents (no-op if current).
    pub fn want_backdrop(&mut self, track: Option<i64>) {
        if self.wanted_backdrop == track {
            return;
        }
        self.wanted_backdrop = track;
        match track {
            Some(t) => {
                let _ = self.jobs.send(Job::Backdrop { track: t });
            }
            None => self.backdrop = None,
        }
    }

    /// Forget everything, after a rescan or a cover change.
    pub fn invalidate(&mut self) {
        self.entries.clear();
        let t = self.wanted_backdrop.take();
        self.want_backdrop(t);
    }

    /// Forget one album's cover, after it changed.
    pub fn invalidate_album(&mut self, album: &str) {
        self.entries.retain(|(k, _), _| !matches!(k, ArtKey::Album(a) if a == album));
    }
}

fn decode(bytes: &[u8], px: u32) -> Option<ColorImage> {
    let img = image::load_from_memory(bytes).ok()?;
    // Square, cropped to fill, like `object-fit: cover`.
    let thumb = img.resize_to_fill(px, px, image::imageops::FilterType::Triangle).to_rgba8();
    Some(ColorImage::from_rgba_unmultiplied([px as usize, px as usize], thumb.as_raw()))
}

fn backdrop(bytes: &[u8]) -> Option<(ColorImage, ArtAccents)> {
    let img = image::load_from_memory(bytes).ok()?;
    let small = img.resize_to_fill(48, 48, image::imageops::FilterType::Triangle).to_rgba8();
    let accents = extract_accents(small.as_raw(), 48, 48);
    // A tiny, well-blurred image stretched over the window reads as a smooth
    // frosted wash, and costs nothing to draw.
    let blurred = image::imageops::blur(&image::DynamicImage::ImageRgba8(small).to_rgba8(), 6.0);
    Some((ColorImage::from_rgba_unmultiplied([48, 48], blurred.as_raw()), accents))
}

fn worker(db: PathBuf, jobs: Receiver<Job>, done: Sender<Done>, ctx: egui::Context) {
    let Ok(lib) = Library::open(&db) else { return };
    while let Ok(job) = jobs.recv() {
        let result = match job {
            Job::Thumb { key, px } => {
                let art = match &key {
                    ArtKey::Track(id) => lib.art(*id).ok().flatten(),
                    ArtKey::Album(a) => lib.album_art(a).ok().flatten(),
                };
                Done::Thumb { image: art.and_then(|a| decode(&a.data, px)), key, px }
            }
            Job::Backdrop { track } => {
                let made = lib.art(track).ok().flatten().and_then(|a| backdrop(&a.data));
                let (image, accents) = match made {
                    Some((i, a)) => (Some(i), a),
                    None => (None, ArtAccents::default()),
                };
                Done::Backdrop { track, image, accents }
            }
        };
        if done.send(result).is_err() {
            return;
        }
        ctx.request_repaint();
    }
}
