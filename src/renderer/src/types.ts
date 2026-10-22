export type View =
  | { kind: 'songs' }
  | { kind: 'artists' }
  | { kind: 'albums' }
  | { kind: 'artist'; name: string }
  | { kind: 'album'; name: string }
  | { kind: 'folder'; path: string }
  | { kind: 'playlist'; id: number; name: string }
  | { kind: 'stats' }

export type SortKey = 'custom' | 'title' | 'artist' | 'album' | 'addedAt' | 'duration'

export interface SortState {
  key: SortKey
  dir: 'asc' | 'desc'
}

export const SORT_LABELS: Record<SortKey, string> = {
  custom: 'Track order',
  title: 'Title',
  artist: 'Artist',
  album: 'Album',
  addedAt: 'Date added',
  duration: 'Duration'
}
