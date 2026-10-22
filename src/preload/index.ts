import { contextBridge, ipcRenderer } from 'electron'
import type {
  AppInfo,
  AppSettings,
  LibraryQuery,
  PlayEvent,
  Playlist,
  PlaylistWithCount,
  PresencePayload,
  ScanProgress,
  ScanResult,
  Track,
  UpdateCheck
} from '../shared/types'

const api = {
  /** Host OS, so the UI can adapt chrome (e.g. macOS traffic-light spacing). */
  platform: process.platform,
  appInfo: (): Promise<AppInfo> => ipcRenderer.invoke('app:info'),
  checkForUpdate: (): Promise<UpdateCheck> => ipcRenderer.invoke('app:checkForUpdate'),

  pickFolder: (): Promise<string[]> => ipcRenderer.invoke('dialog:pickFolder'),

  folders: (): Promise<string[]> => ipcRenderer.invoke('library:folders'),
  addFolder: (p: string): Promise<void> => ipcRenderer.invoke('library:addFolder', p),
  removeFolder: (p: string): Promise<void> => ipcRenderer.invoke('library:removeFolder', p),
  scan: (): Promise<ScanResult> => ipcRenderer.invoke('library:scan'),
  onScanProgress: (cb: (p: ScanProgress) => void): (() => void) => {
    const listener = (_e: unknown, p: ScanProgress): void => cb(p)
    ipcRenderer.on('library:scanProgress', listener)
    return () => ipcRenderer.removeListener('library:scanProgress', listener)
  },

  query: (q: LibraryQuery): Promise<Track[]> => ipcRenderer.invoke('library:query', q),
  artists: (): Promise<{ artist: string; count: number }[]> =>
    ipcRenderer.invoke('library:artists'),
  albums: (): Promise<{ album: string; artist: string; count: number }[]> =>
    ipcRenderer.invoke('library:albums'),
  stats: (): Promise<{ tracks: number; artists: number; albums: number }> =>
    ipcRenderer.invoke('library:stats'),
  getTrack: (id: number): Promise<Track | null> => ipcRenderer.invoke('track:get', id),
  revealTrack: (id: number): Promise<void> => ipcRenderer.invoke('track:reveal', id),

  renameTrack: (id: number, title: string): Promise<void> =>
    ipcRenderer.invoke('meta:renameTrack', id, title),
  renameArtist: (oldName: string, newName: string): Promise<void> =>
    ipcRenderer.invoke('meta:renameArtist', oldName, newName),
  renameAlbum: (oldName: string, newName: string): Promise<void> =>
    ipcRenderer.invoke('meta:renameAlbum', oldName, newName),
  setTrackAlbum: (trackIds: number[], album: string): Promise<void> =>
    ipcRenderer.invoke('meta:setTrackAlbum', trackIds, album),
  setTrackNote: (id: number, note: string): Promise<void> =>
    ipcRenderer.invoke('meta:setTrackNote', id, note),
  reorderAlbum: (album: string, orderedTrackIds: number[]): Promise<void> =>
    ipcRenderer.invoke('album:reorder', album, orderedTrackIds),

  playlists: (): Promise<PlaylistWithCount[]> => ipcRenderer.invoke('playlist:list'),
  createPlaylist: (name: string): Promise<Playlist> => ipcRenderer.invoke('playlist:create', name),
  deletePlaylist: (id: number): Promise<void> => ipcRenderer.invoke('playlist:delete', id),
  renamePlaylist: (id: number, name: string): Promise<void> =>
    ipcRenderer.invoke('playlist:rename', id, name),
  reorderPlaylists: (orderedIds: number[]): Promise<void> =>
    ipcRenderer.invoke('playlist:reorder', orderedIds),
  addToPlaylist: (id: number, trackIds: number[]): Promise<void> =>
    ipcRenderer.invoke('playlist:add', id, trackIds),
  removeFromPlaylist: (id: number, trackId: number): Promise<void> =>
    ipcRenderer.invoke('playlist:remove', id, trackId),
  playlistTracks: (id: number): Promise<Track[]> => ipcRenderer.invoke('playlist:tracks', id),

  getSettings: (): Promise<AppSettings> => ipcRenderer.invoke('settings:get'),
  setSettings: (patch: Partial<AppSettings>): Promise<AppSettings> =>
    ipcRenderer.invoke('settings:set', patch),
  setPresence: (payload: PresencePayload | null): Promise<void> =>
    ipcRenderer.invoke('presence:set', payload),

  mediaUrl: (trackId: number): string => `lumen-media://t/${trackId}`,
  artUrl: (trackId: number): string => `lumen-art://t/${trackId}`,

  albumArtUrl: (album: string): string => `lumen-art://album/${encodeURIComponent(album)}`,
  setAlbumCoverFromTrack: (album: string, trackId: number): Promise<void> =>
    ipcRenderer.invoke('album:coverFromTrack', album, trackId),
  /** Opens a native image picker; resolves true if a cover was set, false if cancelled. */
  setAlbumCoverCustom: (album: string): Promise<boolean> =>
    ipcRenderer.invoke('album:coverCustom', album),
  clearAlbumCover: (album: string): Promise<void> =>
    ipcRenderer.invoke('album:clearCover', album),
  albumHasCover: (album: string): Promise<boolean> =>
    ipcRenderer.invoke('album:hasCover', album),

  /** Record a finished listening session (fire-and-forget from the player). */
  recordPlay: (trackId: number, seconds: number, counted: boolean): Promise<void> =>
    ipcRenderer.invoke('stats:recordPlay', trackId, seconds, counted),
  /** The full play log, aggregated by the stats view into the leaderboard. */
  playHistory: (): Promise<PlayEvent[]> => ipcRenderer.invoke('stats:history'),
  /** Wipe all saved play history. */
  clearPlayHistory: (): Promise<void> => ipcRenderer.invoke('stats:clear')
}

contextBridge.exposeInMainWorld('lumen', api)

export type LumenApi = typeof api
