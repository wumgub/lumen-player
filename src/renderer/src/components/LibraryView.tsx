import React, { useEffect, useMemo, useState } from 'react'
import type { PlaylistWithCount, Track } from '@shared/types'
import type { SortKey, SortState, View } from '../types'
import { usePlayer } from '../context/PlayerContext'
import { formatTime, pluralize } from '../lib/format'
import { Artwork } from './Artwork'
import { Disc, Pencil } from './icons'
import { RenameDialog, type RenameRequest } from './RenameDialog'
import { SortControl } from './SortControl'
import { AlbumPickerDialog, type AlbumPickRequest } from './AlbumPickerDialog'

export function LibraryView({
  view,
  setView,
  search,
  libraryVersion,
  onLibraryChanged
}: {
  view: View
  setView: (v: View) => void
  search: string
  libraryVersion: number
  onLibraryChanged: () => void
}): React.ReactElement {
  const [rename, setRename] = useState<RenameRequest | null>(null)
  const [albumPick, setAlbumPick] = useState<AlbumPickRequest | null>(null)

  let body: React.ReactElement
  if (view.kind === 'artists')
    body = (
      <ArtistGrid
        setView={setView}
        version={libraryVersion}
        requestRename={setRename}
        onLibraryChanged={onLibraryChanged}
      />
    )
  else if (view.kind === 'albums')
    body = (
      <AlbumGrid
        setView={setView}
        version={libraryVersion}
        requestRename={setRename}
        onLibraryChanged={onLibraryChanged}
      />
    )
  else
    body = (
      <TrackList
        view={view}
        setView={setView}
        search={search}
        libraryVersion={libraryVersion}
        onLibraryChanged={onLibraryChanged}
        requestRename={setRename}
        requestAlbumPick={setAlbumPick}
      />
    )

  return (
    <>
      {body}
      <RenameDialog request={rename} onClose={() => setRename(null)} />
      <AlbumPickerDialog request={albumPick} onClose={() => setAlbumPick(null)} />
    </>
  )
}

function headerFor(view: View): { title: string; sub?: string } {
  switch (view.kind) {
    case 'songs':
      return { title: 'All Songs' }
    case 'artist':
      return { title: view.name }
    case 'album':
      return { title: view.name }
    case 'playlist':
      return { title: view.name, sub: 'Playlist' }
    default:
      return { title: '' }
  }
}

function viewKeyOf(v: View): string {
  if (v.kind === 'album' || v.kind === 'artist') return `${v.kind}:${v.name}`
  if (v.kind === 'playlist') return `playlist:${v.id}`
  return v.kind
}

function defaultSort(v: View): SortState {
  if (v.kind === 'album') return { key: 'custom', dir: 'asc' }
  if (v.kind === 'artist') return { key: 'album', dir: 'asc' }
  return { key: 'title', dir: 'asc' }
}

const SORT_OPTIONS: SortKey[] = ['title', 'artist', 'album', 'addedAt', 'duration']

function reorder<T>(arr: T[], from: number, to: number): T[] {
  const next = arr.slice()
  const [moved] = next.splice(from, 1)
  next.splice(to, 0, moved)
  return next
}

function TrackList({
  view,
  setView,
  search,
  libraryVersion,
  onLibraryChanged,
  requestRename,
  requestAlbumPick
}: {
  view: View
  setView: (v: View) => void
  search: string
  libraryVersion: number
  onLibraryChanged: () => void
  requestRename: (r: RenameRequest) => void
  requestAlbumPick: (r: AlbumPickRequest) => void
}): React.ReactElement {
  const player = usePlayer()
  const [tracks, setTracks] = useState<Track[]>([])
  const [playlists, setPlaylists] = useState<PlaylistWithCount[]>([])
  const [menuFor, setMenuFor] = useState<number | null>(null)
  const [selected, setSelected] = useState<Set<number>>(new Set())
  const [sort, setSort] = useState<SortState>(() => defaultSort(view))
  const [dragIndex, setDragIndex] = useState<number | null>(null)
  const [overIndex, setOverIndex] = useState<number | null>(null)

  const viewKey = viewKeyOf(view)

  // Reset sort + selection whenever the view identity changes.
  useEffect(() => {
    setSort(defaultSort(view))
    setSelected(new Set())
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [viewKey])

  useEffect(() => {
    let active = true
    const load = async (): Promise<void> => {
      let rows: Track[]
      if (view.kind === 'playlist') rows = await window.lumen.playlistTracks(view.id)
      else
        rows = await window.lumen.query({
          search: search || undefined,
          artist: view.kind === 'artist' ? view.name : undefined,
          album: view.kind === 'album' ? view.name : undefined,
          sort: sort.key,
          dir: sort.dir
        })
      if (active) setTracks(rows)
    }
    void load()
    return () => {
      active = false
    }
  }, [viewKey, search, libraryVersion, sort, view])

  useEffect(() => {
    void window.lumen.playlists().then(setPlaylists)
  }, [libraryVersion, menuFor])

  const header = headerFor(view)
  const totalDuration = useMemo(() => tracks.reduce((a, t) => a + t.duration, 0), [tracks])

  const dragEnabled = view.kind === 'album' && sort.key === 'custom' && !search

  const addTo = async (playlistId: number, trackId: number): Promise<void> => {
    await window.lumen.addToPlaylist(playlistId, [trackId])
    setMenuFor(null)
    onLibraryChanged()
  }
  const removeFrom = async (trackId: number): Promise<void> => {
    if (view.kind !== 'playlist') return
    await window.lumen.removeFromPlaylist(view.id, trackId)
    setMenuFor(null)
    onLibraryChanged()
  }

  const renameSong = (t: Track): void => {
    setMenuFor(null)
    requestRename({
      kind: 'song',
      label: 'Display name for this track',
      initial: t.title,
      onSubmit: async (v) => {
        await window.lumen.renameTrack(t.id, v)
        onLibraryChanged()
      }
    })
  }

  const openAlbumPick = (ids: number[]): void => {
    setMenuFor(null)
    requestAlbumPick({
      trackIds: ids,
      onDone: () => {
        setSelected(new Set())
        onLibraryChanged()
      }
    })
  }

  const renameHeader = (): void => {
    if (view.kind === 'artist')
      requestRename({
        kind: 'artist',
        label: 'Renames this artist across every track',
        initial: view.name,
        onSubmit: async (v) => {
          await window.lumen.renameArtist(view.name, v)
          onLibraryChanged()
          setView({ kind: 'artist', name: v })
        }
      })
    else if (view.kind === 'album')
      requestRename({
        kind: 'album',
        label: 'Renames this album across every track',
        initial: view.name,
        onSubmit: async (v) => {
          await window.lumen.renameAlbum(view.name, v)
          onLibraryChanged()
          setView({ kind: 'album', name: v })
        }
      })
  }

  const toggleSelect = (id: number): void =>
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })

  const onDrop = (to: number): void => {
    if (dragIndex === null || dragIndex === to) {
      setDragIndex(null)
      setOverIndex(null)
      return
    }
    const next = reorder(tracks, dragIndex, to)
    setTracks(next)
    setDragIndex(null)
    setOverIndex(null)
    if (view.kind === 'album')
      void window.lumen.reorderAlbum(
        view.name,
        next.map((t) => t.id)
      )
  }

  const canRenameHeader = view.kind === 'artist' || view.kind === 'album'

  return (
    <div className="track-view">
      <header className="view-header">
        <div>
          <h1 className="view-title">
            {header.title}
            {canRenameHeader && (
              <button className="title-edit" onClick={renameHeader} title={`Rename ${view.kind}`}>
                <Pencil size={16} />
              </button>
            )}
          </h1>
          <div className="view-sub">
            {header.sub ? header.sub + ' · ' : ''}
            {pluralize(tracks.length, 'song')}
            {totalDuration > 0 && ` · ${formatTime(totalDuration)}`}
            {dragEnabled && tracks.length > 1 && ' · drag to reorder'}
          </div>
        </div>
        <div className="header-actions">
          {view.kind !== 'playlist' && (
            <SortControl
              sort={sort}
              options={view.kind === 'album' ? ['custom', ...SORT_OPTIONS] : SORT_OPTIONS}
              onChange={setSort}
            />
          )}
          {tracks.length > 0 && (
            <button className="play-all" onClick={() => player.playQueue(tracks, 0)}>
              Play all
            </button>
          )}
        </div>
      </header>

      {tracks.length === 0 ? (
        <div className="empty-state">
          <Disc size={40} />
          <p>{search ? 'No matches' : 'Nothing here yet — add a folder and scan.'}</p>
        </div>
      ) : (
        <div className="track-rows">
          {tracks.map((t, i) => {
            const isCurrent = player.current?.id === t.id
            const isSelected = selected.has(t.id)
            return (
              <div
                key={`${t.id}-${i}`}
                className={`track-row ${isCurrent ? 'current' : ''} ${
                  isSelected ? 'selected' : ''
                } ${overIndex === i && dragIndex !== null ? 'drop-target' : ''} ${
                  dragEnabled ? 'draggable' : ''
                }`}
                draggable={dragEnabled}
                onDragStart={() => dragEnabled && setDragIndex(i)}
                onDragOver={(e) => {
                  if (dragEnabled && dragIndex !== null) {
                    e.preventDefault()
                    setOverIndex(i)
                  }
                }}
                onDrop={() => dragEnabled && onDrop(i)}
                onDragEnd={() => {
                  setDragIndex(null)
                  setOverIndex(null)
                }}
                onClick={(e) => {
                  // Clicking the row body toggles selection; buttons stop propagation.
                  if ((e.target as HTMLElement).closest('button')) return
                  toggleSelect(t.id)
                }}
                onDoubleClick={() => player.playQueue(tracks, i)}
              >
                <div className="tr-index">
                  {dragEnabled ? (
                    <span className="tr-grip" title="Drag to reorder">
                      ⠿
                    </span>
                  ) : isCurrent && player.isPlaying ? (
                    <span className="eq">
                      <i />
                      <i />
                      <i />
                    </span>
                  ) : (
                    <span className="tr-num">{i + 1}</span>
                  )}
                  <button
                    className="tr-play"
                    onClick={() => player.playQueue(tracks, i)}
                    aria-label="Play"
                  />
                </div>
                <Artwork trackId={t.id} hasArt={!!t.hasArt} className="tr-art" size={16} />
                <div className="tr-main">
                  <div className="tr-title">{t.title}</div>
                  <div className="tr-artist">{t.artist}</div>
                </div>
                <div className="tr-album">{t.album}</div>
                <div className="tr-time">{formatTime(t.duration)}</div>
                <div className="tr-actions">
                  <button
                    className="tr-more"
                    onClick={() => setMenuFor(menuFor === t.id ? null : t.id)}
                  >
                    ⋯
                  </button>
                  {menuFor === t.id && (
                    <div className="row-menu" onMouseLeave={() => setMenuFor(null)}>
                      <button onClick={() => renameSong(t)}>Rename song…</button>
                      <button onClick={() => openAlbumPick([t.id])}>Add to album…</button>
                      <div className="row-menu-label">Add to playlist</div>
                      {playlists.length === 0 && <div className="row-menu-empty">No playlists</div>}
                      {playlists.map((pl) => (
                        <button key={pl.id} onClick={() => addTo(pl.id, t.id)}>
                          {pl.name}
                        </button>
                      ))}
                      {view.kind === 'playlist' && (
                        <button className="danger" onClick={() => removeFrom(t.id)}>
                          Remove from this playlist
                        </button>
                      )}
                    </div>
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}

      {selected.size > 0 && (
        <div className="selection-bar">
          <span className="sel-count">{pluralize(selected.size, 'song')} selected</span>
          <button className="sel-btn" onClick={() => openAlbumPick([...selected])}>
            Add to album…
          </button>
          <button className="sel-btn ghost" onClick={() => setSelected(new Set())}>
            Clear
          </button>
        </div>
      )}
    </div>
  )
}

function ArtistGrid({
  setView,
  version,
  requestRename,
  onLibraryChanged
}: {
  setView: (v: View) => void
  version: number
  requestRename: (r: RenameRequest) => void
  onLibraryChanged: () => void
}): React.ReactElement {
  const [artists, setArtists] = useState<{ artist: string; count: number }[]>([])
  useEffect(() => {
    void window.lumen.artists().then(setArtists)
  }, [version])

  const rename = (name: string): void =>
    requestRename({
      kind: 'artist',
      label: 'Renames this artist across every track',
      initial: name,
      onSubmit: async (v) => {
        await window.lumen.renameArtist(name, v)
        onLibraryChanged()
      }
    })

  return (
    <div className="grid-view">
      <header className="view-header">
        <h1>Artists</h1>
        <div className="view-sub">{pluralize(artists.length, 'artist')}</div>
      </header>
      <div className="card-grid">
        {artists.map((a) => (
          <div
            key={a.artist}
            className="card artist-card"
            role="button"
            tabIndex={0}
            onClick={() => setView({ kind: 'artist', name: a.artist })}
          >
            <div className="card-circle" style={gradientFor(a.artist)}>
              {initials(a.artist)}
            </div>
            <div className="card-title">{a.artist}</div>
            <div className="card-sub">{pluralize(a.count, 'song')}</div>
            <button
              className="card-edit"
              title="Rename artist"
              onClick={(e) => {
                e.stopPropagation()
                rename(a.artist)
              }}
            >
              <Pencil size={14} />
            </button>
          </div>
        ))}
      </div>
    </div>
  )
}

function AlbumGrid({
  setView,
  version,
  requestRename,
  onLibraryChanged
}: {
  setView: (v: View) => void
  version: number
  requestRename: (r: RenameRequest) => void
  onLibraryChanged: () => void
}): React.ReactElement {
  const [albums, setAlbums] = useState<{ album: string; artist: string; count: number }[]>([])
  useEffect(() => {
    void window.lumen.albums().then(setAlbums)
  }, [version])

  const rename = (name: string): void =>
    requestRename({
      kind: 'album',
      label: 'Renames this album across every track',
      initial: name,
      onSubmit: async (v) => {
        await window.lumen.renameAlbum(name, v)
        onLibraryChanged()
      }
    })

  return (
    <div className="grid-view">
      <header className="view-header">
        <h1>Albums</h1>
        <div className="view-sub">{pluralize(albums.length, 'album')}</div>
      </header>
      <div className="card-grid">
        {albums.map((a) => (
          <div
            key={a.album}
            className="card album-card"
            role="button"
            tabIndex={0}
            onClick={() => setView({ kind: 'album', name: a.album })}
          >
            <div className="card-cover" style={gradientFor(a.album)}>
              <Disc size={28} />
            </div>
            <div className="card-title">{a.album}</div>
            <div className="card-sub">{a.artist}</div>
            <button
              className="card-edit"
              title="Rename album"
              onClick={(e) => {
                e.stopPropagation()
                rename(a.album)
              }}
            >
              <Pencil size={14} />
            </button>
          </div>
        ))}
      </div>
    </div>
  )
}

function initials(name: string): string {
  return name
    .split(/\s+/)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase() ?? '')
    .join('')
}

function gradientFor(seed: string): React.CSSProperties {
  let h = 0
  for (let i = 0; i < seed.length; i++) h = (h * 31 + seed.charCodeAt(i)) % 360
  const h2 = (h + 50) % 360
  return {
    background: `linear-gradient(135deg, hsl(${h} 60% 45%), hsl(${h2} 65% 35%))`
  }
}
