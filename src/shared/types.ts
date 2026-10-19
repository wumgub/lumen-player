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
  discordEnabled: boolean
  discordClientId: string
  /** App Client Secret — needed only to register external cover art with Discord. */
  discordClientSecret: string
  /** Asset key uploaded to your Discord app, used as the fallback large image. */
  discordLargeImageKey: string
  /** Fetch public per-track cover art online (iTunes) for the presence image. */
  discordCoverArt: boolean
}

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
