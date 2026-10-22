export interface Track {
  id: number
  path: string
  title: string
  artist: string
  albumArtist: string
  album: string
  genre: string
  year: number | null
  trackNo: number | null
  discNo: number | null
  duration: number // seconds
  hasArt: number // 0 | 1 (sqlite boolean)
  addedAt: number // epoch ms
  note: string // user's free-text note; also matched by search
  isVideo: number // 0 | 1 — has a video track (e.g. an .mp4 music video)
}

/** One listening session for a track, used by the stats/leaderboard view. */
export interface PlayEvent {
  trackId: number
  playedAt: number // epoch ms
  seconds: number // whole seconds actually listened in this session
  counted: number // 1 if it crossed the "counts as a play" threshold, else 0
}

export interface ScanProgress {
  phase: 'discovering' | 'parsing' | 'done'
  processed: number
  total: number
  current?: string
}

export interface ScanResult {
  added: number
  updated: number
  total: number
}

export interface LibraryQuery {
  search?: string
  artist?: string
  album?: string
  genre?: string
  /** Match tracks whose file path is under this folder. */
  folder?: string
  sort?: 'title' | 'artist' | 'album' | 'addedAt' | 'year' | 'duration' | 'custom'
  dir?: 'asc' | 'desc'
  limit?: number
  offset?: number
}

export interface Playlist {
  id: number
  name: string
  createdAt: number
}

export interface PlaylistWithCount extends Playlist {
  count: number
}

export interface AppSettings {
  /** Record play history for the stats/leaderboard view. Off = nothing saved. */
  statsEnabled: boolean
  discordEnabled: boolean
  discordClientId: string
  /** App Client Secret — needed only to register external cover art with Discord. */
  discordClientSecret: string
  /** Asset key uploaded to your Discord app, used as the fallback large image. */
  discordLargeImageKey: string
  /** Fetch public per-track cover art online (iTunes) for the presence image. */
  discordCoverArt: boolean
}

export interface AppInfo {
  version: string
  commit: string // short git SHA the build was made from
  buildDate: string // ISO timestamp
}

/** Result of checking GitHub for a newer build. */
export type UpdateCheck =
  | {
      ok: true
      current: string
      latest: string
      date: string | null
      message: string
      upToDate: boolean
      url: string
    }
  | { ok: false; error: string; url: string }

/** Playback info the renderer sends to the main process for Discord presence. */
export interface PresencePayload {
  title: string
  artist: string
  album: string
  isPlaying: boolean
  position: number // seconds
  duration: number // seconds
}

export const AUDIO_EXTENSIONS = [
  '.mp3',
  '.flac',
  '.wav',
  '.aif',
  '.aiff',
  '.m4a',
  '.aac',
  '.ogg',
  '.oga',
  '.opus'
] as const

// Video containers whose audio Chromium can play and whose picture we can show
// (music videos). Parsed for tags/duration by music-metadata just like audio.
export const VIDEO_EXTENSIONS = ['.mp4', '.m4v', '.webm', '.mov'] as const

/** Everything the scanner indexes. */
export const MEDIA_EXTENSIONS = [...AUDIO_EXTENSIONS, ...VIDEO_EXTENSIONS] as const

/** True when a file path has a video-container extension. */
export function isVideoPath(path: string): boolean {
  const dot = path.lastIndexOf('.')
  if (dot < 0) return false
  return (VIDEO_EXTENSIONS as readonly string[]).includes(path.slice(dot).toLowerCase())
}
