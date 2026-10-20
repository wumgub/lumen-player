import React, { useCallback, useEffect, useMemo, useState } from 'react'
import type { PlaylistWithCount, ScanProgress } from '@shared/types'
import type { View } from '../types'
import { pluralize } from '../lib/format'
import { randomPlaylistName } from '../lib/playlistNames'
import { Disc, Folder, ListIcon, MusicNote, Pencil, Plus, Users } from './icons'
import { PromptDialog } from './PromptDialog'

type PlaylistDialog = { mode: 'new' } | { mode: 'rename'; id: number; name: string } | null

export function Sidebar({
  view,
  setView,
  onLibraryChanged,
  libraryVersion
}: {
  view: View
  setView: (v: View) => void
  onLibraryChanged: () => void
  libraryVersion: number
}): React.ReactElement {
  const [folders, setFolders] = useState<string[]>([])
  const [playlists, setPlaylists] = useState<PlaylistWithCount[]>([])
  const [stats, setStats] = useState({ tracks: 0, artists: 0, albums: 0 })
  const [scanning, setScanning] = useState(false)
  const [progress, setProgress] = useState<ScanProgress | null>(null)
  const [playlistDialog, setPlaylistDialog] = useState<PlaylistDialog>(null)
  const [dragPl, setDragPl] = useState<number | null>(null)
  const [overPl, setOverPl] = useState<number | null>(null)
  // Re-roll a random example name each time the "New playlist" dialog opens
  // (playlistDialog is a fresh object per open, so this recomputes then, but
  // stays stable while the dialog is open).
  const playlistPlaceholder = useMemo(() => `e.g. ${randomPlaylistName()}`, [playlistDialog])

  const refresh = useCallback(async () => {
    const [f, p, s] = await Promise.all([
      window.lumen.folders(),
      window.lumen.playlists(),
      window.lumen.stats()
    ])
    setFolders(f)
    setPlaylists(p)
    setStats(s)
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh, libraryVersion])

  useEffect(() => window.lumen.onScanProgress(setProgress), [])

  const addFolders = async (): Promise<void> => {
    const picked = await window.lumen.pickFolder()
    for (const p of picked) await window.lumen.addFolder(p)
    if (picked.length) {
      await refresh()
      await scan()
    }
  }

  const scan = async (): Promise<void> => {
    setScanning(true)
    setProgress({ phase: 'discovering', processed: 0, total: 0 })
    try {
      await window.lumen.scan()
      onLibraryChanged()
      await refresh()
    } finally {
      setScanning(false)
      setProgress(null)
    }
  }

  const removeFolder = async (p: string): Promise<void> => {
    await window.lumen.removeFolder(p)
    await refresh()
  }

  const submitPlaylistDialog = async (name: string): Promise<void> => {
    if (playlistDialog?.mode === 'rename') {
      await window.lumen.renamePlaylist(playlistDialog.id, name)
      if (view.kind === 'playlist' && view.id === playlistDialog.id)
        setView({ kind: 'playlist', id: playlistDialog.id, name })
    } else {
      await window.lumen.createPlaylist(name)
    }
    await refresh()
  }

  const deletePlaylist = async (id: number): Promise<void> => {
    if (!confirm('Delete this playlist?')) return
    await window.lumen.deletePlaylist(id)
    if (view.kind === 'playlist' && view.id === id) setView({ kind: 'songs' })
    await refresh()
  }

  const dropPlaylist = async (targetId: number): Promise<void> => {
    const from = playlists.findIndex((p) => p.id === dragPl)
    const to = playlists.findIndex((p) => p.id === targetId)
    setDragPl(null)
    setOverPl(null)
    if (from < 0 || to < 0 || from === to) return
    const next = playlists.slice()
    const [moved] = next.splice(from, 1)
    next.splice(to, 0, moved)
    setPlaylists(next) // optimistic
    await window.lumen.reorderPlaylists(next.map((p) => p.id))
  }

  const navActive = (kind: View['kind']): string => (view.kind === kind ? 'active' : '')

  const pct =
    progress && progress.total > 0 ? Math.round((progress.processed / progress.total) * 100) : 0

  return (
    <nav className="sidebar">
      <div className="nav-section">
        <button className={`nav-item ${navActive('songs')}`} onClick={() => setView({ kind: 'songs' })}>
          <MusicNote size={18} /> Songs
        </button>
        <button
          className={`nav-item ${navActive('artists')}`}
          onClick={() => setView({ kind: 'artists' })}
        >
          <Users size={18} /> Artists
        </button>
        <button
          className={`nav-item ${navActive('albums')}`}
          onClick={() => setView({ kind: 'albums' })}
        >
          <Disc size={18} /> Albums
        </button>
      </div>

      <div className="nav-section">
        <div className="nav-head">
          <span>Playlists</span>
          <button
            className="mini-btn"
            onClick={() => setPlaylistDialog({ mode: 'new' })}
            title="New playlist"
          >
            <Plus size={14} />
          </button>
        </div>
        <div className="playlist-list">
          {playlists.map((pl) => (
            <div
              key={pl.id}
              className={`nav-item playlist-item ${
                view.kind === 'playlist' && view.id === pl.id ? 'active' : ''
              } ${overPl === pl.id && dragPl !== null && dragPl !== pl.id ? 'drop-target' : ''} ${
                dragPl === pl.id ? 'dragging' : ''
              }`}
              role="button"
              tabIndex={0}
              draggable
              onDragStart={(e) => {
                setDragPl(pl.id)
                e.dataTransfer.effectAllowed = 'move'
              }}
              onDragOver={(e) => {
                if (dragPl !== null) {
                  e.preventDefault()
                  setOverPl(pl.id)
                }
              }}
              onDrop={() => void dropPlaylist(pl.id)}
              onDragEnd={() => {
                setDragPl(null)
                setOverPl(null)
              }}
              onClick={() => setView({ kind: 'playlist', id: pl.id, name: pl.name })}
            >
              <ListIcon size={16} />
              <span className="pl-name">{pl.name}</span>
              <span className="pl-count">{pl.count}</span>
              <div className="pl-actions">
                <button
                  className="mini-btn ghost"
                  title="Rename playlist"
                  onClick={(e) => {
                    e.stopPropagation()
                    setPlaylistDialog({ mode: 'rename', id: pl.id, name: pl.name })
                  }}
                >
                  <Pencil size={13} />
                </button>
                <button
                  className="mini-btn ghost"
                  title="Delete playlist"
                  onClick={(e) => {
                    e.stopPropagation()
                    void deletePlaylist(pl.id)
                  }}
                >
                  ×
                </button>
              </div>
            </div>
          ))}
          {playlists.length === 0 && <div className="empty-hint">No playlists yet</div>}
        </div>
      </div>

      <div className="nav-section folders">
        <div className="nav-head">
          <span>Folders</span>
          <button className="mini-btn" onClick={addFolders} title="Add folder">
            <Plus size={14} />
          </button>
        </div>
        <div className="folder-list">
          {folders.map((f) => (
            <div
              key={f}
              className={`folder-row ${view.kind === 'folder' && view.path === f ? 'active' : ''}`}
              title={f}
              role="button"
              tabIndex={0}
              onClick={() => setView({ kind: 'folder', path: f })}
            >
              <Folder size={14} />
              <span className="folder-path">{f.split('/').pop() || f}</span>
              <button
                className="mini-btn ghost"
                title="Remove folder"
                onClick={(e) => {
                  e.stopPropagation()
                  void removeFolder(f)
                }}
              >
                ×
              </button>
            </div>
          ))}
          {folders.length === 0 && <div className="empty-hint">Add a music folder to begin</div>}
        </div>
      </div>

      <div className="sidebar-footer">
        {scanning ? (
          <div className="scan-status">
            <div className="scan-label">
              {progress?.phase === 'discovering'
                ? 'Finding files…'
                : progress?.phase === 'done'
                  ? 'Done'
                  : `Scanning ${progress?.processed ?? 0}/${progress?.total ?? 0}`}
            </div>
            <div className="scan-bar">
              <div className="scan-fill" style={{ width: `${pct}%` }} />
            </div>
          </div>
        ) : (
          <>
            <button
              className="scan-btn"
              onClick={scan}
              disabled={folders.length === 0}
              title={folders.length === 0 ? 'Add a folder first' : 'Rescan library'}
            >
              Rescan library
            </button>
            <div className="lib-stats">
              {pluralize(stats.tracks, 'song')} · {pluralize(stats.artists, 'artist')}
            </div>
          </>
        )}
      </div>

      <PromptDialog
        open={!!playlistDialog}
        title={playlistDialog?.mode === 'rename' ? 'Rename playlist' : 'New playlist'}
        label={
          playlistDialog?.mode === 'rename'
            ? 'Choose a new name'
            : 'Give your playlist a name'
        }
        placeholder={playlistPlaceholder}
        initial={playlistDialog?.mode === 'rename' ? playlistDialog.name : ''}
        submitLabel={playlistDialog?.mode === 'rename' ? 'Rename' : 'Create'}
        onSubmit={submitPlaylistDialog}
        onClose={() => setPlaylistDialog(null)}
      />
    </nav>
  )
}
