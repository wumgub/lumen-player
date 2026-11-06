//! Discord Rich Presence over the desktop client's local IPC socket: a port of
//! the Electron app's discordIpc.ts and presence.ts, including the fixes that
//! made it reliable. A lost connection (Discord quit or restarted) is always
//! noticed and retried with backoff; a handshake that never gets READY, or a
//! request that never gets an answer, counts as lost; bursts of updates
//! collapse into one; and one-character titles are padded because Discord
//! rejects them.
//!
//! Everything runs on one thread fed by [`Presence`]'s methods, so callers
//! never block on Discord.
//!
//! Wire format: each packet is a little-endian u32 opcode, a u32 length, then
//! that much JSON.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const RECONNECT_MIN: Duration = Duration::from_secs(2);
const RECONNECT_MAX: Duration = Duration::from_secs(15);
const DEBOUNCE: Duration = Duration::from_millis(250);
const MIN_GAP: Duration = Duration::from_secs(2);
const RETRY_AFTER_ERROR: Duration = Duration::from_secs(5);

/// What's playing, as the app sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Payload {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    /// Seconds.
    pub position: f64,
    pub duration: f64,
}

enum Msg {
    Configure { enabled: bool, client_id: String, large_image: String },
    Update(Option<Payload>),
    Shutdown,
    // From a connection's reader thread.
    Frame { conn: u64, op: u32, body: Value },
    Closed { conn: u64 },
}

/// Trim to Discord's 128-character limit (it also rejects 1-character strings).
pub fn text(s: &str) -> String {
    let t = s.trim();
    let n = t.chars().count();
    match n {
        0 => String::new(),
        1 => format!("{t}\u{200b}"),
        _ if n <= 128 => t.to_owned(),
        _ => t.chars().take(127).collect::<String>() + "…",
    }
}

/// Every place a Discord desktop client might be listening.
pub fn socket_paths() -> Vec<PathBuf> {
    let ids = 0..10;
    if cfg!(windows) {
        return ids.map(|i| PathBuf::from(format!(r"\\?\pipe\discord-ipc-{i}"))).collect();
    }
    let mut bases: Vec<PathBuf> = Vec::new();
    for var in ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"] {
        if let Some(v) = std::env::var_os(var) {
            let p = PathBuf::from(v);
            if !bases.contains(&p) {
                bases.push(p);
            }
        }
    }
    if !bases.iter().any(|b| b == std::path::Path::new("/tmp")) {
        bases.push("/tmp".into());
    }
    // Flatpak and Snap builds of Discord put the socket in a subfolder.
    let subdirs: &[&str] = if cfg!(target_os = "linux") {
        &["", "app/com.discordapp.Discord", "app/com.discordapp.DiscordCanary", "app/com.discordapp.DiscordPTB", "app/dev.vencord.Vesktop", "snap.discord"]
    } else {
        &[""]
    };
    let mut found = Vec::new();
    for b in &bases {
        for s in subdirs {
            for i in ids.clone() {
                let p = b.join(s).join(format!("discord-ipc-{i}"));
                if p.exists() {
                    found.push(p);
                }
            }
        }
    }
    found
}

#[cfg(unix)]
type Stream = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Stream = std::fs::File;

fn open(path: &PathBuf) -> std::io::Result<Stream> {
    #[cfg(unix)]
    {
        Stream::connect(path)
    }
    #[cfg(windows)]
    {
        std::fs::OpenOptions::new().read(true).write(true).open(path)
    }
}

fn write_packet(w: &mut Stream, op: u32, body: &Value) -> std::io::Result<()> {
    let data = serde_json::to_vec(body).unwrap_or_default();
    let mut buf = Vec::with_capacity(8 + data.len());
    buf.extend_from_slice(&op.to_le_bytes());
    buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
    buf.extend_from_slice(&data);
    w.write_all(&buf)
}

fn reader(mut r: Stream, conn: u64, tx: Sender<Msg>) {
    let mut head = [0u8; 8];
    loop {
        if r.read_exact(&mut head).is_err() {
            break;
        }
        let op = u32::from_le_bytes(head[0..4].try_into().unwrap());
        let len = u32::from_le_bytes(head[4..8].try_into().unwrap()) as usize;
        if len > 1 << 20 {
            break;
        }
        let mut body = vec![0u8; len];
        if r.read_exact(&mut body).is_err() {
            break;
        }
        let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
        if tx.send(Msg::Frame { conn, op, body }).is_err() {
            return;
        }
    }
    let _ = tx.send(Msg::Closed { conn });
}

struct Conn {
    id: u64,
    w: Stream,
    ready: bool,
    opened: Instant,
    /// The request in flight: (nonce, sent at).
    waiting: Option<(String, Instant)>,
}

impl Conn {
    fn close(self) {
        #[cfg(unix)]
        let _ = self.w.shutdown(std::net::Shutdown::Both);
    }
}

struct State {
    enabled: bool,
    client_id: String,
    large_image: String,
    paths: Box<dyn Fn() -> Vec<PathBuf> + Send>,
    conn: Option<Conn>,
    next_conn_id: u64,
    reconnect_at: Option<Instant>,
    delay: Duration,
    failing: bool,
    payload: Option<Payload>,
    payload_at: i64,
    dirty: bool,
    flush_at: Option<Instant>,
    last_sent: Option<Instant>,
    retried: bool,
    nonce: u64,
    tx: Sender<Msg>,
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

impl State {
    fn connect(&mut self) {
        if self.conn.is_some() || !self.enabled || self.client_id.is_empty() {
            return;
        }
        for path in (self.paths)() {
            let Ok(stream) = open(&path) else { continue }; // a stale socket left behind
            let Ok(r) = stream.try_clone() else { continue };
            let mut w = stream;
            let id = self.next_conn_id;
            self.next_conn_id += 1;
            if write_packet(&mut w, OP_HANDSHAKE, &json!({ "v": 1, "client_id": self.client_id })).is_err() {
                continue;
            }
            let tx = self.tx.clone();
            std::thread::Builder::new().name("lumen-discord-read".into()).spawn(move || reader(r, id, tx)).ok();
            self.conn = Some(Conn { id, w, ready: false, opened: Instant::now(), waiting: None });
            return;
        }
        if !self.failing {
            log::warn!("presence: can't reach Discord");
        }
        self.failing = true;
        self.schedule_reconnect();
    }

    fn schedule_reconnect(&mut self) {
        if self.reconnect_at.is_some() || self.conn.is_some() || !self.enabled || self.client_id.is_empty() {
            return;
        }
        self.reconnect_at = Some(Instant::now() + self.delay);
        self.delay = (self.delay * 2).min(RECONNECT_MAX);
    }

    /// The connection is gone, for whatever reason: try again later.
    fn lost(&mut self) {
        if let Some(c) = self.conn.take() {
            c.close();
            log::info!("presence: lost the connection to Discord, reconnecting");
        }
        self.schedule_reconnect();
    }

    fn teardown(&mut self) {
        if let Some(c) = self.conn.take() {
            // Discord clears our activity when the connection goes.
            c.close();
        }
        self.reconnect_at = None;
        self.flush_at = None;
        self.delay = RECONNECT_MIN;
        self.failing = false;
    }

    fn flush(&mut self, extra: Duration) {
        self.dirty = true;
        let ready = self.conn.as_ref().is_some_and(|c| c.ready && c.waiting.is_none());
        if self.flush_at.is_some() || !ready {
            return;
        }
        let gap = self.last_sent.map(|t| (t + MIN_GAP).saturating_duration_since(Instant::now())).unwrap_or_default();
        self.flush_at = Some(Instant::now() + extra.max(DEBOUNCE).max(gap));
    }

    fn activity(&self, p: &Payload) -> Value {
        let state = if p.album.is_empty() { p.artist.clone() } else { format!("{} • {}", p.artist, p.album) };
        let details = text(&p.title);
        let state = text(&state);
        let large_text = text(if p.album.is_empty() { &p.title } else { &p.album });
        let large_text = if large_text.is_empty() { "Lumen".to_owned() } else { large_text };
        // type 2 is "Listening".
        let mut a = json!({
            "type": 2,
            "created_at": now_ms(),
            "instance": false,
            "details": if details.is_empty() { "Unknown track".to_owned() } else { details },
            "state": if state.is_empty() { "Unknown artist".to_owned() } else { state },
            "assets": {
                "large_image": self.large_image,
                "large_text": large_text,
                "small_image": if p.playing { "play" } else { "pause" },
                "small_text": if p.playing { "Playing" } else { "Paused" },
            }
        });
        if p.playing && p.duration > 0.0 {
            let start = self.payload_at - (p.position * 1000.0) as i64;
            a["timestamps"] = json!({ "start": start, "end": start + (p.duration * 1000.0) as i64 });
        }
        a
    }

    fn send(&mut self) {
        self.flush_at = None;
        if !self.dirty {
            return;
        }
        let args = match &self.payload {
            Some(p) => json!({ "pid": std::process::id(), "activity": self.activity(p) }),
            None => json!({ "pid": std::process::id() }),
        };
        let Some(c) = self.conn.as_mut().filter(|c| c.ready && c.waiting.is_none()) else { return };
        self.dirty = false;
        self.nonce += 1;
        let nonce = format!("lumen-{}", self.nonce);
        let frame = json!({ "cmd": "SET_ACTIVITY", "args": args, "nonce": nonce });
        if write_packet(&mut c.w, OP_FRAME, &frame).is_err() {
            self.dirty = true;
            self.lost();
            return;
        }
        c.waiting = Some((nonce, Instant::now()));
        self.last_sent = Some(Instant::now());
    }

    fn on_frame(&mut self, id: u64, op: u32, body: Value) {
        let Some(c) = self.conn.as_mut().filter(|c| c.id == id) else { return };
        match op {
            OP_PING => {
                let _ = write_packet(&mut c.w, OP_PONG, &body);
            }
            OP_CLOSE => {
                // e.g. { code: 4000, message: "Invalid Client ID" }
                log::warn!("presence: Discord closed the connection: {}", body.get("message").and_then(Value::as_str).unwrap_or("?"));
                self.lost();
            }
            OP_FRAME => {
                if body.get("cmd").and_then(Value::as_str) == Some("DISPATCH") && body.get("evt").and_then(Value::as_str) == Some("READY") {
                    c.ready = true;
                    self.failing = false;
                    self.delay = RECONNECT_MIN;
                    self.reconnect_at = None;
                    log::info!("presence: connected to Discord");
                    self.flush(Duration::ZERO);
                    return;
                }
                let nonce = body.get("nonce").and_then(Value::as_str);
                if c.waiting.as_ref().map(|(n, _)| Some(n.as_str())) == Some(nonce) {
                    c.waiting = None;
                    if body.get("evt").and_then(Value::as_str) == Some("ERROR") {
                        log::warn!("presence: update failed: {}", body["data"]["message"].as_str().unwrap_or("?"));
                        if !self.retried {
                            self.retried = true;
                            self.dirty = true;
                            self.flush(RETRY_AFTER_ERROR);
                            return;
                        }
                    } else {
                        self.retried = false;
                    }
                    if self.dirty {
                        self.flush(Duration::ZERO);
                    }
                }
            }
            _ => {}
        }
    }

    /// Time out a handshake or a request that never got an answer.
    fn check_deadlines(&mut self) {
        let stuck = self.conn.as_ref().is_some_and(|c| {
            (!c.ready && c.opened.elapsed() > HANDSHAKE_TIMEOUT) || c.waiting.as_ref().is_some_and(|(_, t)| t.elapsed() > REQUEST_TIMEOUT)
        });
        if stuck {
            log::warn!("presence: Discord stopped answering");
            self.dirty = true;
            self.lost();
        }
    }

    fn next_wake(&self) -> Duration {
        let mut wake = Duration::from_secs(1);
        for t in [self.reconnect_at, self.flush_at].into_iter().flatten() {
            wake = wake.min(t.saturating_duration_since(Instant::now()));
        }
        wake
    }
}

/// Mirrors playback to Discord. Cheap to call from the UI thread.
pub struct Presence {
    tx: Sender<Msg>,
    thread: Option<JoinHandle<()>>,
}

impl Presence {
    pub fn new() -> Presence {
        Presence::with_paths(socket_paths)
    }

    /// For tests: look for Discord somewhere else.
    pub fn with_paths(paths: impl Fn() -> Vec<PathBuf> + Send + 'static) -> Presence {
        let (tx, rx) = channel();
        let state = State {
            enabled: false,
            client_id: String::new(),
            large_image: "lumen".into(),
            paths: Box::new(paths),
            conn: None,
            next_conn_id: 1,
            reconnect_at: None,
            delay: RECONNECT_MIN,
            failing: false,
            payload: None,
            payload_at: 0,
            dirty: false,
            flush_at: None,
            last_sent: None,
            retried: false,
            nonce: 0,
            tx: tx.clone(),
        };
        let thread = std::thread::Builder::new().name("lumen-discord".into()).spawn(move || run(state, rx)).ok();
        Presence { tx, thread }
    }

    pub fn configure(&self, enabled: bool, client_id: &str, large_image: &str) {
        let _ = self.tx.send(Msg::Configure {
            enabled,
            client_id: client_id.trim().to_owned(),
            large_image: if large_image.trim().is_empty() { "lumen".into() } else { large_image.trim().to_owned() },
        });
    }

    /// Show `payload`, or clear the status with None.
    pub fn update(&self, payload: Option<Payload>) {
        let _ = self.tx.send(Msg::Update(payload));
    }
}

impl Default for Presence {
    fn default() -> Self {
        Presence::new()
    }
}

impl Drop for Presence {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run(mut s: State, rx: Receiver<Msg>) {
    loop {
        let msg = match rx.recv_timeout(s.next_wake()) {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        match msg {
            Some(Msg::Configure { enabled, client_id, large_image }) => {
                let changed = client_id != s.client_id;
                s.enabled = enabled;
                s.client_id = client_id;
                s.large_image = large_image;
                if !s.enabled || s.client_id.is_empty() || changed {
                    s.teardown();
                }
                if s.enabled && !s.client_id.is_empty() {
                    s.connect();
                    s.flush(Duration::ZERO);
                }
            }
            Some(Msg::Update(p)) => {
                s.payload = p;
                s.payload_at = now_ms();
                s.retried = false;
                if s.enabled && !s.client_id.is_empty() {
                    if s.conn.is_none() && s.reconnect_at.is_none() {
                        s.connect();
                    }
                    s.flush(Duration::ZERO);
                }
            }
            Some(Msg::Shutdown) => {
                s.payload = None;
                s.enabled = false;
                s.teardown();
                return;
            }
            Some(Msg::Frame { conn, op, body }) => s.on_frame(conn, op, body),
            Some(Msg::Closed { conn }) => {
                if s.conn.as_ref().is_some_and(|c| c.id == conn) {
                    s.dirty = true;
                    s.lost();
                }
            }
            None => {}
        }
        if s.reconnect_at.is_some_and(|t| t <= Instant::now()) {
            s.reconnect_at = None;
            s.connect();
        }
        if s.flush_at.is_some_and(|t| t <= Instant::now()) {
            s.send();
        }
        s.check_deadlines();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::sync::{Arc, Mutex};

    /// A pretend Discord: answers the handshake with READY and every request
    /// with a reply, recording the activities it was sent.
    struct Fake {
        seen: Arc<Mutex<Vec<Value>>>,
        handshakes: Arc<Mutex<usize>>,
    }

    fn serve(listener: UnixListener, f: &Fake, die_after: Option<usize>) -> JoinHandle<()> {
        let (seen, hs) = (f.seen.clone(), f.handshakes.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { return };
                let mut count = 0;
                loop {
                    let mut head = [0u8; 8];
                    if s.read_exact(&mut head).is_err() {
                        break;
                    }
                    let op = u32::from_le_bytes(head[0..4].try_into().unwrap());
                    let len = u32::from_le_bytes(head[4..8].try_into().unwrap()) as usize;
                    let mut body = vec![0; len];
                    s.read_exact(&mut body).unwrap();
                    let v: Value = serde_json::from_slice(&body).unwrap();
                    if op == OP_HANDSHAKE {
                        *hs.lock().unwrap() += 1;
                        write_packet(&mut s, OP_FRAME, &json!({"cmd":"DISPATCH","evt":"READY","data":{}})).unwrap();
                    } else {
                        seen.lock().unwrap().push(v["args"].clone());
                        write_packet(&mut s, OP_FRAME, &json!({"cmd":"SET_ACTIVITY","nonce":v["nonce"],"data":{}})).unwrap();
                        count += 1;
                        if Some(count) == die_after {
                            // Discord quitting: drop the connection and stop listening.
                            return;
                        }
                    }
                }
            }
        })
    }

    fn wait(mut f: impl FnMut() -> bool) -> bool {
        let end = Instant::now() + Duration::from_secs(12);
        while Instant::now() < end {
            if f() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    #[test]
    fn padding_and_trimming() {
        assert_eq!(text("x"), "x\u{200b}");
        assert_eq!(text("  "), "");
        assert_eq!(text(&"я".repeat(200)).chars().count(), 128);
    }

    #[test]
    fn reconnects_after_discord_restarts() {
        let dir = std::env::temp_dir().join(format!("lumen-discord-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("discord-ipc-0");
        let _ = std::fs::remove_file(&sock);
        let fake = Fake { seen: Arc::default(), handshakes: Arc::default() };
        let server = serve(UnixListener::bind(&sock).unwrap(), &fake, Some(1));

        let s2 = sock.clone();
        let p = Presence::with_paths(move || if s2.exists() { vec![s2.clone()] } else { vec![] });
        p.configure(true, "123", "lumen");
        let song = |t: &str| Payload { title: t.into(), artist: "Кино".into(), album: "".into(), playing: true, position: 10.0, duration: 200.0 };
        p.update(Some(song("Группа крови")));
        assert!(wait(|| fake.seen.lock().unwrap().len() == 1), "first update never arrived");
        let first = fake.seen.lock().unwrap()[0].clone();
        assert_eq!(first["activity"]["details"], "Группа крови");
        assert_eq!(first["activity"]["type"], 2);
        assert!(first["activity"]["timestamps"]["end"].as_i64().unwrap() - first["activity"]["timestamps"]["start"].as_i64().unwrap() == 200_000);

        // Discord quits, then comes back.
        server.join().unwrap();
        let _ = std::fs::remove_file(&sock);
        p.update(Some(song("Кукушка")));
        std::thread::sleep(Duration::from_millis(500));
        let _server2 = serve(UnixListener::bind(&sock).unwrap(), &fake, None);
        assert!(wait(|| *fake.handshakes.lock().unwrap() >= 2), "never reconnected");
        assert!(wait(|| fake.seen.lock().unwrap().iter().any(|a| a["activity"]["details"] == "Кукушка")), "latest song not re-sent");
        drop(p);
        let _ = std::fs::remove_dir_all(dir);
    }
}
