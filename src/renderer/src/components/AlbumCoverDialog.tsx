import React, { useEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import type { Track } from '@shared/types'
import { Artwork } from './Artwork'
import { Disc, ImageIcon } from './icons'

/**
 * Change an album's cover: supply a custom image, borrow another song's embedded
 * art, or reset to the default (the first track's art).
 */
export function AlbumCoverDialog({
  album,
  onChanged,
  onClose
}: {
  album: string
  onChanged: () => void
  onClose: () => void
}): React.ReactElement {
  const [mode, setMode] = useState<'choices' | 'song'>('choices')
  const [songs, setSongs] = useState<Track[]>([])
  const [q, setQ] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  // Load the library's art-bearing songs when switching to the picker.
  useEffect(() => {
    if (mode !== 'song') return
    void window.lumen
      .query({ sort: 'artist', dir: 'asc' })
      .then((ts) => setSongs(ts.filter((t) => t.hasArt)))
    requestAnimationFrame(() => inputRef.current?.focus())
  }, [mode])

  const query = q.trim().toLowerCase()
  const matches = useMemo(
    () =>
      songs
        .filter((t) => `${t.title} ${t.artist} ${t.album}`.toLowerCase().includes(query))
        .slice(0, 80),
    [songs, query]
  )

  const customImage = async (): Promise<void> => {
    const ok = await window.lumen.setAlbumCoverCustom(album)
    if (ok) onChanged()
    onClose()
  }
  const reset = async (): Promise<void> => {
    await window.lumen.clearAlbumCover(album)
    onChanged()
    onClose()
  }
  const pick = async (trackId: number): Promise<void> => {
    await window.lumen.setAlbumCoverFromTrack(album, trackId)
    onChanged()
    onClose()
  }

  return createPortal(
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal cover-dialog" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Cover for “{album}”</div>

        {mode === 'choices' ? (
          <>
            <div className="modal-label">Choose where the album cover comes from</div>
            <div className="cover-choices">
              <button className="cover-choice" onClick={() => void customImage()}>
                <ImageIcon size={18} />
                <span>
                  <strong>Custom image…</strong>
                  <small>Pick an image from your files</small>
                </span>
              </button>
              <button className="cover-choice" onClick={() => setMode('song')}>
                <Disc size={18} />
                <span>
                  <strong>Choose from a song…</strong>
                  <small>Use the cover of another song in your library</small>
                </span>
              </button>
              <button className="cover-choice" onClick={() => void reset()}>
                <span>
                  <strong>Reset to default</strong>
                  <small>Use the first song on the album</small>
                </span>
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="modal-label">Pick a song whose cover to use</div>
            <input
              ref={inputRef}
              className="modal-input"
              placeholder="Search songs…"
              value={q}
              onChange={(e) => setQ(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Escape') onClose()
              }}
            />
            <div className="cover-song-list">
              {matches.map((t) => (
                <button key={t.id} className="cover-song" onClick={() => void pick(t.id)}>
                  <Artwork trackId={t.id} hasArt className="cover-song-art" size={14} />
                  <span className="cover-song-main">
                    <span className="cover-song-title">{t.title}</span>
                    <span className="cover-song-sub">
                      {t.artist}
                      {t.album ? ` — ${t.album}` : ''}
                    </span>
                  </span>
                </button>
              ))}
              {matches.length === 0 && (
                <div className="row-menu-empty">
                  {songs.length ? 'No matches' : 'No songs with embedded art yet'}
                </div>
              )}
            </div>
          </>
        )}

        <div className="modal-actions">
          {mode === 'song' && (
            <button className="modal-btn ghost" onClick={() => setMode('choices')}>
              Back
            </button>
          )}
          <button className="modal-btn ghost" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>,
    document.body
  )
}
