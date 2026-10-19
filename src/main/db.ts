import { app } from 'electron'
import { join } from 'path'
import Database from 'better-sqlite3'
import type {
  AppSettings,
  LibraryQuery,
  Playlist,
  PlaylistWithCount,
  Track
} from '../shared/types'

let db: Database.Database

export function initDb(): void {
  const file = join(app.getPath('userData'), 'library.db')
  db = new Database(file)
  db.pragma('journal_mode = WAL')
  db.pragma('foreign_keys = ON')

  db.exec(`
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
  `)

  // Migration: `locked` is a bitmask of fields the user has renamed, so a
  // rescan won't overwrite their edits with the file's original tags.
  const cols = db.prepare('PRAGMA table_info(tracks)').all() as { name: string }[]
  if (!cols.some((c) => c.name === 'locked')) {
    db.exec('ALTER TABLE tracks ADD COLUMN locked INTEGER NOT NULL DEFAULT 0')
  }
  // `albumPos` is the user's manual order of a track within its album (null = unset).
  if (!cols.some((c) => c.name === 'albumPos')) {
    db.exec('ALTER TABLE tracks ADD COLUMN albumPos INTEGER')
  }
}

// Lock bits for user-edited fields.
const L_TITLE = 1
const L_ARTIST = 2
const L_ALBUM = 4

export interface UpsertTrack {
  path: string
  title: string
  artist: string
  albumArtist: string
  album: string
  genre: string
  year: number | null
  trackNo: number | null
  discNo: number | null
  duration: number
  art?: { mime: string; data: Buffer } | null
}

const COLS =
  'id, path, title, artist, albumArtist, album, genre, year, trackNo, discNo, duration, hasArt, addedAt'

export function upsertTrack(t: UpsertTrack): { id: number; isNew: boolean } {
  const existing = db
    .prepare('SELECT id, title, artist, albumArtist, album, locked FROM tracks WHERE path = ?')
    .get(t.path) as
    | { id: number; title: string; artist: string; albumArtist: string; album: string; locked: number }
    | undefined
  const hasArt = t.art ? 1 : 0

  const txn = db.transaction(() => {
    let id: number
    if (existing) {
      const locked = existing.locked
      // Preserve any field the user has renamed; refresh the rest from tags.
      const resolved = {
        title: locked & L_TITLE ? existing.title : t.title,
        artist: locked & L_ARTIST ? existing.artist : t.artist,
        albumArtist: locked & L_ARTIST ? existing.albumArtist : t.albumArtist,
        album: locked & L_ALBUM ? existing.album : t.album
      }
      db.prepare(
        `UPDATE tracks SET title=@title, artist=@artist, albumArtist=@albumArtist, album=@album,
         genre=@genre, year=@year, trackNo=@trackNo, discNo=@discNo, duration=@duration, hasArt=@hasArt
         WHERE path=@path`
      ).run({ ...t, ...resolved, hasArt })
      id = existing.id
    } else {
      const info = db
        .prepare(
          `INSERT INTO tracks (path, title, artist, albumArtist, album, genre, year, trackNo, discNo, duration, hasArt, addedAt)
           VALUES (@path, @title, @artist, @albumArtist, @album, @genre, @year, @trackNo, @discNo, @duration, @hasArt, @addedAt)`
        )
        .run({ ...t, hasArt, addedAt: Date.now() })
      id = Number(info.lastInsertRowid)
    }
    if (t.art) {
      db.prepare(
        `INSERT INTO art (track_id, mime, data) VALUES (?, ?, ?)
         ON CONFLICT(track_id) DO UPDATE SET mime=excluded.mime, data=excluded.data`
      ).run(id, t.art.mime, t.art.data)
    }
    return id
  })

  const id = txn()
  return { id, isNew: !existing }
}

export function getArt(trackId: number): { mime: string; data: Buffer } | null {
  const row = db.prepare('SELECT mime, data FROM art WHERE track_id = ?').get(trackId) as
    | { mime: string; data: Buffer }
    | undefined
  return row ?? null
}

export function getTrack(id: number): Track | null {
  return (db.prepare(`SELECT ${COLS} FROM tracks WHERE id = ?`).get(id) as Track) ?? null
}

// ---- renames (non-destructive: DB only, files untouched; locked against rescan) ----
export function renameTrack(id: number, title: string): void {
  db.prepare('UPDATE tracks SET title = ?, locked = locked | ? WHERE id = ?').run(
    title,
    L_TITLE,
    id
  )
}

export function renameArtist(oldName: string, newName: string): void {
  // Rewrite the effective artist (albumArtist when set, else artist) for the whole group.
  db.prepare(
    `UPDATE tracks SET
       albumArtist = CASE WHEN albumArtist = @old THEN @new ELSE albumArtist END,
       artist = CASE WHEN albumArtist = '' AND artist = @old THEN @new ELSE artist END,
       locked = locked | @bit
     WHERE albumArtist = @old OR (albumArtist = '' AND artist = @old)`
  ).run({ old: oldName, new: newName, bit: L_ARTIST })
}

export function renameAlbum(oldName: string, newName: string): void {
  db.prepare('UPDATE tracks SET album = @new, locked = locked | @bit WHERE album = @old').run({
    old: oldName,
    new: newName,
    bit: L_ALBUM
  })
}

/** Assign tracks to an album by name (creating the group if it's new). */
export function setTrackAlbum(trackIds: number[], album: string): void {
  const upd = db.prepare(
    'UPDATE tracks SET album = @album, albumPos = NULL, locked = locked | @bit WHERE id = @id'
  )
  const txn = db.transaction(() => {
    for (const id of trackIds) upd.run({ id, album, bit: L_ALBUM })
  })
  txn()
}

/** Persist a manual track order within an album. */
export function reorderAlbum(album: string, orderedTrackIds: number[]): void {
  const upd = db.prepare('UPDATE tracks SET albumPos = @pos WHERE id = @id AND album = @album')
  const txn = db.transaction(() => {
    orderedTrackIds.forEach((id, i) => upd.run({ pos: i, id, album }))
  })
  txn()
}

export function queryTracks(q: LibraryQuery): Track[] {
  const where: string[] = []
  const params: Record<string, unknown> = {}
  if (q.search) {
    where.push('(title LIKE @s OR artist LIKE @s OR album LIKE @s)')
    params.s = `%${q.search}%`
  }
  if (q.artist) {
    // Match the effective artist (albumArtist when present) used by the Artists view.
    where.push("(CASE WHEN albumArtist != '' THEN albumArtist ELSE artist END) = @artist")
    params.artist = q.artist
  }
  if (q.album) {
    where.push('album = @album')
    params.album = q.album
  }
  if (q.genre) {
    where.push('genre = @genre')
    params.genre = q.genre
  }
  const dir = q.dir === 'desc' ? 'DESC' : 'ASC'
  let orderBy: string
  if (q.sort === 'custom') {
    // Manual album order: user-positioned tracks first (null = unset, sorts last).
    orderBy = `albumPos IS NULL ${dir}, albumPos ${dir}, discNo ASC, trackNo ASC, title ASC`
  } else {
    const sortCol = ['title', 'artist', 'album', 'addedAt', 'year', 'duration'].includes(
      q.sort ?? ''
    )
      ? q.sort
      : 'artist'
    orderBy = `${sortCol} ${dir}, album ASC, discNo ASC, trackNo ASC, title ASC`
  }
  const limit = q.limit ? `LIMIT ${Math.max(0, q.limit | 0)}` : ''
  const offset = q.offset ? `OFFSET ${Math.max(0, q.offset | 0)}` : ''
  const sql = `SELECT ${COLS} FROM tracks
    ${where.length ? 'WHERE ' + where.join(' AND ') : ''}
    ORDER BY ${orderBy}
    ${limit} ${offset}`
  return db.prepare(sql).all(params) as Track[]
}

export function listArtists(): { artist: string; count: number }[] {
  return db
    .prepare(
      `SELECT CASE WHEN albumArtist != '' THEN albumArtist ELSE artist END AS artist,
       COUNT(*) AS count FROM tracks GROUP BY artist ORDER BY artist COLLATE NOCASE ASC`
    )
    .all() as { artist: string; count: number }[]
}

export function listAlbums(): { album: string; artist: string; count: number }[] {
  return db
    .prepare(
      `SELECT album, MAX(CASE WHEN albumArtist != '' THEN albumArtist ELSE artist END) AS artist,
       COUNT(*) AS count FROM tracks WHERE album != '' GROUP BY album ORDER BY album COLLATE NOCASE ASC`
    )
    .all() as { album: string; artist: string; count: number }[]
}

export function stats(): { tracks: number; artists: number; albums: number } {
  const tracks = (db.prepare('SELECT COUNT(*) c FROM tracks').get() as { c: number }).c
  const artists = (
    db.prepare('SELECT COUNT(DISTINCT artist) c FROM tracks').get() as { c: number }
  ).c
  const albums = (
    db.prepare("SELECT COUNT(DISTINCT album) c FROM tracks WHERE album != ''").get() as {
      c: number
    }
  ).c
  return { tracks, artists, albums }
}

export function listAllPaths(): string[] {
  return (db.prepare('SELECT path FROM tracks').all() as { path: string }[]).map((r) => r.path)
}

export function removeMissing(existingPaths: Set<string>): number {
  const all = db.prepare('SELECT id, path FROM tracks').all() as { id: number; path: string }[]
  const del = db.prepare('DELETE FROM tracks WHERE id = ?')
  let removed = 0
  const txn = db.transaction(() => {
    for (const row of all) {
      if (!existingPaths.has(row.path)) {
        del.run(row.id)
        removed++
      }
    }
  })
  txn()
  return removed
}

// ---- settings ----
export function getSetting(key: string, fallback = ''): string {
  const row = db.prepare('SELECT value FROM settings WHERE key = ?').get(key) as
    | { value: string }
    | undefined
  return row?.value ?? fallback
}
export function setSetting(key: string, value: string): void {
  db.prepare(
    'INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value'
  ).run(key, value)
}
export function getAppSettings(): AppSettings {
  return {
    discordEnabled: getSetting('discordEnabled', 'false') === 'true',
    discordClientId: getSetting('discordClientId', ''),
    discordLargeImageKey: getSetting('discordLargeImageKey', 'lumen')
  }
}
export function setAppSettings(p: Partial<AppSettings>): AppSettings {
  if (p.discordEnabled !== undefined) setSetting('discordEnabled', String(p.discordEnabled))
  if (p.discordClientId !== undefined) setSetting('discordClientId', p.discordClientId.trim())
  if (p.discordLargeImageKey !== undefined)
    setSetting('discordLargeImageKey', p.discordLargeImageKey.trim())
  return getAppSettings()
}

// ---- folders ----
export function addFolder(path: string): void {
  db.prepare('INSERT OR IGNORE INTO folders (path) VALUES (?)').run(path)
}
export function listFolders(): string[] {
  return (db.prepare('SELECT path FROM folders ORDER BY path').all() as { path: string }[]).map(
    (r) => r.path
  )
}
export function removeFolder(path: string): void {
  db.prepare('DELETE FROM folders WHERE path = ?').run(path)
}

// ---- playlists ----
export function createPlaylist(name: string): Playlist {
  const info = db
    .prepare('INSERT INTO playlists (name, createdAt) VALUES (?, ?)')
    .run(name, Date.now())
  return getPlaylist(Number(info.lastInsertRowid))!
}
export function getPlaylist(id: number): Playlist | null {
  return (db.prepare('SELECT * FROM playlists WHERE id = ?').get(id) as Playlist) ?? null
}
export function listPlaylists(): PlaylistWithCount[] {
  return db
    .prepare(
      `SELECT p.*, COUNT(pt.track_id) AS count FROM playlists p
       LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
       GROUP BY p.id ORDER BY p.name COLLATE NOCASE`
    )
    .all() as PlaylistWithCount[]
}
export function deletePlaylist(id: number): void {
  db.prepare('DELETE FROM playlists WHERE id = ?').run(id)
}
export function renamePlaylist(id: number, name: string): void {
  db.prepare('UPDATE playlists SET name = ? WHERE id = ?').run(name, id)
}
export function addToPlaylist(playlistId: number, trackIds: number[]): void {
  const max =
    (
      db
        .prepare('SELECT COALESCE(MAX(position), -1) m FROM playlist_tracks WHERE playlist_id = ?')
        .get(playlistId) as { m: number }
    ).m + 1
  const ins = db.prepare(
    'INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?, ?, ?)'
  )
  const txn = db.transaction(() => {
    trackIds.forEach((tid, i) => ins.run(playlistId, tid, max + i))
  })
  txn()
}
export function removeFromPlaylist(playlistId: number, trackId: number): void {
  db.prepare('DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?').run(
    playlistId,
    trackId
  )
}
export function playlistTracks(playlistId: number): Track[] {
  return db
    .prepare(
      `SELECT ${COLS.split(', ')
        .map((c) => 't.' + c)
        .join(', ')} FROM playlist_tracks pt
       JOIN tracks t ON t.id = pt.track_id
       WHERE pt.playlist_id = ? ORDER BY pt.position ASC`
    )
    .all(playlistId) as Track[]
}
