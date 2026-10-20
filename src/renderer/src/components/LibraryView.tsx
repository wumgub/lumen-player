import React, { useEffect, useMemo, useRef, useState } from 'react'
import type { PlaylistWithCount, Track } from '@shared/types'
import type { SortKey, SortState, View } from '../types'
import { usePlayer } from '../context/PlayerContext'
import { formatDuration, formatTime, pluralize } from '../lib/format'
import { Artwork } from './Artwork'
import { Disc, Pencil, Play } from './icons'
import { RenameDialog, type RenameRequest } from './RenameDialog'
import { NoteDialog, type NoteRequest } from './NoteDialog'
import { SortControl } from './SortControl'
import { AlbumPickerDialog, type AlbumPickRequest } from './AlbumPickerDialog'
import { TrackContextMenu, type ContextMenuState } from './TrackContextMenu'

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
  const [note, setNote] = useState<NoteRequest | null>(null)
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
        requestNote={setNote}
        requestAlbumPick={setAlbumPick}
      />
    )

  return (
    <>
      {body}
      <RenameDialog request={rename} onClose={() => setRename(null)} />
      <NoteDialog request={note} onClose={() => setNote(null)} />
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
    case 'folder':
      return { title: view.path.split('/').pop() || view.path, sub: 'Folder' }
    default:
      return { title: '' }
  }
}

function viewKeyOf(v: View): string {
  if (v.kind === 'album' || v.kind === 'artist') return `${v.kind}:${v.name}`
  if (v.kind === 'folder') return `folder:${v.path}`
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
  requestNote,
  requestAlbumPick
}: {
  view: View
  setView: (v: View) => void
  search: string
  libraryVersion: number
  onLibraryChanged: () => void
  requestRename: (r: RenameRequest) => void
  requestNote: (r: NoteRequest) => void
  requestAlbumPick: (r: AlbumPickRequest) => void
}): React.ReactElement {
  const player = usePlayer()
  const [tracks, setTracks] = useState<Track[]>([])
  const [playlists, setPlaylists] = useState<PlaylistWithCount[]>([])
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null)
  const [selected, setSelected] = useState<Set<number>>(new Set())
  const [sort, setSort] = useState<SortState>(() => defaultSort(view))
  const [editMode, setEditMode] = useState(false)
  const [dragIndex, setDragIndex] = useState<number | null>(null)
  const [overIndex, setOverIndex] = useState<number | null>(null)
  const [playlistPicker, setPlaylistPicker] = useState(false)
  const playlistPickerRef = useRef<HTMLDivElement>(null)

  const viewKey = viewKeyOf(view)

  // Reset interaction state whenever the view identity changes.
  useEffect(() => {
    setSort(defaultSort(view))
    setSelected(new Set())
    setEditMode(false)
    setContextMenu(null)
    setPlaylistPicker(false)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [viewKey])

  // Close the bulk playlist picker on an outside click.
  useEffect(() => {
    if (!playlistPicker) return
    const onDown = (e: MouseEvent): void => {
      if (playlistPickerRef.current && !playlistPickerRef.current.contains(e.target as Node))
        setPlaylistPicker(false)
    }
    document.addEventListener('mousedown', onDown)
    return () => document.removeEventListener('mousedown', onDown)
  }, [playlistPicker])

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
          folder: view.kind === 'folder' ? view.path : undefined,
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
  }, [libraryVersion, contextMenu])

  const header = headerFor(view)
  const totalDuration = useMemo(() => tracks.reduce((a, t) => a + t.duration, 0), [tracks])

  // Reordering only happens in edit mode, on an album sorted by Track order.
  const reorderable = editMode && view.kind === 'album' && sort.key === 'custom' && !search
  // Elsewhere, edit mode enables multi-select for bulk actions.
  const selectable = editMode && !reorderable

  const toggleEdit = (): void =>
    setEditMode((on) => {
      const next = !on
      if (!next) setSelected(new Set())
      // Entering edit on an album defaults to Track order so dragging works.
      if (next && view.kind === 'album' && sort.key !== 'custom')
        setSort({ key: 'custom', dir: 'asc' })
      return next
    })

  const toggleSelect = (id: number): void =>
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })

  const renameSong = (t: Track): void =>
    requestRename({
      kind: 'song',
      label: 'Display name for this track',
      initial: t.title,
      onSubmit: async (v) => {
        await window.lumen.renameTrack(t.id, v)
        onLibraryChanged()
      }
    })

  const editNote = (t: Track): void =>
    requestNote({
      title: t.title,
      initial: t.note ?? '',
      onSubmit: async (v) => {
        await window.lumen.setTrackNote(t.id, v)
        onLibraryChanged()
      }
    })

  const openAlbumPick = (ids: number[]): void =>
    requestAlbumPick({
      trackIds: ids,
      onDone: () => {
        setSelected(new Set())
        onLibraryChanged()
      }
    })

  const addToPlaylist = async (playlistId: number, trackId: number): Promise<void> => {
    await window.lumen.addToPlaylist(playlistId, [trackId])
    onLibraryChanged()
  }
  const bulkAddToPlaylist = async (playlistId: number): Promise<void> => {
    await window.lumen.addToPlaylist(playlistId, [...selected])
    setPlaylistPicker(false)
    setSelected(new Set())
    onLibraryChanged()
  }
  const removeFromAlbum = async (trackId: number): Promise<void> => {
    await window.lumen.setTrackAlbum([trackId], '')
    onLibraryChanged()
  }
  const removeFromPlaylist = async (trackId: number): Promise<void> => {
    if (view.kind !== 'playlist') return
    await window.lumen.removeFromPlaylist(view.id, trackId)
    onLibraryChanged()
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

  const openMenu = (e: React.MouseEvent, t: Track): void => {
    e.preventDefault()
    e.stopPropagation()
    setContextMenu({ x: e.clientX, y: e.clientY, track: t })
  }

  const onRowClick = (i: number, t: Track): void => {
    if (!editMode) player.playQueue(tracks, i)
    else if (selectable) toggleSelect(t.id)
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
            {totalDuration > 0 && ` · ${formatDuration(totalDuration)}`}
            {reorderable && tracks.length > 1 && ' · drag to reorder'}
            {selectable && ' · click to select'}
          </div>
        </div>
        <div className="header-actions">
          {view.kind !== 'playlist' && !editMode && (
            <SortControl
              sort={sort}
              options={view.kind === 'album' ? ['custom', ...SORT_OPTIONS] : SORT_OPTIONS}
              onChange={setSort}
            />
          )}
          {tracks.length > 0 && (
            <button
              className={`edit-btn ${editMode ? 'active' : ''}`}
              onClick={toggleEdit}
              title={editMode ? 'Finish editing' : 'Reorder / select tracks'}
            >
              {editMode ? 'Done' : 'Edit'}
            </button>
          )}
          {tracks.length > 0 && !editMode && (
            <button className="play-all" onClick={() => player.playQueue(tracks, 0)}>
              <Play size={14} />
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
        <div className={`track-rows ${editMode ? 'editing' : ''}`}>
          {tracks.map((t, i) => {
            const isCurrent = player.current?.id === t.id
            const isSelected = selected.has(t.id)
            return (
              <div
                key={`${t.id}-${i}`}
                className={`track-row ${isCurrent ? 'current' : ''} ${
                  selectable && isSelected ? 'selected' : ''
                } ${overIndex === i && dragIndex !== null ? 'drop-target' : ''} ${
                  reorderable ? 'draggable' : ''
                }`}
                draggable={reorderable}
                onDragStart={() => reorderable && setDragIndex(i)}
                onDragOver={(e) => {
                  if (reorderable && dragIndex !== null) {
                    e.preventDefault()
                    setOverIndex(i)
                  }
                }}
                onDrop={() => reorderable && onDrop(i)}
                onDragEnd={() => {
                  setDragIndex(null)
                  setOverIndex(null)
                }}
                onClick={(e) => {
                  if ((e.target as HTMLElement).closest('button')) return
                  onRowClick(i, t)
                }}
                onContextMenu={(e) => openMenu(e, t)}
              >
                <div className="tr-index">
                  {selectable ? (
                    <span className={`tr-check ${isSelected ? 'checked' : ''}`} />
                  ) : reorderable ? (
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
                  {!editMode && (
                    <button
                      className="tr-play"
                      onClick={() => player.playQueue(tracks, i)}
                      aria-label="Play"
                    />
                  )}
                </div>
                <Artwork trackId={t.id} hasArt={!!t.hasArt} className="tr-art" size={16} />
                <div className="tr-main">
                  <div className="tr-title">{t.title}</div>
                  <div className="tr-artist">{t.artist}</div>
                  {t.note && (
                    <div className="tr-note" title={t.note}>
                      {t.note}
                    </div>
                  )}
                </div>
                <div className="tr-album">{t.album}</div>
                <div className="tr-time">{formatTime(t.duration)}</div>
                <div className="tr-actions">
                  <button className="tr-more" onClick={(e) => openMenu(e, t)} title="More…">
                    ⋯
                  </button>
                </div>
              </div>
            )
          })}
        </div>
      )}

      {selectable && selected.size > 0 && (
        <div className="selection-bar">
          <span className="sel-count">{pluralize(selected.size, 'song')} selected</span>
          <div className="sel-menu-wrap" ref={playlistPickerRef}>
            <button className="sel-btn" onClick={() => setPlaylistPicker((v) => !v)}>
              Add to playlist…
            </button>
            {playlistPicker && (
              <div className="sel-menu">
                {playlists.length === 0 ? (
                  <div className="ctx-empty">No playlists yet</div>
                ) : (
                  <div className="ctx-scroll">
                    {playlists.map((pl) => (
                      <button
                        key={pl.id}
                        className="ctx-item"
                        onClick={() => void bulkAddToPlaylist(pl.id)}
                      >
                        {pl.name}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
          <button
            className="sel-btn"
            onClick={() => {
              player.addToQueue(tracks.filter((t) => selected.has(t.id)))
              setSelected(new Set())
            }}
          >
            Add to queue
          </button>
          <button className="sel-btn" onClick={() => openAlbumPick([...selected])}>
            Add to album…
          </button>
          <button className="sel-btn ghost" onClick={() => setSelected(new Set())}>
            Clear
          </button>
        </div>
      )}

      {contextMenu && (
        <TrackContextMenu
          menu={contextMenu}
          playlists={playlists}
          actions={{
            rename: () => renameSong(contextMenu.track),
            editNote: () => editNote(contextMenu.track),
            showFile: () => void window.lumen.revealTrack(contextMenu.track.id),
            playNext: () => player.playNext([contextMenu.track]),
            addToQueue: () => player.addToQueue([contextMenu.track]),
            addToPlaylist: (plId) => void addToPlaylist(plId, contextMenu.track.id),
            addToAlbum: () => openAlbumPick([contextMenu.track.id]),
            removeFromAlbum: contextMenu.track.album
              ? () => void removeFromAlbum(contextMenu.track.id)
              : undefined,
            removeFromPlaylist:
              view.kind === 'playlist'
                ? () => void removeFromPlaylist(contextMenu.track.id)
                : undefined
          }}
          onClose={() => setContextMenu(null)}
        />
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
