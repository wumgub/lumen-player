import React, { useEffect, useState } from 'react'
import { Disc, ImageIcon } from './icons'
import { AlbumCoverDialog } from './AlbumCoverDialog'

/** Deterministic gradient fallback for an album with no art at all. */
function gradientFor(seed: string): React.CSSProperties {
  let h = 0
  for (let i = 0; i < seed.length; i++) h = (h * 31 + seed.charCodeAt(i)) % 360
  const h2 = (h + 50) % 360
  return { background: `linear-gradient(135deg, hsl(${h} 60% 45%), hsl(${h2} 65% 35%))` }
}

/**
 * An album's cover: its first track's embedded art (or a user-set override),
 * falling back to a gradient + disc glyph when the album has no art. When
 * `editable`, shows a button to change the cover (custom image / another song's
 * art / reset).
 */
export function AlbumCover({
  album,
  version,
  onChanged,
  className,
  iconSize = 28,
  editable = false
}: {
  album: string
  version: number
  onChanged?: () => void
  className?: string
  iconSize?: number
  editable?: boolean
}): React.ReactElement {
  const [err, setErr] = useState(false)
  const [dialog, setDialog] = useState(false)

  // A cover may have appeared/changed since the last error — retry on version bump.
  useEffect(() => setErr(false), [album, version])

  // Keying the <img> on `version` remounts it when a cover changes, forcing a
  // reload across both backends: the desktop sends the album art uncached, and
  // the mobile shim returns a fresh blob: URL (which a ?v= query would break).
  const src = window.lumen.albumArtUrl(album)
  const showImg = !!src && !err

  return (
    <div className={`album-cover ${className ?? ''}`}>
      {showImg ? (
        <img key={version} src={src} alt="" onError={() => setErr(true)} />
      ) : (
        <div className="album-cover-fallback" style={gradientFor(album)}>
          <Disc size={iconSize} />
        </div>
      )}
      {editable && (
        <button
          className="album-cover-edit"
          title="Change cover"
          onClick={(e) => {
            e.stopPropagation()
            setDialog(true)
          }}
        >
          <ImageIcon size={14} />
        </button>
      )}
      {dialog && (
        <AlbumCoverDialog
          album={album}
          onChanged={() => onChanged?.()}
          onClose={() => setDialog(false)}
        />
      )}
    </div>
  )
}
