import React, { useState } from 'react'
import { usePlayer } from '../context/PlayerContext'
import { VISUALIZER_LIST, type VisualizerId } from '../visualizers'
import { VisualizerHost } from './VisualizerHost'
import { VizToggle } from './VizToggle'
import { VideoSurface } from './VideoSurface'
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
  const [videoMode, setVideoMode] = useState<boolean>(
    () => localStorage.getItem('lumen.video-mode') !== '0'
  )
  const chooseVideoMode = (on: boolean): void => {
    setVideoMode(on)
    localStorage.setItem('lumen.video-mode', on ? '1' : '0')
  }
  const isVideo = !!current?.isVideo

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
            {isVideo && videoMode ? (
              <VideoSurface className="np-media" />
            ) : (
              <Artwork
                trackId={current.id}
                hasArt={!!current.hasArt}
                className="np-art"
                size={64}
              />
            )}
            {isVideo && (
              <div className="np-media-toggle" role="group" aria-label="Show">
                <button
                  className={!videoMode ? 'active' : ''}
                  onClick={() => chooseVideoMode(false)}
                >
                  Art
                </button>
                <button
                  className={videoMode ? 'active' : ''}
                  onClick={() => chooseVideoMode(true)}
                >
                  Video
                </button>
              </div>
            )}
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
