import React, { useEffect, useRef, useState } from 'react'
import { PlayerProvider, usePlayer } from './context/PlayerContext'
import { ThemeProvider, useTheme } from './context/ThemeContext'
import { Sidebar } from './components/Sidebar'
import { LibraryView } from './components/LibraryView'
import { StatsView } from './components/StatsView'
import { NowPlaying } from './components/NowPlaying'
import { TransportBar } from './components/TransportBar'
import { ThemeSelector } from './components/ThemeSelector'
import { SettingsModal } from './components/SettingsModal'
import { VisualizerHost } from './components/VisualizerHost'
import { VizToggle } from './components/VizToggle'
import { VideoSurface } from './components/VideoSurface'
import { Collapse, Gear, Pause, Play, Next, Prev } from './components/icons'
import { SearchBox } from './components/SearchBox'
import { formatTime } from './lib/format'
import { VISUALIZER_LIST, isVisualizerId, type VisualizerId } from './visualizers'
import type { View } from './types'

export default function App(): React.ReactElement {
  return (
    <ThemeProvider>
      <PlayerProvider>
        <Shell />
      </PlayerProvider>
    </ThemeProvider>
  )
}

function Shell(): React.ReactElement {
  const [view, setView] = useState<View>({ kind: 'songs' })
  const [search, setSearch] = useState('')
  const [vizId, setVizId] = useState<VisualizerId>(() => {
    const saved = localStorage.getItem('lumen.viz')
    return saved && isVisualizerId(saved) ? saved : 'bars'
  })
  const [vizOn, setVizOn] = useState<boolean>(() => localStorage.getItem('lumen.viz-off') !== '1')
  const [vizFps, setVizFps] = useState<number>(() => {
    const n = Number(localStorage.getItem('lumen.viz-fps'))
    return n === 30 || n === 15 ? n : 0 // 0 = uncapped (native refresh rate)
  })
  const [immersive, setImmersive] = useState(false)
  const [videoMode, setVideoMode] = useState<boolean>(
    () => localStorage.getItem('lumen.video-mode') !== '0'
  )
  const [videoFull, setVideoFull] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [libraryVersion, setLibraryVersion] = useState(0)

  const player = usePlayer()
  const { current } = player
  const { setArtwork } = useTheme()

  // Expose the host OS so CSS can reserve traffic-light space only on macOS.
  useEffect(() => {
    document.documentElement.dataset.os = window.lumen.platform
  }, [])

  const chooseVideoMode = (on: boolean): void => {
    setVideoMode(on)
    localStorage.setItem('lumen.video-mode', on ? '1' : '0')
  }
  // A music video is on screen in the now-playing panel (rather than its art).
  const showingVideo = !!current?.isVideo && videoMode

  // Leave the window-filling video if the next track isn't a video.
  useEffect(() => {
    if (!showingVideo) setVideoFull(false)
  }, [showingVideo])

  const chooseViz = (v: VisualizerId): void => {
    setVizId(v)
    localStorage.setItem('lumen.viz', v)
  }
  const toggleViz = (on: boolean): void => {
    setVizOn(on)
    localStorage.setItem('lumen.viz-off', on ? '0' : '1')
  }
  const chooseVizFps = (n: number): void => {
    setVizFps(n)
    localStorage.setItem('lumen.viz-fps', String(n))
  }

  // Feed artwork to the theme engine for adaptive tinting.
  useEffect(() => {
    setArtwork(current && current.hasArt ? window.lumen.artUrl(current.id) : null)
  }, [current, setArtwork])

  // Mirror playback to Discord presence. Only fires on track / play-state /
  // duration changes (not the per-second tick) to stay under Discord's rate limit;
  // the progress bar animates itself from the start/end timestamps.
  useEffect(() => {
    if (!current) {
      void window.lumen.setPresence(null)
      return
    }
    void window.lumen.setPresence({
      title: current.title,
      artist: current.artist,
      album: current.album,
      isPlaying: player.isPlaying,
      position: player.engine.currentTime,
      duration: player.engine.duration
    })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [current?.id, player.isPlaying, player.duration])

  // Keyboard shortcuts.
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const tag = (e.target as HTMLElement)?.tagName
      if (tag === 'INPUT' || tag === 'TEXTAREA') return
      if (e.code === 'Space') {
        e.preventDefault()
        player.togglePlay()
      } else if (e.code === 'ArrowRight' && e.metaKey) player.next()
      else if (e.code === 'ArrowLeft' && e.metaKey) player.prev()
      else if (e.key === 'Escape') {
        setImmersive(false)
        setVideoFull(false)
      } else if (e.key === 'f' && !e.metaKey && !e.ctrlKey) {
        // F fills the window with the music video when one is showing,
        // otherwise it toggles the immersive visualizer.
        if (showingVideo) setVideoFull((v) => !v)
        else setImmersive((v) => !v)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [player, showingVideo])

  return (
    <div className="app">
      <div className="titlebar">
        <div className="titlebar-drag" />
        <div className="titlebar-brand">
          <span className="brand-dot" />
          Lumen
        </div>
        <SearchBox
          value={search}
          onChange={(v) => {
            setSearch(v)
            if (v && view.kind !== 'songs') setView({ kind: 'songs' })
          }}
        />
        <ThemeSelector />
        <button
          className="titlebar-btn"
          onClick={() => setShowSettings(true)}
          title="Settings"
        >
          <Gear size={18} />
        </button>
      </div>

      <div className="body">
        <Sidebar
          view={view}
          setView={setView}
          onLibraryChanged={() => setLibraryVersion((v) => v + 1)}
          libraryVersion={libraryVersion}
        />
        <main className="content">
          {view.kind === 'stats' ? (
            <StatsView />
          ) : (
            <LibraryView
              view={view}
              setView={setView}
              search={search}
              libraryVersion={libraryVersion}
              onLibraryChanged={() => setLibraryVersion((v) => v + 1)}
            />
          )}
        </main>
        <NowPlaying
          vizId={vizId}
          setVizId={chooseViz}
          vizOn={vizOn}
          setVizOn={toggleViz}
          vizFps={vizFps}
          onImmersive={() => setImmersive(true)}
          videoMode={videoMode}
          chooseVideoMode={chooseVideoMode}
          videoFull={videoFull}
          onVideoFull={() => setVideoFull(true)}
        />
      </div>

      <TransportBar />

      {immersive && (
        <ImmersiveMode
          vizId={vizId}
          setVizId={chooseViz}
          vizOn={vizOn}
          setVizOn={toggleViz}
          vizFps={vizFps}
          onExit={() => setImmersive(false)}
        />
      )}

      {videoFull && <VideoFullscreen onExit={() => setVideoFull(false)} />}

      {showSettings && (
        <SettingsModal
          vizFps={vizFps}
          setVizFps={chooseVizFps}
          onClose={() => setShowSettings(false)}
        />
      )}
    </div>
  )
}

function ImmersiveMode({
  vizId,
  setVizId,
  vizOn,
  setVizOn,
  vizFps,
  onExit
}: {
  vizId: VisualizerId
  setVizId: (v: VisualizerId) => void
  vizOn: boolean
  setVizOn: (on: boolean) => void
  vizFps: number
  onExit: () => void
}): React.ReactElement {
  const p = usePlayer()
  return (
    <div className="immersive">
      {vizOn ? (
        <VisualizerHost vizId={vizId} fps={vizFps} className="immersive-canvas" />
      ) : (
        <div className="immersive-canvas viz-off">Visualizer off</div>
      )}
      <VizToggle on={vizOn} onChange={setVizOn} className="immersive-toggle" />
      <button className="immersive-exit" onClick={onExit} title="Exit (Esc)">
        <Collapse size={20} />
      </button>
      <div className="immersive-overlay">
        <div className="immersive-meta">
          <div className="im-title">{p.current?.title ?? 'Nothing playing'}</div>
          <div className="im-artist">{p.current?.artist ?? ''}</div>
        </div>
        <div className="immersive-controls">
          <button className="icon-btn" onClick={p.prev}>
            <Prev size={26} />
          </button>
          <button className="play-btn" onClick={p.togglePlay}>
            {p.isPlaying ? <Pause size={28} /> : <Play size={28} />}
          </button>
          <button className="icon-btn" onClick={p.next}>
            <Next size={26} />
          </button>
        </div>
        {vizOn && (
          <div className="immersive-viz-tabs">
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
    </div>
  )
}

/**
 * The music video filling the app window (not OS fullscreen — the window stays
 * as it is). Reuses the engine's single <video> element, re-parented here, so
 * playback carries on uninterrupted. Controls fade out while the mouse is idle.
 */
function VideoFullscreen({ onExit }: { onExit: () => void }): React.ReactElement {
  const p = usePlayer()
  const [idle, setIdle] = useState(false)
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)

  const wake = (): void => {
    setIdle(false)
    if (timer.current) clearTimeout(timer.current)
    timer.current = setTimeout(() => setIdle(true), 2500)
  }
  useEffect(() => {
    wake()
    return () => {
      if (timer.current) clearTimeout(timer.current)
    }
  }, [])

  const pct = p.duration > 0 ? (p.currentTime / p.duration) * 100 : 0

  return (
    <div className={`video-full ${idle ? 'idle' : ''}`} onMouseMove={wake}>
      <div className="video-full-stage" onDoubleClick={onExit}>
        <VideoSurface className="video-full-surface" />
      </div>
      <button className="immersive-exit video-full-exit" onClick={onExit} title="Exit (Esc)">
        <Collapse size={20} />
      </button>
      <div className="immersive-overlay video-full-overlay">
        <div className="immersive-meta">
          <div className="im-title">{p.current?.title ?? ''}</div>
          <div className="im-artist">{p.current?.artist ?? ''}</div>
        </div>
        <div className="immersive-controls">
          <button className="icon-btn" onClick={p.prev}>
            <Prev size={26} />
          </button>
          <button className="play-btn" onClick={p.togglePlay}>
            {p.isPlaying ? <Pause size={28} /> : <Play size={28} />}
          </button>
          <button className="icon-btn" onClick={p.next}>
            <Next size={26} />
          </button>
        </div>
        <div className="seek-row video-full-seek">
          <span className="time">{formatTime(p.currentTime)}</span>
          <input
            className="seek"
            type="range"
            min={0}
            max={p.duration || 0}
            step="0.1"
            value={p.currentTime}
            style={{ ['--pct' as string]: `${pct}%` }}
            onChange={(e) => p.seek(Number(e.target.value))}
          />
          <span className="time">{formatTime(p.duration)}</span>
        </div>
      </div>
    </div>
  )
}
