import React, { useCallback, useEffect, useState } from 'react'
import type { PlaylistWithCount, ScanProgress } from '@shared/types'
import type { View } from '../types'
import { pluralize } from '../lib/format'
import { Disc, Folder, ListIcon, MusicNote, Plus, Users } from './icons'
import { PromptDialog } from './PromptDialog'

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
  const [newPlaylistOpen, setNewPlaylistOpen] = useState(false)

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

  const createPlaylist = async (name: string): Promise<void> => {
    await window.lumen.createPlaylist(name)
    await refresh()
  }

  const deletePlaylist = async (id: number): Promise<void> => {
    if (!confirm('Delete this playlist?')) return
    await window.lumen.deletePlaylist(id)
    if (view.kind === 'playlist' && view.id === id) setView({ kind: 'songs' })
    await refresh()
  }

  const navActive = (kind: View['kind']): string => (view.kind === kind ? 'active' : '')

  const pct =
    progress && progress.total > 0 ? Math.round((progress.processed / progress.total) * 100) : 0

  return (
    <nav className="sidebar">
      <div className="brand">
        <span className="brand-dot" />
        Lumen
      </div>

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
          <button className="mini-btn" onClick={() => setNewPlaylistOpen(true)} title="New playlist">
            <Plus size={14} />
          </button>
        </div>
        <div className="playlist-list">
          {playlists.map((pl) => (
            <button
              key={pl.id}
              className={`nav-item ${
                view.kind === 'playlist' && view.id === pl.id ? 'active' : ''
              }`}
              onClick={() => setView({ kind: 'playlist', id: pl.id, name: pl.name })}
              onContextMenu={(e) => {
                e.preventDefault()
                void deletePlaylist(pl.id)
              }}
              title="Right-click to delete"
            >
              <ListIcon size={16} />
              <span className="pl-name">{pl.name}</span>
              <span className="pl-count">{pl.count}</span>
            </button>
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
            <div key={f} className="folder-row" title={f}>
              <Folder size={14} />
              <span className="folder-path">{f.split('/').pop() || f}</span>
              <button className="mini-btn ghost" onClick={() => removeFolder(f)}>
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
        open={newPlaylistOpen}
        title="New playlist"
        label="Give your playlist a name"
        placeholder="e.g. Late night coding"
        submitLabel="Create"
        onSubmit={createPlaylist}
        onClose={() => setNewPlaylistOpen(false)}
      />
    </nav>
  )
}
