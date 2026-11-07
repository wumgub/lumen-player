//! The app: everything the UI needs, and the frame-by-frame plumbing (player
//! events, media keys, preferences, the theme, keyboard shortcuts). The views
//! themselves live in `ui/`.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Key, Pos2};
use lumen_core::{AppSettings, Track};
use lumen_playback::analyzer::{Analyzer, Spectrum};
use lumen_playback::player::{Item, Player, PlayerEvent, Snapshot};

use crate::art::ArtCache;
use crate::fonts;
use crate::library::{LibraryState, SortState, View};
use crate::palette::{ArtAccents, Palette};
use crate::prefs::UiPrefs;
use crate::video::VideoSurface;
use crate::viz;

/// Command-line switches, mostly for testing without a person at the keyboard.
#[derive(Default, Clone)]
pub struct Opts {
    pub ao: Option<String>,
    pub screenshot: Option<PathBuf>,
    pub frames: u64,
    pub play: Option<String>,
    pub seek: Option<f64>,
    pub view: Option<String>,
    pub theme: Option<String>,
    pub db: Option<PathBuf>,
    /// Open something at startup: settings, queue, theme-menu, immersive, editor.
    pub open: Option<String>,
}

pub enum RenameKind {
    Artist(String),
    Album(String),
    Playlist(i64),
}

pub enum Dialog {
    Rename { kind: RenameKind, title: &'static str, label: &'static str, text: String, initial: String },
    NewPlaylist { text: String, placeholder: String, add: Vec<i64> },
    /// The song menu's Edit song…: title, artist, album and note together.
    EditSong(Box<SongEdit>),
    AlbumPicker { tracks: Vec<i64>, text: String },
    Cover { album: String },
    CoverFromSong { album: String, text: String },
    DeletePlaylist { id: i64, name: String },
}

pub struct SongEdit {
    pub track: Track,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub note: String,
}

impl SongEdit {
    pub fn new(t: &Track) -> SongEdit {
        SongEdit { track: t.clone(), title: t.title.clone(), artist: t.artist.clone(), album: t.album.clone(), note: t.note.clone() }
    }
}

pub struct ContextMenu {
    pub track: Track,
    pub pos: Pos2,
    pub opened: Instant,
}

#[derive(Default)]
pub struct UpdateCheck {
    pub running: bool,
    pub result: Option<Result<(String, bool, String), String>>,
    pub rx: Option<Receiver<Result<(String, bool, String), String>>>,
}

pub struct LumenApp {
    pub lib: LibraryState,
    pub player: Player,
    events: Receiver<PlayerEvent>,
    pub snap: Snapshot,
    pub video: Option<VideoSurface>,
    pub analyzer: Analyzer,
    analyzer_track: Option<i64>,
    pub spectrum: Spectrum,
    pub viz_state: viz::State,
    pub art: ArtCache,
    pub prefs: UiPrefs,
    prefs_saved: UiPrefs,
    pub settings: AppSettings,
    pub p: Palette,
    palette_key: Option<(String, String, ArtAccents, bool)>,
    mono_fonts: Option<bool>,

    pub view: View,
    pub search: String,
    pub search_open: bool,
    pub sort: SortState,
    pub edit_mode: bool,
    pub selected: HashSet<i64>,
    pub drag_row: Option<usize>,

    pub queue_open: bool,
    pub settings_open: bool,
    pub theme_menu_open: bool,
    pub sort_menu_open: bool,
    pub immersive: bool,
    pub video_full: bool,
    pub theme_editor_open: bool,
    pub dialog: Option<Dialog>,
    pub menu: Option<ContextMenu>,
    pub bulk_playlist_open: bool,
    pub confirm_clear_stats: bool,
    pub update: UpdateCheck,
    pub seek_drag: Option<f64>,
    pub idle_since: Instant,
    pub theme_pill_rect: egui::Rect,
    pub drag_playlist: Option<usize>,
    pub editor: crate::ui::theme_editor::EditorState,
    pub perf: crate::perf::Monitor,
    healed: Option<i64>,
    video_on: Option<bool>,

    pub presence: lumen_discord::Presence,
    presence_key: Option<(Option<i64>, bool, i64)>,
    presence_pos: (f64, Instant),
    presence_resend: Option<Instant>,
    media: Option<souvlaki::MediaControls>,
    media_rx: Receiver<souvlaki::MediaControlEvent>,
    media_tx: std::sync::mpsc::Sender<souvlaki::MediaControlEvent>,
    media_last: Option<(Option<i64>, bool, i64)>,

    pub started: Instant,
    last_frame: Instant,
    pub dt: f32,
    frames: u64,
    opts: Opts,
    shot: (std::sync::mpsc::Sender<Arc<egui::ColorImage>>, Receiver<Arc<egui::ColorImage>>),
}

fn make_media_controls(
    hwnd: Option<*mut std::ffi::c_void>,
    tx: std::sync::mpsc::Sender<souvlaki::MediaControlEvent>,
    ctx: egui::Context,
) -> Option<souvlaki::MediaControls> {
    let mut m = souvlaki::MediaControls::new(souvlaki::PlatformConfig { dbus_name: "lumen", display_name: "Lumen", hwnd }).ok()?;
    m.attach(move |e| {
        let _ = tx.send(e);
        ctx.request_repaint();
    })
    .ok()?;
    Some(m)
}

pub fn item(t: &Track) -> Item {
    Item { track_id: t.id, path: t.path.clone(), duration: t.duration, is_video: t.is_video }
}

impl LumenApp {
    pub fn new(cc: &eframe::CreationContext<'_>, opts: Opts) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let db_path = opts.db.clone().or_else(lumen_core::library_path).ok_or("no data folder")?;
        let lib = LibraryState::open(db_path.clone())?;
        let mut prefs = UiPrefs::load(&lib.db);
        if let Some(t) = &opts.theme {
            prefs.theme = t.clone();
        }
        let settings = lib.db.app_settings().unwrap_or_default();

        let (tx, events) = channel();
        let ctx = cc.egui_ctx.clone();
        let mut extra = Vec::new();
        if let Some(ao) = &opts.ao {
            extra.push(("ao", ao.as_str()));
        }
        let player = Player::new(&extra, prefs.volume, move |e| { let _ = tx.send(e); }, move || ctx.request_repaint())?;

        let video = match (&cc.gl, &cc.get_proc_address) {
            (Some(gl), Some(gpa)) => VideoSurface::new(player.mpv(), gl, gpa.clone(), cc.egui_ctx.clone())
                .map_err(|e| log::error!("video unavailable: {e}"))
                .ok(),
            _ => None,
        };

        let (media_tx, media_rx) = channel();
        // Windows needs the window's handle, which only exists once it's shown;
        // there the controls are made on the first frame instead.
        let media = if cfg!(windows) { None } else { make_media_controls(None, media_tx.clone(), cc.egui_ctx.clone()) };

        let art = ArtCache::new(db_path, cc.egui_ctx.clone());
        let presence = lumen_discord::Presence::new();
        presence.configure(settings.discord_enabled, &settings.discord_client_id, &settings.discord_large_image_key);
        let view = match opts.view.as_deref() {
            Some("artists") => View::Artists,
            Some("albums") => View::Albums,
            Some("stats") => View::Stats,
            Some(v) if v.starts_with("album:") => View::Album(v[6..].to_owned()),
            _ => View::Songs,
        };
        let mut app = LumenApp {
            sort: view.default_sort(),
            view,
            lib,
            player,
            events,
            snap: Snapshot::default(),
            video,
            analyzer: Analyzer::new(),
            analyzer_track: None,
            spectrum: Spectrum::new(),
            viz_state: viz::State::default(),
            art,
            prefs_saved: prefs.clone(),
            prefs,
            settings,
            p: Palette::resolve("neon", &Default::default(), ArtAccents::default(), false),
            palette_key: None,
            mono_fonts: None,
            search: String::new(),
            search_open: false,
            edit_mode: false,
            selected: HashSet::new(),
            drag_row: None,
            queue_open: false,
            settings_open: false,
            theme_menu_open: false,
            sort_menu_open: false,
            immersive: false,
            video_full: false,
            theme_editor_open: false,
            dialog: None,
            menu: None,
            bulk_playlist_open: false,
            confirm_clear_stats: false,
            update: UpdateCheck::default(),
            seek_drag: None,
            idle_since: Instant::now(),
            theme_pill_rect: egui::Rect::NOTHING,
            drag_playlist: None,
            editor: Default::default(),
            perf: Default::default(),
            healed: None,
            video_on: None,
            presence,
            presence_key: None,
            presence_pos: (0.0, Instant::now()),
            presence_resend: None,
            media,
            media_rx,
            media_tx,
            media_last: None,
            started: Instant::now(),
            last_frame: Instant::now(),
            dt: 1.0 / 60.0,
            frames: 0,
            opts,
            shot: channel(),
        };
        app.refresh_theme(&cc.egui_ctx);
        match app.opts.open.as_deref() {
            Some("settings") => app.settings_open = true,
            Some("queue") => app.queue_open = true,
            Some("theme-menu") => app.theme_menu_open = true,
            Some("immersive") => app.immersive = true,
            Some("editor") => app.theme_editor_open = true,
            _ => {}
        }
        if let Some(q) = app.opts.play.clone() {
            let q = q.to_lowercase();
            let all = app.lib.all_tracks();
            if let Some(i) = all.iter().position(|t| t.title.to_lowercase().contains(&q)) {
                if app.opts.open.as_deref() == Some("edit-song") {
                    app.dialog = Some(Dialog::EditSong(Box::new(SongEdit::new(&all[i]))));
                }
                app.play_tracks(&all, i);
                if let Some(s) = app.opts.seek {
                    std::thread::sleep(Duration::from_millis(150));
                    app.player.seek(s);
                }
            }
        }
        Ok(app)
    }

    // ---- playback helpers the views call ----

    pub fn play_tracks(&mut self, tracks: &[Track], start: usize) {
        self.player.play(tracks.iter().map(item).collect(), start);
    }

    pub fn current(&self) -> Option<&Item> {
        self.snap.current.as_ref()
    }

    pub fn current_track(&self) -> Option<Track> {
        self.snap.current.as_ref().and_then(|c| self.lib.db.track(c.track_id).ok().flatten())
    }

    pub fn set_view(&mut self, v: View) {
        if v != self.view {
            self.sort = v.default_sort();
            self.edit_mode = false;
            self.selected.clear();
            self.menu = None;
            self.bulk_playlist_open = false;
            self.sort_menu_open = false;
        }
        self.view = v;
    }

    /// Something in the library changed: re-read it and drop stale art.
    pub fn library_changed(&mut self) {
        self.lib.reload();
    }

    fn refresh_theme(&mut self, ctx: &egui::Context) {
        let accents = self.art.backdrop.as_ref().map(|b| b.accents).unwrap_or_default();
        let custom = lumen_core::theme::export(&self.prefs.custom);
        let key = (self.prefs.theme.clone(), custom, accents, self.prefs.adaptive);
        if self.palette_key.as_ref() != Some(&key) {
            self.p = Palette::resolve(&self.prefs.theme, &self.prefs.custom, accents, self.prefs.adaptive);
            self.p.apply(ctx);
            self.palette_key = Some(key);
        }
        if self.mono_fonts != Some(self.p.mono) {
            ctx.set_fonts(fonts::definitions(self.p.mono));
            self.mono_fonts = Some(self.p.mono);
        }
    }

    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(e) = self.events.try_recv() {
            match e {
                PlayerEvent::Listened { track_id, seconds, counted } => {
                    if self.settings.stats_enabled {
                        let _ = self.lib.db.record_play(track_id, seconds, counted);
                    }
                }
                PlayerEvent::TrackChanged => {}
            }
        }
        self.snap = self.player.snapshot();
        // Some files don't say how long they are; once mpv knows, remember it.
        if let Some(c) = &self.snap.current
            && c.duration <= 0.0
            && self.snap.duration > 0.0
            && self.healed != Some(c.track_id)
        {
            self.healed = Some(c.track_id);
            let _ = self.lib.db.set_duration(c.track_id, self.snap.duration);
            self.library_changed();
        }

        // The analyzer follows whatever's playing.
        let cur = self.snap.current.as_ref().map(|c| c.track_id);
        if cur != self.analyzer_track {
            self.analyzer_track = cur;
            self.analyzer.load(self.snap.current.as_ref().map(|c| std::path::Path::new(&c.path)));
            self.viz_state.reset();
        }
        let wants_backdrop = (self.prefs.theme == "glass" || self.prefs.theme == "blackout") && self.prefs.adaptive
            || self.prefs.theme == "glass";
        self.art.want_backdrop(if wants_backdrop { cur } else { None });
        if !self.snap.has_video_track() {
            self.video_full = false;
        }
        // Only decode video while something shows it (the panel's Video mode or
        // the full-window view); otherwise mpv plays just the sound.
        let want_video = self.snap.has_video_track() && (self.video_full || (self.prefs.video_mode && !self.immersive));
        if self.video_on != Some(want_video) {
            self.video_on = Some(want_video);
            let _ = self.player.mpv().set_string("vid", if want_video { "auto" } else { "no" });
        }

        // Media keys and the OS "Now Playing" widget.
        while let Ok(e) = self.media_rx.try_recv() {
            use souvlaki::MediaControlEvent as M;
            match e {
                M::Toggle => self.player.toggle(),
                M::Play => self.player.set_paused(false),
                M::Pause => self.player.set_paused(true),
                M::Next => self.player.next(),
                M::Previous => self.player.prev(),
                M::SetPosition(souvlaki::MediaPosition(d)) => self.player.seek(d.as_secs_f64()),
                M::Stop => self.player.set_paused(true),
                _ => {}
            }
        }
        let state = (cur, self.snap.paused, (self.snap.position / 5.0) as i64);
        if self.media_last != Some(state) {
            let track_changed = self.media_last.map(|m| m.0) != Some(cur);
            self.media_last = Some(state);
            if let Some(m) = self.media.as_mut() {
                if track_changed {
                    let t = cur.and_then(|id| self.lib.db.track(id).ok().flatten());
                    let _ = m.set_metadata(souvlaki::MediaMetadata {
                        title: t.as_ref().map(|t| t.title.as_str()),
                        artist: t.as_ref().map(|t| t.artist.as_str()),
                        album: t.as_ref().map(|t| t.album.as_str()),
                        duration: t.as_ref().map(|t| Duration::from_secs_f64(t.duration.max(0.0))),
                        cover_url: None,
                    });
                }
                let progress = Some(souvlaki::MediaPosition(Duration::from_secs_f64(self.snap.position.max(0.0))));
                let _ = m.set_playback(if cur.is_none() || self.snap.stopped {
                    souvlaki::MediaPlayback::Stopped
                } else if self.snap.paused {
                    souvlaki::MediaPlayback::Paused { progress }
                } else {
                    souvlaki::MediaPlayback::Playing { progress }
                });
            }
        }

        self.sync_presence();

        if self.lib.poll_scan() {
            self.art.invalidate();
        }
        if self.art.poll(ctx) {
            self.refresh_theme(ctx);
        }
        if let Some(rx) = &self.update.rx
            && let Ok(r) = rx.try_recv()
        {
            self.update.result = Some(r);
            self.update.running = false;
            self.update.rx = None;
        }

        if self.prefs != self.prefs_saved {
            self.prefs.save(&self.lib.db);
            self.prefs_saved = self.prefs.clone();
        }
    }

    /// Re-apply the Discord settings after they change.
    pub fn configure_presence(&mut self) {
        let s = &self.settings;
        self.presence.configure(s.discord_enabled, &s.discord_client_id, &s.discord_large_image_key);
        self.presence_key = None;
    }

    /// Tell Discord about track, play/pause and duration changes, and about
    /// seeks (debounced, so dragging the slider sends one update), but not
    /// the per-second tick: its progress bar runs itself from timestamps.
    fn sync_presence(&mut self) {
        let cur = self.snap.current.as_ref().map(|c| c.track_id);
        let playing = !self.snap.paused;
        let key = (cur, playing, self.snap.duration.round() as i64);
        let (pos, at) = self.presence_pos;
        let expected = if playing { pos + at.elapsed().as_secs_f64() } else { pos };
        if self.presence_key == Some(key) && (self.snap.position - expected).abs() > 2.0 && self.presence_resend.is_none() {
            self.presence_resend = Some(Instant::now() + Duration::from_millis(600));
        }
        let due = self.presence_resend.is_some_and(|t| t <= Instant::now());
        if self.presence_key != Some(key) || due {
            self.presence_key = Some(key);
            self.presence_resend = None;
            self.presence_pos = (self.snap.position, Instant::now());
            let payload = cur.and_then(|id| self.lib.db.track(id).ok().flatten()).filter(|_| !self.snap.stopped).map(|t| lumen_discord::Payload {
                title: t.title,
                artist: t.artist,
                album: t.album,
                playing,
                position: self.snap.position,
                duration: self.snap.duration,
            });
            self.presence.update(payload);
        } else if self.presence_resend.is_none() {
            self.presence_pos = (self.snap.position, Instant::now());
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let mac = cfg!(target_os = "macos");
        let (space, left, right, esc, f) = ctx.input(|i| {
            let m = if mac { i.modifiers.mac_cmd } else { i.modifiers.ctrl };
            (
                i.key_pressed(Key::Space),
                m && i.key_pressed(Key::ArrowLeft),
                m && i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::F) && !i.modifiers.command && !i.modifiers.ctrl && !i.modifiers.alt,
            )
        });
        if space {
            self.player.toggle();
        }
        if left {
            self.player.prev();
        }
        if right {
            self.player.next();
        }
        if esc {
            if self.menu.is_some() || self.dialog.is_some() || self.queue_open || self.settings_open {
                self.menu = None;
                self.dialog = None;
                self.queue_open = false;
                self.settings_open = false;
                self.theme_menu_open = false;
            } else {
                self.immersive = false;
                self.video_full = false;
            }
        }
        if f {
            // F fills the window with the music video when one is showing,
            // otherwise it toggles the immersive visualizer.
            if self.showing_video() {
                self.video_full = !self.video_full;
            } else {
                self.immersive = !self.immersive;
            }
        }
    }

    /// A music video is on screen in the now-playing panel (rather than its art).
    pub fn showing_video(&self) -> bool {
        self.snap.current.as_ref().is_some_and(|c| c.is_video) && self.prefs.video_mode
    }

    /// Feed the analyzer and step the visualizer's numbers for this frame.
    pub fn step_spectrum(&mut self) {
        let pos = self.player.position();
        let ok = !self.snap.paused && self.analyzer.read(pos, &mut self.spectrum.samples);
        if !ok {
            self.spectrum.samples.fill(0.0);
        }
        self.spectrum.analyse(self.dt);
    }

    pub fn viz_colors(&self) -> viz::Colors {
        viz::Colors { accent: self.p.viz_accent, accent2: self.p.viz_accent2, fg: self.p.viz_fg }
    }

    pub fn viz_kind(&self) -> viz::Kind {
        match self.prefs.viz.as_str() {
            "waveform" => viz::Kind::Waveform,
            "radial" => viz::Kind::Radial,
            _ => viz::Kind::Bars,
        }
    }

    fn screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.opts.screenshot.clone() else { return };
        if self.frames == self.opts.frames {
            let tx = self.shot.0.clone();
            ctx.request_screenshot(move |img| {
                let _ = tx.send(img);
            });
        }
        if let Ok(img) = self.shot.1.try_recv() {
            let [w, h] = img.size;
            let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_srgba_unmultiplied()).collect();
            if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, rgba) {
                let _ = buf.save(&path);
            }
            println!(
                "screenshot {} pos={:.2} playing={} video={} level={:.3}",
                path.display(),
                self.snap.position,
                !self.snap.paused,
                self.snap.has_video,
                self.spectrum.frame.level
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

trait SnapshotExt {
    fn has_video_track(&self) -> bool;
}

impl SnapshotExt for Snapshot {
    fn has_video_track(&self) -> bool {
        self.current.as_ref().is_some_and(|c| c.is_video)
    }
}

impl eframe::App for LumenApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();
        self.dt = now.duration_since(self.last_frame).as_secs_f32().clamp(1.0 / 240.0, 0.25);
        self.last_frame = now;
        self.frames += 1;

        #[cfg(windows)]
        if self.media.is_none() && self.frames == 1 {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(h) = frame.window_handle()
                && let RawWindowHandle::Win32(w) = h.as_raw()
            {
                self.media = make_media_controls(Some(w.hwnd.get() as *mut std::ffi::c_void), self.media_tx.clone(), ctx.clone());
            }
        }
        #[cfg(target_os = "macos")]
        if self.frames == 1 {
            inset_window_buttons(frame);
        }
        self.pump(&ctx);
        self.perf.set_enabled(self.prefs.debug_overlay, &ctx);
        self.refresh_theme(&ctx);
        self.shortcuts(&ctx);
        crate::ui::shell(self, ui, frame);

        // Keep the visualizer and progress moving while something plays;
        // otherwise only wake for input and events.
        let animating = !self.snap.paused && self.prefs.viz_on;
        if animating || self.immersive {
            let fps = self.prefs.viz_fps;
            if fps == 0 {
                ctx.request_repaint();
            } else {
                ctx.request_repaint_after(Duration::from_secs_f32(1.0 / fps as f32));
            }
        } else if !self.snap.paused {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        self.screenshot(&ctx);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.player.flush_listen();
        // mpv's renderer has to go before the core does.
        if let (Some(v), Some(gl)) = (self.video.take(), gl) {
            v.destroy(gl);
        }
        self.prefs.save(&self.lib.db);
        self.presence.update(None);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let c: Color32 = self.p.bg;
        [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, 1.0]
    }
}

/// Give the window an empty unified toolbar, which is what makes macOS draw
/// a 52 pt title bar with the window buttons centred in it, in line with
/// Lumen's own title bar (Electron's `hiddenInset`). Without it the buttons
/// sit in a 28 pt strip at the very top.
#[cfg(target_os = "macos")]
fn inset_window_buttons(frame: &eframe::Frame) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSToolbar, NSView, NSWindowToolbarStyle};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let (Ok(handle), Some(mtm)) = (frame.window_handle(), MainThreadMarker::new()) else { return };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else { return };
    // Safety: winit hands out the window's live NSView, and we're on the main thread.
    let view: &NSView = unsafe { h.ns_view.cast().as_ref() };
    if let Some(window) = view.window() {
        let toolbar = NSToolbar::new(mtm);
        window.setToolbar(Some(&toolbar));
        window.setToolbarStyle(NSWindowToolbarStyle::Unified);
    }
}
