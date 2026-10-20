import React from 'react'
import { createPortal } from 'react-dom'
import { usePlayer } from '../context/PlayerContext'
import { Artwork } from './Artwork'
import { formatTime, pluralize } from '../lib/format'

/** Slide-over list of the play queue: jump to any track, remove, or clear what's next. */
export function QueuePanel({ onClose }: { onClose: () => void }): React.ReactElement {
  const p = usePlayer()
  const upcoming = p.index >= 0 ? p.queue.length - (p.index + 1) : p.queue.length
  const hasUpcoming = p.index >= 0 && p.index < p.queue.length - 1

  return createPortal(
    <div className="queue-backdrop" onMouseDown={onClose}>
      <aside className="queue-panel" onMouseDown={(e) => e.stopPropagation()}>
        <div className="queue-head">
          <div>
            <div className="queue-title">Queue</div>
            <div className="queue-sub">
              {p.queue.length === 0
                ? 'Empty'
                : `${pluralize(upcoming, 'song')} up next`}
            </div>
          </div>
          {hasUpcoming && (
            <button className="queue-clear" onClick={p.clearUpcoming}>
              Clear upcoming
            </button>
          )}
        </div>

        {p.queue.length === 0 ? (
          <div className="queue-empty">
            Nothing queued. Use “Play next” or “Add to queue” on any track.
          </div>
        ) : (
          <div className="queue-list">
            {p.queue.map((t, i) => (
              <div
                key={`${t.id}-${i}`}
                className={`queue-row ${i === p.index ? 'current' : ''} ${
                  i < p.index ? 'played' : ''
                }`}
                onClick={() => p.jumpTo(i)}
                title="Play"
              >
                <Artwork trackId={t.id} hasArt={!!t.hasArt} className="queue-art" size={16} />
                <div className="queue-main">
                  <div className="queue-row-title">{t.title}</div>
                  <div className="queue-row-sub">{t.artist}</div>
                </div>
                <span className="queue-time">{formatTime(t.duration)}</span>
                <button
                  className="queue-remove"
                  title="Remove from queue"
                  onClick={(e) => {
                    e.stopPropagation()
                    p.removeFromQueue(i)
                  }}
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        )}
      </aside>
    </div>,
    document.body
  )
}
