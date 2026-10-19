import React, { useEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { Disc, Plus } from './icons'

export interface AlbumPickRequest {
  trackIds: number[]
  onDone: () => void
}

/** Assign one or more tracks to an album — pick an existing one or create a new name. */
export function AlbumPickerDialog({
  request,
  onClose
}: {
  request: AlbumPickRequest | null
  onClose: () => void
}): React.ReactElement | null {
  const [albums, setAlbums] = useState<{ album: string; artist: string }[]>([])
  const [text, setText] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (!request) return
    setText('')
    void window.lumen.albums().then(setAlbums)
    requestAnimationFrame(() => inputRef.current?.focus())
  }, [request])

  const query = text.trim().toLowerCase()
  const matches = useMemo(
    () => albums.filter((a) => a.album.toLowerCase().includes(query)).slice(0, 50),
    [albums, query]
  )
  const exactExists = albums.some((a) => a.album.toLowerCase() === query)

  if (!request) return null

  const assign = async (album: string): Promise<void> => {
    const name = album.trim()
    if (!name) return
    await window.lumen.setTrackAlbum(request.trackIds, name)
    request.onDone()
    onClose()
  }

  const n = request.trackIds.length

  return createPortal(
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal album-picker" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Add {n === 1 ? 'song' : `${n} songs`} to album</div>
        <div className="modal-label">Search for an album or type a new name</div>
        <input
          ref={inputRef}
          className="modal-input"
          placeholder="Album name…"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && text.trim()) void assign(text)
            else if (e.key === 'Escape') onClose()
          }}
        />
        <div className="album-options">
          {query && !exactExists && (
            <button className="album-option create" onClick={() => void assign(text)}>
              <Plus size={16} />
              <span>
                Create album “<strong>{text.trim()}</strong>”
              </span>
            </button>
          )}
          {matches.map((a) => (
            <button key={a.album} className="album-option" onClick={() => void assign(a.album)}>
              <Disc size={16} />
              <span className="album-option-main">
                <span className="album-option-name">{a.album}</span>
                {a.artist && <span className="album-option-sub">{a.artist}</span>}
              </span>
            </button>
          ))}
          {matches.length === 0 && !query && (
            <div className="row-menu-empty">Start typing to find or create an album</div>
          )}
        </div>
        <div className="modal-actions">
          <button className="modal-btn ghost" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>,
    document.body
  )
}
