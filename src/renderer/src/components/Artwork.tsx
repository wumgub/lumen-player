import React from 'react'
import { MusicNote } from './icons'

/** Album art by track id, falling back to a note glyph when none is embedded. */
export function Artwork({
  trackId,
  hasArt,
  className,
  size = 20
}: {
  trackId: number
  hasArt?: boolean
  className?: string
  size?: number
}): React.ReactElement {
  const [err, setErr] = React.useState(false)
  const show = hasArt !== false && !err
  return (
    <div className={`artwork ${className ?? ''}`}>
      {show ? (
        <img src={window.lumen.artUrl(trackId)} alt="" onError={() => setErr(true)} />
      ) : (
        <div className="artwork-fallback">
          <MusicNote size={size} />
        </div>
      )}
    </div>
  )
}
