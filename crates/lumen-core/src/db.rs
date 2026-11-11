//! The library database. Same file, schema and rules as the Electron app had
//! (up to 0.2.5), so an upgrade opens the existing `library.db` in place.

use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::functions::FunctionFlags;
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter};

use crate::fold::fold;
use crate::models::*;

pub use rusqlite::Error;
pub type Result<T> = rusqlite::Result<T>;

// Lock bits for fields the user has renamed, so a rescan keeps their edits.
const L_TITLE: i64 = 1;
const L_ARTIST: i64 = 2;
const L_ALBUM: i64 = 4;

const COLS: &str = "id, path, title, artist, albumArtist, album, genre, year, trackNo, discNo, \
                    duration, hasArt, addedAt, note, isVideo, loudness, peak";

pub(crate) fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn track_from(r: &Row<'_>) -> Result<Track> {
    Ok(Track {
        id: r.get(0)?,
        path: r.get(1)?,
        title: r.get(2)?,
        artist: r.get(3)?,
        album_artist: r.get(4)?,
        album: r.get(5)?,
        genre: r.get(6)?,
        year: r.get(7)?,
        track_no: r.get(8)?,
        disc_no: r.get(9)?,
        duration: r.get(10)?,
        has_art: r.get::<_, i64>(11)? != 0,
        added_at: r.get(12)?,
        note: r.get(13)?,
        is_video: r.get::<_, i64>(14)? != 0,
        loudness: r.get(15)?,
        peak: r.get(16)?,
    })
}

/// What the scanner read from a file.
#[derive(Debug, Clone, Default)]
pub struct UpsertTrack {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<i64>,
    pub track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub duration: f64,
    pub is_video: bool,
    pub art: Option<Art>,
}


fn upsert_in(tx: &Connection, t: &UpsertTrack) -> Result<(i64, bool)> {
    let existing = tx
        .query_row(
            "SELECT id, title, artist, albumArtist, album, locked FROM tracks WHERE path = ?",
            [&t.path],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()?;
    let has_art = t.art.is_some() as i64;
    let id = match &existing {
        Some((id, title, artist, album_artist, album, locked)) => {
            let keep = |bit: i64, old: &str, new: &str| -> String {
                if locked & bit != 0 { old.to_owned() } else { new.to_owned() }
            };
            // A file that doesn't say how long it is keeps the length learned on playback.
            tx.execute(
                "UPDATE tracks SET title=?1, artist=?2, albumArtist=?3, album=?4, genre=?5,
                 year=?6, trackNo=?7, discNo=?8,
                 duration=CASE WHEN ?9 > 0 THEN ?9 ELSE duration END, hasArt=?10, isVideo=?11
                 WHERE path=?12",
                params![
                    keep(L_TITLE, title, &t.title),
                    keep(L_ARTIST, artist, &t.artist),
                    keep(L_ARTIST, album_artist, &t.album_artist),
                    keep(L_ALBUM, album, &t.album),
                    t.genre,
                    t.year,
                    t.track_no,
                    t.disc_no,
                    t.duration,
                    has_art,
                    t.is_video as i64,
                    t.path
                ],
            )?;
            *id
        }
        None => {
            tx.execute(
                "INSERT INTO tracks (path, title, artist, albumArtist, album, genre, year,
                 trackNo, discNo, duration, hasArt, addedAt, isVideo)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    t.path,
                    t.title,
                    t.artist,
                    t.album_artist,
                    t.album,
                    t.genre,
                    t.year,
                    t.track_no,
                    t.disc_no,
                    t.duration,
                    has_art,
                    now_ms(),
                    t.is_video as i64
                ],
            )?;
            tx.last_insert_rowid()
        }
    };
    if let Some(art) = &t.art {
        tx.execute(
            "INSERT INTO art (track_id, mime, data) VALUES (?, ?, ?)
             ON CONFLICT(track_id) DO UPDATE SET mime=excluded.mime, data=excluded.data",
            params![id, art.mime, art.data],
        )?;
    }
    Ok((id, existing.is_none()))
}

pub struct Library {
    conn: Connection,
}

impl Library {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // SQLite's LIKE/lower() only fold ASCII; search compares through this.
        conn.create_scalar_function(
            "fold",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            |ctx| Ok(ctx.get::<Option<String>>(0)?.map(|s| fold(&s)).unwrap_or_default()),
        )?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tracks (
              id          INTEGER PRIMARY KEY AUTOINCREMENT,
              path        TEXT UNIQUE NOT NULL,
              title       TEXT NOT NULL,
              artist      TEXT NOT NULL DEFAULT '',
              albumArtist TEXT NOT NULL DEFAULT '',
              album       TEXT NOT NULL DEFAULT '',
              genre       TEXT NOT NULL DEFAULT '',
              year        INTEGER,
              trackNo     INTEGER,
              discNo      INTEGER,
              duration    REAL NOT NULL DEFAULT 0,
              hasArt      INTEGER NOT NULL DEFAULT 0,
              addedAt     INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
            CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);

            CREATE TABLE IF NOT EXISTS art (
              track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
              mime     TEXT NOT NULL,
              data     BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playlists (
              id        INTEGER PRIMARY KEY AUTOINCREMENT,
              name      TEXT NOT NULL,
              createdAt INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playlist_tracks (
              playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
              track_id    INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
              position    INTEGER NOT NULL,
              PRIMARY KEY (playlist_id, track_id)
            );

            CREATE TABLE IF NOT EXISTS folders (
              path TEXT PRIMARY KEY
            );

            CREATE TABLE IF NOT EXISTS settings (
              key   TEXT PRIMARY KEY,
              value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS album_covers (
              album    TEXT PRIMARY KEY,
              source   TEXT NOT NULL,
              track_id INTEGER,
              mime     TEXT,
              data     BLOB
            );

            CREATE TABLE IF NOT EXISTS plays (
              id        INTEGER PRIMARY KEY AUTOINCREMENT,
              track_id  INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
              played_at INTEGER NOT NULL,
              seconds   INTEGER NOT NULL,
              counted   INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_plays_track ON plays(track_id);
            CREATE INDEX IF NOT EXISTS idx_plays_at ON plays(played_at);
            "#,
        )?;

        let lib = Library { conn };
        let cols = lib.columns("tracks")?;
        let add = |name: &str, ddl: &str| -> Result<()> {
            if !cols.iter().any(|c| c == name) {
                lib.conn.execute_batch(ddl)?;
            }
            Ok(())
        };
        add("locked", "ALTER TABLE tracks ADD COLUMN locked INTEGER NOT NULL DEFAULT 0")?;
        add("albumPos", "ALTER TABLE tracks ADD COLUMN albumPos INTEGER")?;
        add("note", "ALTER TABLE tracks ADD COLUMN note TEXT NOT NULL DEFAULT ''")?;
        add("isVideo", "ALTER TABLE tracks ADD COLUMN isVideo INTEGER NOT NULL DEFAULT 0")?;
        // Balance volume: integrated loudness (LUFS) and sample peak, plus the
        // file's size and modified time when it was measured, so an edited
        // file gets measured again.
        add("loudness", "ALTER TABLE tracks ADD COLUMN loudness REAL")?;
        add("peak", "ALTER TABLE tracks ADD COLUMN peak REAL")?;
        add("loudnessKey", "ALTER TABLE tracks ADD COLUMN loudnessKey TEXT")?;
        if !lib.columns("playlists")?.iter().any(|c| c == "position") {
            lib.conn.execute_batch(
                "ALTER TABLE playlists ADD COLUMN position INTEGER;
                 UPDATE playlists SET position = (
                   SELECT COUNT(*) FROM playlists p2
                   WHERE p2.name < playlists.name COLLATE NOCASE
                      OR (p2.name = playlists.name AND p2.id < playlists.id)
                 ) WHERE position IS NULL;",
            )?;
        }
        Ok(lib)
    }

    fn columns(&self, table: &str) -> Result<Vec<String>> {
        let mut q = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        q.query_map([], |r| r.get::<_, String>(1))?.collect()
    }

    // ---- tracks ----

    /// Insert or refresh a track from its tags. Fields the user renamed keep
    /// their edited value. Returns the id and whether it was new.
    pub fn upsert_track(&self, t: &UpsertTrack) -> Result<(i64, bool)> {
        let tx = self.conn.unchecked_transaction()?;
        let r = upsert_in(&tx, t)?;
        tx.commit()?;
        Ok(r)
    }

    /// `upsert_track` for a batch, in one transaction.
    pub fn upsert_tracks(&self, tracks: &[UpsertTrack]) -> Result<Vec<(i64, bool)>> {
        let tx = self.conn.unchecked_transaction()?;
        let r = tracks.iter().map(|t| upsert_in(&tx, t)).collect::<Result<Vec<_>>>()?;
        tx.commit()?;
        Ok(r)
    }

    /// Fill in a length the tags didn't have (mpv works it out on playback).
    pub fn set_duration(&self, id: i64, seconds: f64) -> Result<()> {
        self.conn.execute("UPDATE tracks SET duration = ? WHERE id = ? AND duration <= 0", params![seconds, id])?;
        Ok(())
    }

    /// Every track with the key its loudness was measured under (None if it
    /// never was), for the Balance volume worker to compare with the file.
    pub fn loudness_keys(&self) -> Result<Vec<(i64, String, Option<String>)>> {
        let mut st = self.conn.prepare("SELECT id, path, loudnessKey FROM tracks ORDER BY id")?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Store a measurement. `None` loudness means "couldn't measure", which
    /// still records the key so it isn't retried until the file changes.
    pub fn set_loudness(&self, id: i64, loudness: Option<f64>, peak: Option<f64>, key: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET loudness = ?, peak = ?, loudnessKey = ? WHERE id = ?",
            params![loudness, peak, key, id],
        )?;
        Ok(())
    }

    pub fn track(&self, id: i64) -> Result<Option<Track>> {
        self.conn
            .query_row(&format!("SELECT {COLS} FROM tracks WHERE id = ?"), [id], track_from)
            .optional()
    }

    pub fn art(&self, track_id: i64) -> Result<Option<Art>> {
        self.conn
            .query_row("SELECT mime, data FROM art WHERE track_id = ?", [track_id], |r| {
                Ok(Art { mime: r.get(0)?, data: r.get(1)? })
            })
            .optional()
    }

    pub fn query(&self, q: &Query) -> Result<Vec<Track>> {
        let mut clauses: Vec<&str> = Vec::new();
        let mut args: Vec<Value> = Vec::new();
        if let Some(s) = q.search.as_deref().filter(|s| !s.is_empty()) {
            // instr() rather than LIKE, so a typed % or _ is matched literally.
            clauses.push(
                "(instr(fold(title), ?) OR instr(fold(artist), ?) OR instr(fold(album), ?) \
                 OR instr(fold(note), ?))",
            );
            let f = fold(s);
            args.extend(std::iter::repeat_n(Value::Text(f), 4));
        }
        if let Some(a) = &q.artist {
            clauses.push("(CASE WHEN albumArtist != '' THEN albumArtist ELSE artist END) = ?");
            args.push(Value::Text(a.clone()));
        }
        if let Some(a) = &q.album {
            clauses.push("album = ?");
            args.push(Value::Text(a.clone()));
        }
        if let Some(g) = &q.genre {
            clauses.push("genre = ?");
            args.push(Value::Text(g.clone()));
        }
        if let Some(f) = &q.folder {
            clauses.push("path LIKE ?");
            let base = if f.ends_with('/') || f.ends_with('\\') { f.clone() } else { format!("{f}/") };
            args.push(Value::Text(format!("{base}%")));
        }
        let mut sql = format!("SELECT {COLS} FROM tracks");
        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }
        sql.push_str(" ORDER BY ");
        sql.push_str(&q.order_by());
        if let Some(l) = q.limit {
            sql.push_str(&format!(" LIMIT {l}"));
        }
        if let Some(o) = q.offset {
            if q.limit.is_none() {
                sql.push_str(" LIMIT -1");
            }
            sql.push_str(&format!(" OFFSET {o}"));
        }
        let mut stmt = self.conn.prepare(&sql)?;
        stmt.query_map(params_from_iter(args), track_from)?.collect()
    }

    pub fn artists(&self) -> Result<Vec<ArtistRow>> {
        let mut q = self.conn.prepare(
            "SELECT CASE WHEN albumArtist != '' THEN albumArtist ELSE artist END AS artist,
             COUNT(*) FROM tracks GROUP BY artist ORDER BY artist COLLATE NOCASE ASC",
        )?;
        q.query_map([], |r| Ok(ArtistRow { artist: r.get(0)?, count: r.get(1)? }))?.collect()
    }

    pub fn albums(&self) -> Result<Vec<AlbumRow>> {
        let mut q = self.conn.prepare(
            "SELECT album, MAX(CASE WHEN albumArtist != '' THEN albumArtist ELSE artist END),
             COUNT(*) FROM tracks WHERE album != '' GROUP BY album ORDER BY album COLLATE NOCASE ASC",
        )?;
        q.query_map([], |r| Ok(AlbumRow { album: r.get(0)?, artist: r.get(1)?, count: r.get(2)? }))?
            .collect()
    }

    pub fn counts(&self) -> Result<Counts> {
        let one = |sql: &str| self.conn.query_row(sql, [], |r| r.get::<_, i64>(0));
        Ok(Counts {
            tracks: one("SELECT COUNT(*) FROM tracks")?,
            artists: one("SELECT COUNT(DISTINCT artist) FROM tracks")?,
            albums: one("SELECT COUNT(DISTINCT album) FROM tracks WHERE album != ''")?,
        })
    }

    pub fn all_paths(&self) -> Result<Vec<String>> {
        let mut q = self.conn.prepare("SELECT path FROM tracks")?;
        q.query_map([], |r| r.get(0))?.collect()
    }

    /// Every track's id, path, title (as the tags or the user named it) and length,
    /// for the scanner to recognise files that moved.
    pub fn path_index(&self) -> Result<Vec<(i64, String, String, f64)>> {
        let mut q = self.conn.prepare("SELECT id, path, title, duration FROM tracks")?;
        q.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect()
    }

    /// Point a track at the file's new location, keeping everything attached to it
    /// (plays, playlists, edits, notes).
    pub fn move_track(&self, id: i64, path: &str) -> Result<()> {
        self.conn.execute("UPDATE tracks SET path = ? WHERE id = ?", params![path, id])?;
        Ok(())
    }

    /// Delete every track whose path isn't in `keep`. Returns how many went.
    pub fn remove_missing(&self, keep: &HashSet<String>) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let rows: Vec<(i64, String)> = {
            let mut q = tx.prepare("SELECT id, path FROM tracks")?;
            q.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_>>()?
        };
        let mut removed = 0;
        for (id, path) in rows {
            if !keep.contains(&path) {
                tx.execute("DELETE FROM tracks WHERE id = ?", [id])?;
                removed += 1;
            }
        }
        tx.commit()?;
        Ok(removed)
    }

    // ---- edits (database only; files are never touched) ----

    pub fn rename_track(&self, id: i64, title: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET title = ?, locked = locked | ? WHERE id = ?",
            params![title, L_TITLE, id],
        )?;
        Ok(())
    }

    /// Set a track's note; an empty string clears it.
    pub fn set_track_note(&self, id: i64, note: &str) -> Result<()> {
        self.conn.execute("UPDATE tracks SET note = ? WHERE id = ?", params![note, id])?;
        Ok(())
    }

    /// Set one track's artist, leaving every other track alone. An album
    /// artist that matched the old artist follows it, so the track moves to
    /// the new name in Artists too.
    pub fn set_track_artist(&self, id: i64, artist: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET
               albumArtist = CASE WHEN albumArtist = artist THEN ?1 ELSE albumArtist END,
               artist = ?1,
               locked = locked | ?2
             WHERE id = ?3",
            params![artist, L_ARTIST, id],
        )?;
        Ok(())
    }

    /// Rename the effective artist (album artist when set, else artist) for the
    /// whole group.
    pub fn rename_artist(&self, old: &str, new: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET
               albumArtist = CASE WHEN albumArtist = ?1 THEN ?2 ELSE albumArtist END,
               artist = CASE WHEN albumArtist = '' AND artist = ?1 THEN ?2 ELSE artist END,
               locked = locked | ?3
             WHERE albumArtist = ?1 OR (albumArtist = '' AND artist = ?1)",
            params![old, new, L_ARTIST],
        )?;
        Ok(())
    }

    pub fn rename_album(&self, old: &str, new: &str) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE tracks SET album = ?1, locked = locked | ?2 WHERE album = ?3",
            params![new, L_ALBUM, old],
        )?;
        // Covers are keyed by album name, so carry a custom cover across the
        // rename. Merging into an album that has its own cover keeps that one.
        let target_has_cover =
            tx.query_row("SELECT 1 FROM album_covers WHERE album = ?", [new], |_| Ok(())).optional()?.is_some();
        if target_has_cover {
            tx.execute("DELETE FROM album_covers WHERE album = ?", [old])?;
        } else {
            tx.execute("UPDATE album_covers SET album = ? WHERE album = ?", [new, old])?;
        }
        tx.commit()
    }

    /// Put tracks into an album by name, creating the group if it's new.
    pub fn set_track_album(&self, ids: &[i64], album: &str) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut upd = tx.prepare(
                "UPDATE tracks SET album = ?1, albumPos = NULL, locked = locked | ?2 WHERE id = ?3",
            )?;
            for id in ids {
                upd.execute(params![album, L_ALBUM, id])?;
            }
        }
        tx.commit()
    }

    /// Save a manual track order within an album.
    pub fn reorder_album(&self, album: &str, ordered: &[i64]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut upd =
                tx.prepare("UPDATE tracks SET albumPos = ?1 WHERE id = ?2 AND album = ?3")?;
            for (i, id) in ordered.iter().enumerate() {
                upd.execute(params![i as i64, id, album])?;
            }
        }
        tx.commit()
    }

    // ---- album covers ----

    /// The track whose art stands for an album by default: its first track
    /// (manual order, then disc/track number) that has embedded art.
    fn first_art_track(&self, album: &str) -> Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT id FROM tracks WHERE album = ? AND hasArt = 1
                 ORDER BY albumPos IS NULL, albumPos, discNo, trackNo, id LIMIT 1",
                [album],
                |r| r.get(0),
            )
            .optional()
    }

    /// An album's cover: the override if set, else its first track's art.
    pub fn album_art(&self, album: &str) -> Result<Option<Art>> {
        let over = self
            .conn
            .query_row(
                "SELECT source, track_id, mime, data FROM album_covers WHERE album = ?",
                [album],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<i64>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<Vec<u8>>>(3)?,
                    ))
                },
            )
            .optional()?;
        match over {
            Some((source, _, Some(mime), Some(data))) if source == "custom" => {
                return Ok(Some(Art { mime, data }));
            }
            Some((source, Some(track), _, _)) if source == "track" => return self.art(track),
            _ => {}
        }
        match self.first_art_track(album)? {
            Some(id) => self.art(id),
            None => Ok(None),
        }
    }

    pub fn album_has_cover(&self, album: &str) -> Result<bool> {
        Ok(self
            .conn
            .query_row("SELECT 1 FROM album_covers WHERE album = ?", [album], |_| Ok(()))
            .optional()?
            .is_some())
    }

    pub fn set_album_cover_from_track(&self, album: &str, track_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO album_covers (album, source, track_id, mime, data)
             VALUES (?1, 'track', ?2, NULL, NULL)
             ON CONFLICT(album) DO UPDATE SET source='track', track_id=?2, mime=NULL, data=NULL",
            params![album, track_id],
        )?;
        Ok(())
    }

    pub fn set_album_cover_custom(&self, album: &str, mime: &str, data: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO album_covers (album, source, track_id, mime, data)
             VALUES (?1, 'custom', NULL, ?2, ?3)
             ON CONFLICT(album) DO UPDATE SET source='custom', track_id=NULL, mime=?2, data=?3",
            params![album, mime, data],
        )?;
        Ok(())
    }

    pub fn clear_album_cover(&self, album: &str) -> Result<()> {
        self.conn.execute("DELETE FROM album_covers WHERE album = ?", [album])?;
        Ok(())
    }

    // ---- listening stats ----

    pub fn record_play(&self, track_id: i64, seconds: f64, counted: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO plays (track_id, played_at, seconds, counted) VALUES (?, ?, ?, ?)",
            params![track_id, now_ms(), seconds.max(0.0).round() as i64, counted as i64],
        )?;
        Ok(())
    }

    /// The whole play log, oldest first.
    pub fn play_history(&self) -> Result<Vec<PlayEvent>> {
        let mut q = self
            .conn
            .prepare("SELECT track_id, played_at, seconds, counted FROM plays ORDER BY played_at")?;
        q.query_map([], |r| {
            Ok(PlayEvent {
                track_id: r.get(0)?,
                played_at: r.get(1)?,
                seconds: r.get(2)?,
                counted: r.get::<_, i64>(3)? != 0,
            })
        })?
        .collect()
    }

    pub fn clear_play_history(&self) -> Result<()> {
        self.conn.execute("DELETE FROM plays", [])?;
        Ok(())
    }

    // ---- settings ----

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key = ?", [key], |r| r.get(0))
            .optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    pub fn app_settings(&self) -> Result<AppSettings> {
        let d = AppSettings::default();
        let text = |k: &str, or: &str| -> Result<String> {
            Ok(self.setting(k)?.unwrap_or_else(|| or.to_owned()))
        };
        let flag = |k: &str, or: bool| -> Result<bool> {
            Ok(self.setting(k)?.map(|v| v == "true").unwrap_or(or))
        };
        Ok(AppSettings {
            stats_enabled: flag("statsEnabled", d.stats_enabled)?,
            discord_enabled: flag("discordEnabled", d.discord_enabled)?,
            discord_client_id: text("discordClientId", "")?,
            discord_client_secret: text("discordClientSecret", "")?,
            discord_large_image_key: text("discordLargeImageKey", &d.discord_large_image_key)?,
            discord_cover_art: flag("discordCoverArt", d.discord_cover_art)?,
        })
    }

    pub fn save_app_settings(&self, s: &AppSettings) -> Result<()> {
        let flag = |b: bool| if b { "true" } else { "false" };
        self.set_setting("statsEnabled", flag(s.stats_enabled))?;
        self.set_setting("discordEnabled", flag(s.discord_enabled))?;
        self.set_setting("discordClientId", s.discord_client_id.trim())?;
        self.set_setting("discordClientSecret", s.discord_client_secret.trim())?;
        self.set_setting("discordLargeImageKey", s.discord_large_image_key.trim())?;
        self.set_setting("discordCoverArt", flag(s.discord_cover_art))
    }

    // ---- folders ----

    pub fn add_folder(&self, path: &str) -> Result<()> {
        self.conn.execute("INSERT OR IGNORE INTO folders (path) VALUES (?)", [path])?;
        Ok(())
    }

    pub fn folders(&self) -> Result<Vec<String>> {
        let mut q = self.conn.prepare("SELECT path FROM folders ORDER BY path")?;
        q.query_map([], |r| r.get(0))?.collect()
    }

    pub fn remove_folder(&self, path: &str) -> Result<()> {
        self.conn.execute("DELETE FROM folders WHERE path = ?", [path])?;
        Ok(())
    }

    // ---- playlists ----

    pub fn create_playlist(&self, name: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO playlists (name, createdAt, position)
             VALUES (?, ?, (SELECT COALESCE(MAX(position), -1) + 1 FROM playlists))",
            params![name, now_ms()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        let mut q = self.conn.prepare(
            "SELECT p.id, p.name, p.createdAt, COUNT(pt.track_id) FROM playlists p
             LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
             GROUP BY p.id ORDER BY p.position ASC, p.name COLLATE NOCASE",
        )?;
        q.query_map([], |r| {
            Ok(Playlist { id: r.get(0)?, name: r.get(1)?, created_at: r.get(2)?, count: r.get(3)? })
        })?
        .collect()
    }

    pub fn reorder_playlists(&self, ordered: &[i64]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut upd = tx.prepare("UPDATE playlists SET position = ? WHERE id = ?")?;
            for (i, id) in ordered.iter().enumerate() {
                upd.execute(params![i as i64, id])?;
            }
        }
        tx.commit()
    }

    pub fn rename_playlist(&self, id: i64, name: &str) -> Result<()> {
        self.conn.execute("UPDATE playlists SET name = ? WHERE id = ?", params![name, id])?;
        Ok(())
    }

    pub fn delete_playlist(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM playlists WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn add_to_playlist(&self, playlist: i64, tracks: &[i64]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let start: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?",
            [playlist],
            |r| r.get(0),
        )?;
        {
            let mut ins = tx.prepare(
                "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)",
            )?;
            for (i, t) in tracks.iter().enumerate() {
                ins.execute(params![playlist, t, start + i as i64])?;
            }
        }
        tx.commit()
    }

    pub fn remove_from_playlist(&self, playlist: i64, track: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?",
            [playlist, track],
        )?;
        Ok(())
    }

    pub fn playlist_tracks(&self, playlist: i64) -> Result<Vec<Track>> {
        let cols: Vec<String> = COLS.split(", ").map(|c| format!("t.{c}")).collect();
        let mut q = self.conn.prepare(&format!(
            "SELECT {} FROM playlist_tracks pt JOIN tracks t ON t.id = pt.track_id
             WHERE pt.playlist_id = ? ORDER BY pt.position ASC",
            cols.join(", ")
        ))?;
        q.query_map([playlist], track_from)?.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(path: &str, title: &str, artist: &str, album: &str) -> UpsertTrack {
        UpsertTrack {
            path: path.into(),
            title: title.into(),
            artist: artist.into(),
            album: album.into(),
            duration: 200.0,
            ..Default::default()
        }
    }

    #[test]
    fn search_folds_any_script_and_matches_literally() {
        let lib = Library::open_in_memory().unwrap();
        lib.upsert_track(&song("/m/1.mp3", "Группа крови", "Кино", "Группа крови")).unwrap();
        lib.upsert_track(&song("/m/2.mp3", "100% Pure", "Someone", "Ёлка")).unwrap();
        lib.upsert_track(&song("/m/3.mp3", "Other", "Beyoncé", "X")).unwrap();
        let find = |s: &str| {
            lib.query(&Query { search: Some(s.into()), ..Default::default() })
                .unwrap()
                .into_iter()
                .map(|t| t.path)
                .collect::<Vec<_>>()
        };
        assert_eq!(find("кино"), vec!["/m/1.mp3"]);
        assert_eq!(find("елка"), vec!["/m/2.mp3"]);
        assert_eq!(find("beyonce"), vec!["/m/3.mp3"]);
        assert_eq!(find("100%"), vec!["/m/2.mp3"]);
        assert!(find("1_0").is_empty());
    }

    #[test]
    fn rescans_keep_renamed_fields() {
        let lib = Library::open_in_memory().unwrap();
        let (id, new) = lib.upsert_track(&song("/m/a.mp3", "Tag Title", "Tag Artist", "Tag Album")).unwrap();
        assert!(new);
        lib.rename_track(id, "My Title").unwrap();
        lib.set_track_album(&[id], "My Album").unwrap();
        let (id2, new2) =
            lib.upsert_track(&song("/m/a.mp3", "Tag Title 2", "Tag Artist 2", "Tag Album 2")).unwrap();
        assert_eq!((id2, new2), (id, false));
        let t = lib.track(id).unwrap().unwrap();
        assert_eq!(t.title, "My Title");
        assert_eq!(t.album, "My Album");
        assert_eq!(t.artist, "Tag Artist 2"); // not renamed, so it follows the tags
    }

    #[test]
    fn a_songs_artist_changes_only_that_song() {
        let lib = Library::open_in_memory().unwrap();
        let (a, _) = lib.upsert_track(&song("/m/a.mp3", "A", "", "")).unwrap();
        let (b, _) = lib.upsert_track(&song("/m/b.mp3", "B", "", "")).unwrap();
        lib.set_track_artist(a, "Кино").unwrap();
        assert_eq!(lib.track(a).unwrap().unwrap().artist, "Кино");
        assert_eq!(lib.track(b).unwrap().unwrap().artist, "");
        // Locked, so a rescan keeps it.
        lib.upsert_track(&song("/m/a.mp3", "A", "", "")).unwrap();
        assert_eq!(lib.track(a).unwrap().unwrap().artist, "Кино");
    }

    #[test]
    fn album_rename_carries_its_cover() {
        let lib = Library::open_in_memory().unwrap();
        let (a, _) = lib.upsert_track(&song("/m/a.mp3", "A", "X", "Old")).unwrap();
        lib.upsert_track(&song("/m/b.mp3", "B", "X", "Taken")).unwrap();
        lib.set_album_cover_custom("Old", "image/png", b"old-cover").unwrap();
        lib.rename_album("Old", "New").unwrap();
        assert_eq!(lib.album_art("New").unwrap().unwrap().data, b"old-cover");
        assert!(!lib.album_has_cover("Old").unwrap());

        // Merging into an album with its own cover keeps the target's.
        lib.set_album_cover_custom("Taken", "image/png", b"taken-cover").unwrap();
        lib.rename_album("New", "Taken").unwrap();
        assert_eq!(lib.album_art("Taken").unwrap().unwrap().data, b"taken-cover");
        assert!(!lib.album_has_cover("New").unwrap());
        assert_eq!(lib.track(a).unwrap().unwrap().album, "Taken");
    }

    #[test]
    fn album_art_falls_back_to_first_track_with_art() {
        let lib = Library::open_in_memory().unwrap();
        let mut one = song("/m/1.mp3", "1", "X", "Al");
        one.track_no = Some(2);
        one.art = Some(Art { mime: "image/jpeg".into(), data: b"second".to_vec() });
        let mut two = song("/m/2.mp3", "2", "X", "Al");
        two.track_no = Some(1);
        two.art = Some(Art { mime: "image/jpeg".into(), data: b"first".to_vec() });
        lib.upsert_track(&one).unwrap();
        let (t2, _) = lib.upsert_track(&two).unwrap();
        assert_eq!(lib.album_art("Al").unwrap().unwrap().data, b"first");
        lib.set_album_cover_from_track("Al", t2 - 1).unwrap();
        assert_eq!(lib.album_art("Al").unwrap().unwrap().data, b"second");
        lib.clear_album_cover("Al").unwrap();
        assert_eq!(lib.album_art("Al").unwrap().unwrap().data, b"first");
    }

    #[test]
    fn playlists_keep_order_and_counts() {
        let lib = Library::open_in_memory().unwrap();
        let (a, _) = lib.upsert_track(&song("/m/a.mp3", "A", "X", "")).unwrap();
        let (b, _) = lib.upsert_track(&song("/m/b.mp3", "B", "X", "")).unwrap();
        let p = lib.create_playlist("Mix").unwrap();
        let q = lib.create_playlist("Another").unwrap();
        lib.add_to_playlist(p, &[b, a, b]).unwrap();
        let ids: Vec<i64> = lib.playlist_tracks(p).unwrap().iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![b, a]);
        let names: Vec<String> = lib.playlists().unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["Mix", "Another"]); // creation order, not alphabetical
        lib.reorder_playlists(&[q, p]).unwrap();
        assert_eq!(lib.playlists().unwrap()[0].id, q);
        assert_eq!(lib.playlists().unwrap()[1].count, 2);
    }

    #[test]
    fn settings_default_and_round_trip() {
        let lib = Library::open_in_memory().unwrap();
        let mut s = lib.app_settings().unwrap();
        assert_eq!(s, AppSettings::default());
        s.stats_enabled = false;
        s.discord_client_id = "  123 ".into();
        lib.save_app_settings(&s).unwrap();
        let back = lib.app_settings().unwrap();
        assert!(!back.stats_enabled);
        assert_eq!(back.discord_client_id, "123");
    }
}
