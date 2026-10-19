import React from 'react'
import { usePlayer } from '../context/PlayerContext'
import { VISUALIZER_LIST, type VisualizerId } from '../visualizers'
import { VisualizerHost } from './VisualizerHost'
import { Artwork } from './Artwork'
import { Expand } from './icons'

export function NowPlaying({
  vizId,
  setVizId,
  onImmersive
}: {
  vizId: VisualizerId
  setVizId: (v: VisualizerId) => void
  onImmersive: () => void
}): React.ReactElement {
  const { current } = usePlayer()

  return (
    <aside className="now-playing">
      <div className="viz-panel">
        <VisualizerHost vizId={vizId} className="viz-canvas" />
        <button className="viz-expand" onClick={onImmersive} title="Immersive mode">
          <Expand size={18} />
        </button>
        <div className="viz-tabs">
          {VISUALIZER_LIST.map((v) => (
            <button
              key={v.id}
              className={`viz-tab ${v.id === vizId ? 'active' : ''}`}
              onClick={() => setVizId(v.id)}
            >
              {v.label}
            </button>
          ))}
        </div>
      </div>

      <div className="np-detail">
        {current ? (
          <>
            <Artwork
              trackId={current.id}
              hasArt={!!current.hasArt}
              className="np-art"
              size={64}
            />
            <div className="np-title">{current.title}</div>
            <div className="np-artist">{current.artist}</div>
            {current.album && <div className="np-album">{current.album}</div>}
          </>
        ) : (
          <div className="np-empty">No track selected</div>
        )}
      </div>
    </aside>
  )
}
