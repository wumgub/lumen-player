import React, { useEffect, useState } from 'react'
import { PlayerProvider, usePlayer } from './context/PlayerContext'
import { ThemeProvider, useTheme } from './context/ThemeContext'
import { Sidebar } from './components/Sidebar'
import { LibraryView } from './components/LibraryView'
import { NowPlaying } from './components/NowPlaying'
import { TransportBar } from './components/TransportBar'
import { ThemeSelector } from './components/ThemeSelector'
import { SettingsModal } from './components/SettingsModal'
import { VisualizerHost } from './components/VisualizerHost'
import { Collapse, Gear, Pause, Play, Next, Prev, Search } from './components/icons'
import { VIZ_TABS, isVizSelection, type VizSelection } from './visualizers'
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
  const [vizId, setVizId] = useState<VizSelection>(() => {
    const saved = localStorage.getItem('lumen.viz')
    return saved && isVizSelection(saved) ? saved : 'bars'
  })
  const [immersive, setImmersive] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [libraryVersion, setLibraryVersion] = useState(0)

  const player = usePlayer()
  const { current } = player
  const { setArtwork } = useTheme()

  const chooseViz = (v: VizSelection): void => {
    setVizId(v)
    localStorage.setItem('lumen.viz', v)
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
      else if (e.key === 'Escape') setImmersive(false)
      else if (e.key === 'f' && !e.metaKey && !e.ctrlKey) setImmersive((v) => !v)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [player])

  return (
    <div className="app">
      <div className="titlebar">
        <div className="titlebar-drag" />
        <div className="search-box">
          <Search size={16} />
          <input
            placeholder="Search songs, artists, albums…"
            value={search}
            onChange={(e) => {
              setSearch(e.target.value)
              if (e.target.value && view.kind !== 'songs') setView({ kind: 'songs' })
            }}
          />
        </div>
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
          <LibraryView
            view={view}
            setView={setView}
            search={search}
            libraryVersion={libraryVersion}
            onLibraryChanged={() => setLibraryVersion((v) => v + 1)}
          />
        </main>
        <NowPlaying vizId={vizId} setVizId={chooseViz} onImmersive={() => setImmersive(true)} />
      </div>

      <TransportBar />

      {immersive && (
        <ImmersiveMode vizId={vizId} setVizId={chooseViz} onExit={() => setImmersive(false)} />
      )}

      {showSettings && <SettingsModal onClose={() => setShowSettings(false)} />}
    </div>
  )
}

function ImmersiveMode({
  vizId,
  setVizId,
  onExit
}: {
  vizId: VizSelection
  setVizId: (v: VizSelection) => void
  onExit: () => void
}): React.ReactElement {
  const p = usePlayer()
  return (
    <div className="immersive">
      {vizId === 'off' ? (
        <div className="immersive-canvas viz-off">Visualizer off</div>
      ) : (
        <VisualizerHost vizId={vizId} className="immersive-canvas" />
      )}
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
        <div className="immersive-viz-tabs">
          {VIZ_TABS.map((v) => (
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
    </div>
  )
}
