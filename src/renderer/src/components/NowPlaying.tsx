import React from 'react'
import { usePlayer } from '../context/PlayerContext'
import { VISUALIZER_LIST, type VisualizerId } from '../visualizers'
import { VisualizerHost } from './VisualizerHost'
import { VizToggle } from './VizToggle'
import { Artwork } from './Artwork'
import { Expand } from './icons'

export function NowPlaying({
  vizId,
  setVizId,
  vizOn,
  setVizOn,
  vizFps,
  onImmersive
}: {
  vizId: VisualizerId
  setVizId: (v: VisualizerId) => void
  vizOn: boolean
  setVizOn: (on: boolean) => void
  vizFps: number
  onImmersive: () => void
}): React.ReactElement {
  const { current } = usePlayer()

  return (
    <aside className="now-playing">
      <div className="viz-panel">
        {vizOn ? (
          <VisualizerHost vizId={vizId} fps={vizFps} className="viz-canvas" />
        ) : (
          <div className="viz-canvas viz-off">Visualizer off</div>
        )}
        <VizToggle on={vizOn} onChange={setVizOn} />
        <button className="viz-expand" onClick={onImmersive} title="Immersive mode">
          <Expand size={18} />
        </button>
        {vizOn && (
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
        )}
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
