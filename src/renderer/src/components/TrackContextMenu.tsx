import React, { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import type { PlaylistWithCount, Track } from '@shared/types'

export interface TrackMenuActions {
  rename: () => void
  editNote: () => void
  showFile: () => void
  playNext: () => void
  addToQueue: () => void
  addToPlaylist: (playlistId: number) => void
  addToAlbum: () => void
  removeFromAlbum?: () => void
  removeFromPlaylist?: () => void
}

export interface ContextMenuState {
  x: number
  y: number
  track: Track
}

/** Right-click / ⋯ context menu for a track row. Rendered at the cursor via a portal. */
export function TrackContextMenu({
  menu,
  playlists,
  actions,
  onClose
}: {
  menu: ContextMenuState
  playlists: PlaylistWithCount[]
  actions: TrackMenuActions
  onClose: () => void
}): React.ReactElement {
  const ref = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState({ left: menu.x, top: menu.y })

  // Keep the menu on-screen.
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const { width, height } = el.getBoundingClientRect()
    const left = Math.min(menu.x, window.innerWidth - width - 8)
    const top = Math.min(menu.y, window.innerHeight - height - 8)
    setPos({ left: Math.max(8, left), top: Math.max(8, top) })
  }, [menu])

  useEffect(() => {
    const onDown = (e: MouseEvent): void => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose()
    }
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('mousedown', onDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [onClose])

  const run = (fn?: () => void): void => {
    fn?.()
    onClose()
  }

  return createPortal(
    <div
      ref={ref}
      className="context-menu"
      style={{ left: pos.left, top: pos.top }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <button className="ctx-item" onClick={() => run(actions.playNext)}>
        Play next
      </button>
      <button className="ctx-item" onClick={() => run(actions.addToQueue)}>
        Add to queue
      </button>
      <div className="ctx-sep" />
      <button className="ctx-item" onClick={() => run(actions.rename)}>
        Rename song…
      </button>
      <button className="ctx-item" onClick={() => run(actions.editNote)}>
        {menu.track.note ? 'Edit note…' : 'Add note…'}
      </button>
      <button className="ctx-item" onClick={() => run(actions.showFile)}>
        Show file
      </button>
      <button className="ctx-item" onClick={() => run(actions.addToAlbum)}>
        Add to album…
      </button>
      {actions.removeFromAlbum && (
        <button className="ctx-item" onClick={() => run(actions.removeFromAlbum)}>
          Remove from album
        </button>
      )}
      {actions.removeFromPlaylist && (
        <button className="ctx-item danger" onClick={() => run(actions.removeFromPlaylist)}>
          Remove from playlist
        </button>
      )}

      <div className="ctx-sep" />
      <div className="ctx-label">Add to playlist</div>
      {playlists.length === 0 ? (
        <div className="ctx-empty">No playlists yet</div>
      ) : (
        <div className="ctx-scroll">
          {playlists.map((pl) => (
            <button
              key={pl.id}
              className="ctx-item"
              onClick={() => run(() => actions.addToPlaylist(pl.id))}
            >
              {pl.name}
            </button>
          ))}
        </div>
      )}
    </div>,
    document.body
  )
}
