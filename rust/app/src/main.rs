//! Lumen, native. For now this is the D0 spike: it proves the risky parts
//! (system fonts with Cyrillic/CJK, a 10k-row list, libmpv audio and video in
//! the window, a visualizer synced to playback, media keys) before the real UI.

mod fonts;
mod video;
mod viz;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, Sense, vec2};
use lumen_playback::analyzer::{Analyzer, Spectrum};
use lumen_playback::mpv::{Event, Format, Mpv, Value};

struct Track {
    title: String,
    artist: String,
    album: String,
    path: PathBuf,
    duration: f64,
}

fn library_db() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"));
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map(|b| b.join("Lumen").join("library.db"))
}

fn load_library() -> Vec<Track> {
    let Some(path) = library_db() else { return Vec::new() };
    let Ok(db) = rusqlite::Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return Vec::new();
    };
    let Ok(mut q) = db.prepare(
        "SELECT title, artist, album, path, duration FROM tracks ORDER BY artist, album, trackNo",
    ) else {
        return Vec::new();
    };
    q.query_map([], |r| {
        Ok(Track {
            title: r.get(0)?,
            artist: r.get(1)?,
            album: r.get(2)?,
            path: PathBuf::from(r.get::<_, String>(3)?),
            duration: r.get(4)?,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

#[derive(Default)]
struct Opts {
    ao: Option<String>,
    screenshot: Option<PathBuf>,
    frames: u64,
    play: Option<String>,
    viz: Option<viz::Kind>,
    seek: Option<f64>,
}

fn parse_args() -> Opts {
    let mut o = Opts { frames: 90, ..Default::default() };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--ao" => o.ao = args.next(),
            "--screenshot" => o.screenshot = args.next().map(PathBuf::from),
            "--frames" => o.frames = args.next().and_then(|v| v.parse().ok()).unwrap_or(90),
            "--play" => o.play = args.next(),
            "--seek" => o.seek = args.next().and_then(|v| v.parse().ok()),
            "--viz" => {
                o.viz = match args.next().as_deref() {
                    Some("waveform") => Some(viz::Kind::Waveform),
                    Some("radial") => Some(viz::Kind::Radial),
                    _ => Some(viz::Kind::Bars),
                }
            }
            _ => {}
        }
    }
    o
}

const P_TIME: u64 = 1;
const P_DURATION: u64 = 2;
const P_PAUSE: u64 = 3;
const P_VIDEO: u64 = 4;

fn hex(s: &str) -> Color32 {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

struct Spike {
    mpv: Arc<Mpv>,
    events: std::sync::mpsc::Receiver<Event>,
    event_thread: Option<JoinHandle<()>>,
    video: Option<VideoSurfaceSlot>,
    analyzer: Analyzer,
    spectrum: Spectrum,
    viz: viz::State,
    viz_kind: viz::Kind,
    tracks: Vec<Track>,
    current: Option<usize>,
    started: Instant,
    last: Instant,
    pos: f64,
    pos_at: Instant,
    dur: f64,
    paused: bool,
    has_video: bool,
    media: Option<souvlaki::MediaControls>,
    media_rx: std::sync::mpsc::Receiver<souvlaki::MediaControlEvent>,
    opts: Opts,
    frames: u64,
    fps: f32,
    pending_seek: Option<f64>,
    shot_tx: std::sync::mpsc::Sender<Arc<egui::ColorImage>>,
    shot_rx: std::sync::mpsc::Receiver<Arc<egui::ColorImage>>,
}

type VideoSurfaceSlot = video::VideoSurface;

impl Spike {
    fn new(cc: &eframe::CreationContext<'_>, opts: Opts) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        cc.egui_ctx.set_fonts(fonts::definitions(false));
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = hex("#0a0a12");
        visuals.window_fill = hex("#11111d");
        visuals.extreme_bg_color = hex("#11111d");
        visuals.selection.bg_fill = hex("#22d3ee").gamma_multiply(0.35);
        visuals.override_text_color = Some(hex("#f0f0ff"));
        cc.egui_ctx.set_visuals(visuals);

        let mut options = vec![
            ("vo", "libmpv"),
            ("hwdec", "auto-safe"),
            ("audio-display", "no"),
            ("idle", "yes"),
            ("volume-max", "150"),
            ("ytdl", "no"),
            ("load-scripts", "no"),
            ("input-default-bindings", "no"),
            ("audio-client-name", "Lumen"),
            ("terminal", "no"),
        ];
        if let Some(ao) = &opts.ao {
            options.push(("ao", ao.as_str()));
        }
        let mpv = Arc::new(Mpv::new(&options)?);
        log::info!("libmpv client API {:?}", Mpv::api_version());
        mpv.observe(P_TIME, "time-pos", Format::Double)?;
        mpv.observe(P_DURATION, "duration", Format::Double)?;
        mpv.observe(P_PAUSE, "pause", Format::Flag)?;
        mpv.observe(P_VIDEO, "vo-configured", Format::Flag)?;
        mpv.request_log_messages("warn")?;

        let video = match (&cc.gl, &cc.get_proc_address) {
            (Some(gl), Some(gpa)) => {
                match video::VideoSurface::new(&mpv, gl, gpa.clone(), cc.egui_ctx.clone()) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        log::error!("video unavailable: {e}");
                        None
                    }
                }
            }
            _ => None,
        };

        let (tx, events) = std::sync::mpsc::channel();
        let event_thread = {
            let mpv = mpv.clone();
            let ctx = cc.egui_ctx.clone();
            std::thread::Builder::new().name("lumen-mpv-events".into()).spawn(move || {
                loop {
                    match mpv.wait_event(-1.0) {
                        Some(Event::Shutdown) => break,
                        Some(Event::Log { prefix, level, text }) => {
                            log::warn!("mpv[{prefix}] {level}: {}", text.trim_end())
                        }
                        Some(ev) => {
                            if tx.send(ev).is_err() {
                                break;
                            }
                            ctx.request_repaint();
                        }
                        None => {}
                    }
                }
            })?
        };

        let (mtx, media_rx) = std::sync::mpsc::channel();
        let media = souvlaki::MediaControls::new(souvlaki::PlatformConfig {
            dbus_name: "lumen",
            display_name: "Lumen",
            hwnd: None,
        })
        .ok()
        .and_then(|mut m| {
            let ctx = cc.egui_ctx.clone();
            m.attach(move |e| {
                let _ = mtx.send(e);
                ctx.request_repaint();
            })
            .ok()?;
            Some(m)
        });

        let mut tracks = load_library();
        if tracks.is_empty() {
            tracks.push(Track {
                title: "Группа крови".into(),
                artist: "Кино".into(),
                album: "Группа крови".into(),
                path: PathBuf::new(),
                duration: 287.0,
            });
        }
        log::info!("media controls: {}", if media.is_some() { "attached" } else { "unavailable" });
        let (shot_tx, shot_rx) = std::sync::mpsc::channel();
        let mut app = Spike {
            shot_tx,
            shot_rx,
            pending_seek: None,
            mpv,
            events,
            event_thread: Some(event_thread),
            video,
            analyzer: Analyzer::new(),
            spectrum: Spectrum::new(),
            viz: viz::State::default(),
            viz_kind: opts.viz.unwrap_or(viz::Kind::Bars),
            tracks,
            current: None,
            started: Instant::now(),
            last: Instant::now(),
            pos: 0.0,
            pos_at: Instant::now(),
            dur: 0.0,
            paused: true,
            has_video: false,
            media,
            media_rx,
            opts,
            frames: 0,
            fps: 0.0,
        };
        if let Some(q) = app.opts.play.clone() {
            let q = q.to_lowercase();
            if let Some(i) = app.tracks.iter().position(|t| {
                t.title.to_lowercase().contains(&q) || t.path.to_string_lossy().to_lowercase().contains(&q)
            }) {
                app.pending_seek = app.opts.seek;
                app.play(i);
            }
        }
        Ok(app)
    }

    fn play(&mut self, i: usize) {
        let Some(t) = self.tracks.get(i) else { return };
        let path = t.path.to_string_lossy().into_owned();
        let start = format!("start={}", self.pending_seek.take().unwrap_or(0.0));
        if let Err(e) = self.mpv.command(&["loadfile", &path, "replace", "-1", &start]) {
            log::error!("{e}");
            return;
        }
        let _ = self.mpv.set_flag("pause", false);
        self.analyzer.load(Some(Path::new(&path)));
        self.viz.reset();
        self.current = Some(i);
        self.pos = 0.0;
        self.pos_at = Instant::now();
        if let Some(m) = self.media.as_mut() {
            let _ = m.set_metadata(souvlaki::MediaMetadata {
                title: Some(&t.title),
                artist: Some(&t.artist),
                album: Some(&t.album),
                duration: Some(Duration::from_secs_f64(t.duration.max(0.0))),
                cover_url: None,
            });
        }
    }

    fn toggle(&mut self) {
        if self.current.is_none() {
            self.play(0);
        } else {
            let _ = self.mpv.set_flag("pause", !self.paused);
        }
    }

    fn step(&mut self, by: isize) {
        let n = self.tracks.len() as isize;
        if n == 0 {
            return;
        }
        let i = self.current.map(|c| c as isize + by).unwrap_or(0).rem_euclid(n);
        self.play(i as usize);
    }

    /// Where playback is right now, extrapolated between mpv's updates.
    fn position(&self) -> f64 {
        if self.paused {
            self.pos
        } else {
            (self.pos + self.pos_at.elapsed().as_secs_f64()).min(self.pos + 0.25)
        }
    }

    fn pump(&mut self) {
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                Event::Property { id: P_TIME, value: Value::Double(v), .. } => {
                    self.pos = v;
                    self.pos_at = Instant::now();
                }
                Event::Property { id: P_DURATION, value, .. } => {
                    self.dur = if let Value::Double(v) = value { v } else { 0.0 };
                }
                Event::Property { id: P_PAUSE, value: Value::Flag(p), .. } => {
                    self.paused = p;
                    self.pos_at = Instant::now();
                }
                Event::Property { id: P_VIDEO, value, .. } => {
                    self.has_video = value == Value::Flag(true);
                }
                Event::EndFile { reason: lumen_playback::mpv::EndReason::Eof, .. } => self.step(1),
                _ => {}
            }
        }
        while let Ok(e) = self.media_rx.try_recv() {
            use souvlaki::MediaControlEvent as M;
            match e {
                M::Toggle => self.toggle(),
                M::Play => {
                    let _ = self.mpv.set_flag("pause", false);
                }
                M::Pause => {
                    let _ = self.mpv.set_flag("pause", true);
                }
                M::Next => self.step(1),
                M::Previous => self.step(-1),
                _ => {}
            }
        }
        if let Some(m) = self.media.as_mut() {
            let progress = Some(souvlaki::MediaPosition(Duration::from_secs_f64(self.pos.max(0.0))));
            let _ = m.set_playback(if self.current.is_none() {
                souvlaki::MediaPlayback::Stopped
            } else if self.paused {
                souvlaki::MediaPlayback::Paused { progress }
            } else {
                souvlaki::MediaPlayback::Playing { progress }
            });
        }
    }
}

fn fmt_time(s: f64) -> String {
    let s = s.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

impl eframe::App for Spike {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.pump();
        let ctx = ui.ctx().clone();
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.25);
        self.last = now;
        self.fps = self.fps * 0.95 + (1.0 / dt.max(1e-3)) * 0.05;
        self.frames += 1;

        let accent = hex("#22d3ee");
        let dim = hex("#8a8aa8");

        egui::Panel::top("transport").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(if self.paused { "▶ Play" } else { "⏸ Pause" }).clicked() {
                    self.toggle();
                }
                if ui.button("⏮").clicked() {
                    self.step(-1);
                }
                if ui.button("⏭").clicked() {
                    self.step(1);
                }
                for k in viz::Kind::ALL {
                    if ui.selectable_label(self.viz_kind == k, k.label()).clicked() {
                        self.viz_kind = k;
                        self.viz.reset();
                    }
                }
                ui.label(RichText::new(format!("{:.0} fps", self.fps)).color(dim));
            });
            if let Some(t) = self.current.and_then(|i| self.tracks.get(i)) {
                ui.label(RichText::new(&t.title).font(fonts::bold(18.0)));
                ui.label(RichText::new(format!("{} — {}", t.artist, t.album)).color(dim));
            } else {
                ui.label(RichText::new("Lumen — Кино, Земфира, 坂本龍一, 아이유").font(fonts::bold(18.0)));
            }
            let mut p = self.position();
            ui.horizontal(|ui| {
                ui.label(fmt_time(p));
                ui.spacing_mut().slider_width = (ui.available_width() - 60.0).max(80.0);
                if ui
                    .add(egui::Slider::new(&mut p, 0.0..=self.dur.max(0.01)).show_value(false))
                    .drag_stopped()
                {
                    let _ = self.mpv.command(&["seek", &p.to_string(), "absolute"]);
                }
                ui.label(fmt_time(self.dur));
            });
            ui.add_space(6.0);
        });

        egui::Panel::left("songs").default_size(380.0).show(ui, |ui| {
            let total = self.tracks.len().max(1) * 10_000 / self.tracks.len().max(1);
            let row_h = 38.0;
            ui.label(RichText::new(format!("{} rows", total)).color(dim));
            egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_h, total, |ui, range| {
                for row in range {
                    let i = row % self.tracks.len();
                    let t = &self.tracks[i];
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, 6.0, hex("#1a1a2b"));
                    }
                    let playing = self.current == Some(i);
                    let painter = ui.painter_at(rect);
                    painter.text(
                        rect.left_top() + vec2(10.0, 4.0),
                        egui::Align2::LEFT_TOP,
                        format!("{:>5}  {}", row + 1, t.title),
                        fonts::medium(14.0),
                        if playing { accent } else { hex("#f0f0ff") },
                    );
                    painter.text(
                        rect.left_top() + vec2(56.0, 21.0),
                        egui::Align2::LEFT_TOP,
                        format!("{} · {}", t.artist, t.album),
                        egui::FontId::proportional(12.0),
                        dim,
                    );
                    if resp.double_clicked() || resp.clicked() {
                        self.play(i);
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            let rect = ui.available_rect_before_wrap();
            ui.allocate_rect(rect, Sense::hover());
            if self.has_video {
                if let Some(v) = self.video.as_mut() {
                    v.show(ui, frame, rect);
                }
            } else {
                let pos = self.position();
                let ok = !self.paused && self.analyzer.read(pos, &mut self.spectrum.samples);
                if !ok {
                    self.spectrum.samples.fill(0.0);
                }
                self.spectrum.analyse(dt);
                let colors = viz::Colors { accent: hex("#22d3ee"), accent2: hex("#e879f9"), fg: hex("#ffffff") };
                let t = self.started.elapsed().as_secs_f32();
                viz::draw(self.viz_kind, &ui.painter_at(rect), rect, &self.spectrum.frame, colors, &mut self.viz, t, dt);
            }
        });

        if !self.paused || self.has_video {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        if let Some(path) = self.opts.screenshot.clone() {
            if self.frames == self.opts.frames {
                let tx = self.shot_tx.clone();
                ctx.request_screenshot(move |img| {
                    let _ = tx.send(img);
                });
            }
            if let Ok(img) = self.shot_rx.try_recv() {
                let [w, h] = img.size;
                let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_srgba_unmultiplied()).collect();
                if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, rgba) {
                    let _ = buf.save(&path);
                }
                println!(
                    "screenshot {} pos={:.2} dur={:.2} paused={} video={} fps={:.0} level={:.3}",
                    path.display(),
                    self.position(),
                    self.dur,
                    self.paused,
                    self.has_video,
                    self.fps,
                    self.spectrum.frame.level
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        // mpv's renderer has to go before the core does.
        if let (Some(v), Some(gl)) = (self.video.take(), gl) {
            v.destroy(gl);
        }
        let _ = self.mpv.command(&["quit"]);
        if let Some(t) = self.event_thread.take() {
            let _ = t.join();
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let c = hex("#0a0a12");
        [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, 1.0]
    }
}

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let opts = parse_args();
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lumen")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([720.0, 480.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native("Lumen", native, Box::new(move |cc| Ok(Box::new(Spike::new(cc, opts)?))))
}
