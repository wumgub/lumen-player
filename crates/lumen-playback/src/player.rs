//! The player: the [`Queue`] rules driving one mpv core.
//!
//! mpv's playlist holds at most two entries, the song playing and the one
//! planned to follow, so mpv can move between them without a gap. Anything
//! the user does (a new queue, ⏭, a jump) loads the song fresh and plans the
//! next one again. Repeat-one uses mpv's own file loop, which is seamless too.
//!
//! Listening time for the stats is counted here with the Electron app's rule:
//! it only grows on forward playback, so seeks and pauses don't inflate it.

use std::collections::HashMap;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

use parking_lot::Mutex;

use crate::mpv::{EndReason, Event, Format, Mpv, Value};
use crate::queue::{Queue, Repeat, Step};

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub track_id: i64,
    pub path: String,
    /// Seconds, from the library (mpv's figure replaces it once loaded).
    pub duration: f64,
    pub is_video: bool,
}

/// Things the app needs to hear about.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerEvent {
    /// A listening session ended; save it if it's long enough.
    Listened { track_id: i64, seconds: f64, counted: bool },
    /// A different song started (or playback stopped).
    TrackChanged,
}

/// What the UI reads each frame.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub current: Option<Item>,
    pub index: Option<usize>,
    pub queue: Vec<Item>,
    pub position: f64,
    pub duration: f64,
    pub paused: bool,
    /// The queue ran out (or nothing was ever played).
    pub stopped: bool,
    pub has_video: bool,
    pub shuffle: bool,
    pub repeat: Repeat,
    pub volume: f64,
}

struct Listen {
    track_id: i64,
    listened: f64,
    last_pos: f64,
    duration: f64,
    counted: bool,
}

struct Preload {
    index: usize,
    entry: i64,
}

struct Inner {
    queue: Queue<Item>,
    current_entry: Option<i64>,
    preload: Option<Preload>,
    pos: f64,
    pos_at: Instant,
    duration: f64,
    paused: bool,
    stopped: bool,
    has_video: bool,
    volume: f64,
    /// Balance volume: per-track volume multipliers, by track id, used
    /// while `balance` is on. Tracks not in here play as they are.
    gains: HashMap<i64, f64>,
    balance: bool,
    listen: Option<Listen>,
}

type Callback = Arc<dyn Fn(PlayerEvent) + Send + Sync>;
type Notify = Arc<dyn Fn() + Send + Sync>;

pub struct Player {
    mpv: Arc<Mpv>,
    inner: Arc<Mutex<Inner>>,
    on_event: Callback,
    notify: Notify,
    thread: Option<JoinHandle<()>>,
}

const P_TIME: u64 = 1;
const P_DURATION: u64 = 2;
const P_PAUSE: u64 = 3;
const P_VIDEO: u64 = 4;

/// mpv's volume control is cubic (gain = (volume/100)^3); the Electron app's
/// GainNode was linear. Convert so 0.5 still means half the amplitude. Up to
/// mpv's `volume-max` of 150, which is 3.375× (150% plus Balance volume's
/// boost fits under it).
fn mpv_volume(linear: f64) -> f64 {
    100.0 * linear.clamp(0.0, 3.375).cbrt()
}

/// The user's volume times the current song's Balance volume gain.
fn effective_volume(s: &Inner) -> f64 {
    let gain = if s.balance { s.queue.current().and_then(|c| s.gains.get(&c.track_id)).copied().unwrap_or(1.0) } else { 1.0 };
    s.volume * gain
}

fn apply_volume(mpv: &Mpv, s: &Inner) {
    let _ = mpv.set_f64("volume", mpv_volume(effective_volume(s)));
}

impl Player {
    /// `extra` options are applied on top of Lumen's defaults (e.g. `ao=null`
    /// for tests). `notify` is called from mpv's thread whenever something
    /// changed and the UI should redraw.
    pub fn new(
        extra: &[(&str, &str)],
        volume: f64,
        on_event: impl Fn(PlayerEvent) + Send + Sync + 'static,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> crate::mpv::Result<Player> {
        let mut options = vec![
            ("vo", "libmpv"),
            ("hwdec", "auto-safe"),
            // Embedded cover art isn't video; the UI draws it.
            ("audio-display", "no"),
            ("idle", "yes"),
            ("keep-open", "no"),
            ("gapless-audio", "weak"),
            ("prefetch-playlist", "yes"),
            ("volume-max", "150"),
            ("ytdl", "no"),
            ("load-scripts", "no"),
            ("input-default-bindings", "no"),
            ("audio-client-name", "Lumen"),
            ("terminal", "no"),
        ];
        options.extend_from_slice(extra);
        let vol = format!("{:.3}", mpv_volume(volume));
        options.push(("volume", vol.as_str()));
        let mpv = Arc::new(Mpv::new(&options)?);
        mpv.observe(P_TIME, "time-pos", Format::Double)?;
        mpv.observe(P_DURATION, "duration", Format::Double)?;
        mpv.observe(P_PAUSE, "pause", Format::Flag)?;
        mpv.observe(P_VIDEO, "vo-configured", Format::Flag)?;
        mpv.request_log_messages("warn")?;

        let inner = Arc::new(Mutex::new(Inner {
            queue: Queue::new(),
            current_entry: None,
            preload: None,
            pos: 0.0,
            pos_at: Instant::now(),
            duration: 0.0,
            paused: true,
            stopped: true,
            has_video: false,
            volume: volume.clamp(0.0, 1.5),
            gains: HashMap::new(),
            balance: false,
            listen: None,
        }));
        let on_event: Callback = Arc::new(on_event);
        let notify: Notify = Arc::new(notify);
        let thread = {
            let (mpv, inner, on_event, notify) = (mpv.clone(), inner.clone(), on_event.clone(), notify.clone());
            std::thread::Builder::new()
                .name("lumen-player".into())
                .spawn(move || event_loop(mpv, inner, on_event, notify))
                .map_err(|_| crate::mpv::Error { code: -1, context: "spawn".into() })?
        };
        Ok(Player { mpv, inner, on_event, notify, thread: Some(thread) })
    }

    pub fn mpv(&self) -> &Arc<Mpv> {
        &self.mpv
    }

    pub fn snapshot(&self) -> Snapshot {
        let s = self.inner.lock();
        Snapshot {
            current: s.queue.current().cloned(),
            index: s.queue.index(),
            queue: s.queue.items().to_vec(),
            position: position(&s),
            duration: if s.duration > 0.0 { s.duration } else { s.queue.current().map(|c| c.duration).unwrap_or(0.0) },
            paused: s.paused || s.stopped,
            stopped: s.stopped,
            has_video: s.has_video,
            shuffle: s.queue.shuffle,
            repeat: s.queue.repeat,
            volume: s.volume,
        }
    }

    /// Just the playback position, cheaply, for per-frame use.
    pub fn position(&self) -> f64 {
        position(&self.inner.lock())
    }

    pub fn play(&self, items: Vec<Item>, start: usize) {
        let mut s = self.inner.lock();
        let step = s.queue.play(items, start);
        self.apply(&mut s, step);
    }

    pub fn play_next(&self, items: Vec<Item>) {
        self.edit(|q| q.enqueue(items, true));
    }

    pub fn add_to_queue(&self, items: Vec<Item>) {
        self.edit(|q| q.enqueue(items, false));
    }

    pub fn jump_to(&self, index: usize) {
        self.edit(|q| q.jump_to(index));
    }

    pub fn remove(&self, index: usize) {
        self.edit(|q| q.remove(index));
    }

    pub fn clear_upcoming(&self) {
        self.edit(|q| {
            q.clear_upcoming();
            Step::None
        });
    }

    pub fn next(&self) {
        let mut s = self.inner.lock();
        let planned = s.preload.as_ref().map(|p| p.index);
        let step = s.queue.next(false, planned);
        self.apply(&mut s, step);
    }

    pub fn prev(&self) {
        let mut s = self.inner.lock();
        let pos = position(&s);
        let step = s.queue.prev(pos);
        self.apply(&mut s, step);
    }

    pub fn toggle(&self) {
        let mut s = self.inner.lock();
        if s.stopped {
            if let Some(i) = s.queue.index() {
                self.apply(&mut s, Step::Play(i));
            }
            return;
        }
        let pause = !s.paused;
        s.paused = pause;
        s.pos = position(&s);
        s.pos_at = Instant::now();
        let _ = self.mpv.set_flag("pause", pause);
    }

    pub fn set_paused(&self, pause: bool) {
        let s = self.inner.lock();
        if s.stopped || s.paused == pause {
            drop(s);
            if !pause {
                self.toggle();
            }
            return;
        }
        drop(s);
        self.toggle();
    }

    pub fn seek(&self, seconds: f64) {
        let mut s = self.inner.lock();
        if s.stopped {
            return;
        }
        let t = seconds.max(0.0);
        s.pos = t;
        s.pos_at = Instant::now();
        if let Some(l) = s.listen.as_mut() {
            l.last_pos = t;
        }
        let _ = self.mpv.command(&["seek", &format!("{t:.3}"), "absolute"]);
    }

    pub fn set_volume(&self, v: f64) {
        let mut s = self.inner.lock();
        s.volume = v.clamp(0.0, 1.5);
        apply_volume(&self.mpv, &s);
    }

    /// Turn Balance volume on or off; takes effect on the current song at once.
    pub fn set_balance(&self, on: bool) {
        let mut s = self.inner.lock();
        s.balance = on;
        apply_volume(&self.mpv, &s);
    }

    /// Set Balance volume gains (multipliers) for some tracks, e.g. all of
    /// them at startup, or one as soon as it's been measured.
    pub fn set_gains(&self, gains: impl IntoIterator<Item = (i64, f64)>) {
        let mut s = self.inner.lock();
        s.gains.extend(gains);
        apply_volume(&self.mpv, &s);
    }

    pub fn set_shuffle(&self, on: bool) {
        let mut s = self.inner.lock();
        s.queue.set_shuffle(on);
        self.replan(&mut s);
        (self.notify)();
    }

    pub fn cycle_repeat(&self) {
        let mut s = self.inner.lock();
        s.queue.repeat = s.queue.repeat.cycle();
        let _ = self.mpv.set_string("loop-file", if s.queue.repeat == Repeat::One { "inf" } else { "no" });
        self.replan(&mut s);
        (self.notify)();
    }

    /// Save the current listen (e.g. when the app quits).
    pub fn flush_listen(&self) {
        let mut s = self.inner.lock();
        if let Some(e) = bank(&mut s) {
            (self.on_event)(e);
        }
    }

    fn edit(&self, f: impl FnOnce(&mut Queue<Item>) -> Step) {
        let mut s = self.inner.lock();
        let step = f(&mut s.queue);
        self.apply(&mut s, step);
        if step == Step::None {
            self.replan(&mut s);
            (self.notify)();
        }
    }

    fn apply(&self, s: &mut Inner, step: Step) {
        match step {
            Step::Play(i) => {
                let Some(item) = s.queue.items().get(i).cloned() else { return };
                if let Some(e) = bank(s) {
                    (self.on_event)(e);
                }
                s.preload = None;
                s.stopped = false;
                s.paused = false;
                s.pos = 0.0;
                s.pos_at = Instant::now();
                s.duration = item.duration;
                s.has_video = false;
                s.listen = Some(Listen::new(item.track_id, 0.0));
                apply_volume(&self.mpv, s);
                let _ = self.mpv.set_flag("pause", false);
                if let Err(e) = self.mpv.command(&["loadfile", &item.path, "replace"]) {
                    log::warn!("couldn't play {}: {e}", item.path);
                }
                s.current_entry = self.mpv.get_i64("playlist/0/id");
                self.replan(s);
                (self.on_event)(PlayerEvent::TrackChanged);
            }
            Step::Restart => {
                if s.stopped {
                    if let Some(i) = s.queue.index() {
                        self.apply(s, Step::Play(i));
                    }
                } else {
                    s.pos = 0.0;
                    s.pos_at = Instant::now();
                    if let Some(l) = s.listen.as_mut() {
                        l.last_pos = 0.0;
                    }
                    let _ = self.mpv.command(&["seek", "0", "absolute"]);
                }
            }
            Step::Stop => stop(&self.mpv, s, &self.on_event),
            Step::None => {}
        }
        (self.notify)();
    }

    /// Line up the next song in mpv's playlist for a gapless move.
    fn replan(&self, s: &mut Inner) {
        replan(&self.mpv, s);
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.flush_listen();
        let _ = self.mpv.command(&["quit"]);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Listen {
    fn new(track_id: i64, at: f64) -> Listen {
        Listen { track_id, listened: 0.0, last_pos: at, duration: 0.0, counted: false }
    }
}

fn position(s: &Inner) -> f64 {
    if s.paused || s.stopped {
        s.pos
    } else {
        // mpv reports a few times a second; fill in between, but never run far
        // past the last report.
        s.pos + s.pos_at.elapsed().as_secs_f64().min(0.5)
    }
}

fn bank(s: &mut Inner) -> Option<PlayerEvent> {
    let l = s.listen.take()?;
    (l.listened >= 5.0).then_some(PlayerEvent::Listened {
        track_id: l.track_id,
        seconds: l.listened,
        counted: l.counted,
    })
}

fn stop(mpv: &Mpv, s: &mut Inner, on_event: &Callback) {
    if let Some(e) = bank(s) {
        on_event(e);
    }
    s.preload = None;
    s.stopped = true;
    s.paused = true;
    s.has_video = false;
    let _ = mpv.command(&["stop"]);
    on_event(PlayerEvent::TrackChanged);
}

fn replan(mpv: &Mpv, s: &mut Inner) {
    if let Some(p) = s.preload.take() {
        // Our planned entry is always the one after the current.
        let count = mpv.get_i64("playlist-count").unwrap_or(0);
        if count > 1 {
            let _ = mpv.command(&["playlist-remove", "1"]);
        }
        let _ = p;
    }
    if s.stopped || s.queue.repeat == Repeat::One {
        return;
    }
    let planned = s.queue.peek_next(true).or_else(|| s.queue.plan_shuffle_next());
    let Some(index) = planned else { return };
    let Some(item) = s.queue.items().get(index).cloned() else { return };
    log::debug!("planning {} next: {}", index, item.path);
    if mpv.command(&["loadfile", &item.path, "append"]).is_ok()
        && let Some(entry) = mpv.get_i64("playlist/1/id")
    {
        s.preload = Some(Preload { index, entry });
    }
}

fn event_loop(mpv: Arc<Mpv>, inner: Arc<Mutex<Inner>>, on_event: Callback, notify: Notify) {
    loop {
        let Some(ev) = mpv.wait_event(-1.0) else { continue };
        if matches!(ev, Event::StartFile { .. } | Event::EndFile { .. } | Event::FileLoaded) {
            log::debug!("mpv event {ev:?}");
        }
        match ev {
            Event::Shutdown => break,
            Event::Log { prefix, level, text } => {
                log::warn!("mpv[{prefix}] {level}: {}", text.trim_end());
                continue;
            }
            Event::StartFile { entry_id } => {
                let mut s = inner.lock();
                let planned = s.preload.as_ref().filter(|p| p.entry == entry_id).map(|p| p.index);
                if let Some(index) = planned {
                    // mpv moved on to the song we lined up: advance the queue to match.
                    s.preload = None;
                    if let Some(e) = bank(&mut s) {
                        on_event(e);
                    }
                    let step = s.queue.next(true, Some(index));
                    if let Step::Play(i) = step {
                        let item = s.queue.items().get(i).cloned();
                        s.current_entry = Some(entry_id);
                        s.pos = 0.0;
                        s.pos_at = Instant::now();
                        s.duration = item.as_ref().map(|t| t.duration).unwrap_or(0.0);
                        s.has_video = false;
                        s.listen = item.map(|t| Listen::new(t.track_id, 0.0));
                        // mpv's volume acts before its output buffer, so this
                        // lands on the new song's first samples.
                        apply_volume(&mpv, &s);
                        // Drop the finished entry so the playlist stays [current, next].
                        let _ = mpv.command(&["playlist-remove", "0"]);
                        replan(&mpv, &mut s);
                        on_event(PlayerEvent::TrackChanged);
                    }
                }
            }
            Event::EndFile { entry_id, reason, .. } => {
                let mut s = inner.lock();
                let ours = s.current_entry == Some(entry_id);
                if ours && matches!(reason, EndReason::Eof | EndReason::Error) && s.preload.is_none() && !s.stopped {
                    // Nothing lined up: the queue has run out.
                    let step = s.queue.next(true, None);
                    match step {
                        Step::Play(_) | Step::Restart => {
                            // Shouldn't happen (it would have been preloaded),
                            // but play it rather than stall.
                            drop(s);
                            continue;
                        }
                        _ => stop(&mpv, &mut s, &on_event),
                    }
                }
            }
            Event::Property { id: P_TIME, value: Value::Double(t), .. } => {
                let mut s = inner.lock();
                s.pos = t;
                s.pos_at = Instant::now();
                let dur = s.duration;
                if let Some(l) = s.listen.as_mut() {
                    if l.last_pos - t > 1.5 && t < 1.5 {
                        // Back at the top: repeat-one looped. Bank the pass, count anew.
                        let id = l.track_id;
                        if let Some(e) = bank(&mut s) {
                            on_event(e);
                        }
                        s.listen = Some(Listen::new(id, t));
                    } else {
                        let delta = t - l.last_pos;
                        if delta > 0.0 && delta < 2.0 {
                            l.listened += delta;
                            let d = if dur > 0.0 { dur } else { l.duration };
                            if !l.counted && d > 0.0 && l.listened >= (d * 0.5).min(240.0) {
                                l.counted = true;
                            }
                        }
                        l.last_pos = t;
                        if dur > 0.0 {
                            l.duration = dur;
                        }
                    }
                }
            }
            Event::Property { id: P_DURATION, value, .. } => {
                if let Value::Double(d) = value {
                    inner.lock().duration = d;
                }
            }
            Event::Property { id: P_PAUSE, value: Value::Flag(p), .. } => {
                let mut s = inner.lock();
                if !s.stopped {
                    s.pos = position(&s);
                    s.paused = p;
                    s.pos_at = Instant::now();
                }
            }
            // vo-configured only reports changes, and it can stay true from one
            // video to the next, so ask again whenever a file or its video changes.
            Event::FileLoaded | Event::VideoReconfig => {
                inner.lock().has_video = mpv.get_flag("vo-configured").unwrap_or(false);
            }
            Event::Property { id: P_VIDEO, value, .. } => {
                inner.lock().has_video = value == Value::Flag(true);
            }
            _ => continue,
        }
        notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn volume_is_linear_through_mpvs_cubic_curve() {
        let gain = |v: f64| (mpv_volume(v) / 100.0).powi(3);
        assert!((gain(0.5) - 0.5).abs() < 1e-9);
        assert!((gain(1.5) - 1.5).abs() < 1e-9);
        assert_eq!(mpv_volume(1.0), 100.0);
    }

    /// Plays real (generated) files through mpv with no audio output: checks
    /// gapless auto-advance, queue end, and that listening time is banked.
    #[test]
    fn plays_through_a_queue_and_banks_listens() {
        let dir = std::env::temp_dir().join(format!("lumen-player-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut items = Vec::new();
        for (i, secs) in [6.0f64, 2.0].iter().enumerate() {
            let path = dir.join(format!("t{i}.wav"));
            write_wav(&path, *secs);
            items.push(Item { track_id: i as i64 + 1, path: path.to_string_lossy().into(), duration: *secs, is_video: false });
        }
        let (tx, rx) = mpsc::channel();
        let player = Player::new(&[("ao", "null")], 1.0, move |e| { let _ = tx.send(e); }, || {}).unwrap();
        // Balance volume: the first song is turned down, the second up.
        player.set_gains([(1, 0.5), (2, 2.0)]);
        player.set_balance(true);
        player.play(items, 0);
        let volume = || player.mpv().get_f64("volume").unwrap();
        assert!((volume() - mpv_volume(0.5)).abs() < 0.01, "{}", volume());
        let mut listened = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if let Ok(e) = rx.recv_timeout(Duration::from_millis(100)) {
                if let PlayerEvent::Listened { track_id, seconds, counted } = e {
                    listened.push((track_id, seconds, counted));
                }
            }
            if player.snapshot().stopped && player.snapshot().index == Some(1) {
                break;
            }
        }
        let snap = player.snapshot();
        assert!(snap.stopped, "queue should have finished");
        assert_eq!(snap.index, Some(1), "should have moved to the second song");
        assert!((volume() - mpv_volume(2.0)).abs() < 0.01, "gain follows the song: {}", volume());
        player.set_balance(false);
        assert!((volume() - 100.0).abs() < 0.01, "{}", volume());
        // The 6 s song is banked as a counted play; the 2 s one is too short to save.
        assert_eq!(listened.len(), 1, "{listened:?}");
        assert_eq!(listened[0].0, 1);
        assert!(listened[0].1 > 4.5 && listened[0].1 < 6.5, "{listened:?}");
        assert!(listened[0].2);
        drop(player);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn write_wav(path: &std::path::Path, seconds: f64) {
        let rate = 8000u32;
        let n = (rate as f64 * seconds) as u32;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + n * 2).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&(rate * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(n * 2).to_le_bytes());
        for i in 0..n {
            let v = ((i as f64 * 440.0 * std::f64::consts::TAU / rate as f64).sin() * 8000.0) as i16;
            b.extend_from_slice(&v.to_le_bytes());
        }
        std::fs::write(path, b).unwrap();
    }
}
