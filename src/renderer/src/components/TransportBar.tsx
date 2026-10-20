import React, { useState } from 'react'
import { usePlayer } from '../context/PlayerContext'
import { formatTime } from '../lib/format'
import { Artwork } from './Artwork'
import { QueuePanel } from './QueuePanel'
import {
  ListIcon,
  Next,
  Pause,
  Play,
  Prev,
  Repeat,
  RepeatOne,
  Shuffle,
  VolHigh,
  VolMute
} from './icons'

export function TransportBar(): React.ReactElement {
  const p = usePlayer()
  const pct = p.duration > 0 ? (p.currentTime / p.duration) * 100 : 0
  const [showQueue, setShowQueue] = useState(false)
  const upNext = p.index >= 0 ? Math.max(0, p.queue.length - (p.index + 1)) : 0

  return (
    <div className="transport">
      <div className="transport-now">
        {p.current ? (
          <>
            <Artwork trackId={p.current.id} hasArt={!!p.current.hasArt} className="transport-art" />
            <div className="transport-meta">
              <div className="t-title" title={p.current.title}>
                {p.current.title}
              </div>
              <div className="t-sub" title={p.current.artist}>
                {p.current.artist}
                {p.current.album ? ` — ${p.current.album}` : ''}
              </div>
            </div>
          </>
        ) : (
          <div className="transport-meta">
            <div className="t-title">Nothing playing</div>
            <div className="t-sub">Pick a track to start</div>
          </div>
        )}
      </div>

      <div className="transport-center">
        <div className="transport-buttons">
          <button
            className={`icon-btn ${p.shuffle ? 'active' : ''}`}
            onClick={p.toggleShuffle}
            title="Shuffle"
          >
            <Shuffle size={18} />
          </button>
          <button className="icon-btn" onClick={p.prev} title="Previous">
            <Prev size={22} />
          </button>
          <button className="play-btn" onClick={p.togglePlay} title="Play/Pause">
            {p.isPlaying ? <Pause size={24} /> : <Play size={24} />}
          </button>
          <button className="icon-btn" onClick={p.next} title="Next">
            <Next size={22} />
          </button>
          <button
            className={`icon-btn ${p.repeat !== 'off' ? 'active' : ''}`}
            onClick={p.cycleRepeat}
            title={`Repeat: ${p.repeat}`}
          >
            {p.repeat === 'one' ? <RepeatOne size={18} /> : <Repeat size={18} />}
          </button>
        </div>
        <div className="seek-row">
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

      <div className="transport-right">
        <button
          className={`icon-btn queue-btn ${showQueue ? 'active' : ''}`}
          onClick={() => setShowQueue((v) => !v)}
          title="Queue"
        >
          <ListIcon size={18} />
          {upNext > 0 && <span className="queue-badge">{upNext > 99 ? '99+' : upNext}</span>}
        </button>
        <button
          className="icon-btn"
          onClick={() => p.setVolume(p.volume > 0 ? 0 : 0.9)}
          title="Mute"
        >
          {p.volume > 0 ? <VolHigh size={18} /> : <VolMute size={18} />}
        </button>
        <input
          className="volume"
          type="range"
          min={0}
          max={1}
          step="0.01"
          value={p.volume}
          style={{ ['--pct' as string]: `${p.volume * 100}%` }}
          onChange={(e) => p.setVolume(Number(e.target.value))}
        />
      </div>

      {showQueue && <QueuePanel onClose={() => setShowQueue(false)} />}
    </div>
  )
}
