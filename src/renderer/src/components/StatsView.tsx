import React, { useEffect, useMemo, useState } from 'react'
import type { PlayEvent, Track } from '@shared/types'
import { usePlayer } from '../context/PlayerContext'
import { Artwork } from './Artwork'
import { formatDuration, pluralize } from '../lib/format'

const WEEKS = 12

/** Monday 00:00 (local) of the week containing `ts`. */
function weekStart(ts: number): number {
  const d = new Date(ts)
  d.setHours(0, 0, 0, 0)
  const back = (d.getDay() + 6) % 7 // 0 = Monday
  d.setDate(d.getDate() - back)
  return d.getTime()
}

function weekRange(start: number): string {
  const a = new Date(start)
  const b = new Date(start)
  b.setDate(b.getDate() + 6)
  const fmt = (x: Date): string => x.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  return `${fmt(a)} – ${fmt(b)}`
}

function weekTick(start: number): string {
  return new Date(start).toLocaleDateString(undefined, { month: 'numeric', day: 'numeric' })
}

interface TopRow {
  track: Track
  plays: number
  seconds: number
}
interface WeekBucket {
  start: number
  seconds: number
  plays: number
}

/**
 * Listening leaderboard + weekly totals. Pulls the raw play log once and does
 * all aggregation here, so desktop and Android share exactly one stats path.
 */
export function StatsView(): React.ReactElement {
  const player = usePlayer()
  const [history, setHistory] = useState<PlayEvent[] | null>(null)
  const [tracks, setTracks] = useState<Map<number, Track>>(new Map())

  useEffect(() => {
    let alive = true
    void (async () => {
      const [h, all] = await Promise.all([
        window.lumen.playHistory(),
        window.lumen.query({ sort: 'title' })
      ])
      if (!alive) return
      setTracks(new Map(all.map((t) => [t.id, t])))
      setHistory(h)
    })()
    return () => {
      alive = false
    }
  }, [])

  const { top, weeks, totalSeconds, totalPlays, thisWeek } = useMemo(() => {
    const h = history ?? []
    const agg = new Map<number, { plays: number; seconds: number }>()
    let totalSeconds = 0
    let totalPlays = 0
    for (const e of h) {
      totalSeconds += e.seconds
      totalPlays += e.counted
      const a = agg.get(e.trackId) ?? { plays: 0, seconds: 0 }
      a.plays += e.counted
      a.seconds += e.seconds
      agg.set(e.trackId, a)
    }
    const top: TopRow[] = [...agg.entries()]
      .map(([id, a]) => ({ track: tracks.get(id), plays: a.plays, seconds: a.seconds }))
      .filter((r): r is TopRow => !!r.track && r.plays > 0)
      .sort((x, y) => y.plays - x.plays || y.seconds - x.seconds)
      .slice(0, 30)

    // Last WEEKS weeks, oldest → newest. Step by calendar weeks (not fixed ms) so
    // a daylight-saving shift can't nudge a bucket onto the wrong Monday.
    const starts: number[] = []
    const cursor = new Date(weekStart(Date.now()))
    for (let i = 0; i < WEEKS; i++) {
      starts.unshift(cursor.getTime())
      cursor.setDate(cursor.getDate() - 7)
    }
    const idxOf = new Map(starts.map((s, i) => [s, i]))
    const weeks: WeekBucket[] = starts.map((s) => ({ start: s, seconds: 0, plays: 0 }))
    for (const e of h) {
      const i = idxOf.get(weekStart(e.playedAt))
      if (i != null) {
        weeks[i].seconds += e.seconds
        weeks[i].plays += e.counted
      }
    }
    return { top, weeks, totalSeconds, totalPlays, thisWeek: weeks[weeks.length - 1] }
  }, [history, tracks])

  if (history === null) {
    return (
      <div className="stats-view">
        <div className="stats-empty">Loading…</div>
      </div>
    )
  }
  if (history.length === 0) {
    return (
      <div className="stats-view">
        <div className="stats-empty">
          <h2>No listening yet</h2>
          <p>Play some music — your most-played tracks and weekly totals will show up here.</p>
        </div>
      </div>
    )
  }

  const maxWeek = Math.max(1, ...weeks.map((w) => w.seconds))
  const maxPlays = Math.max(1, ...top.map((r) => r.plays))

  return (
    <div className="stats-view">
      <header className="stats-head">
        <h1>Listening</h1>
        <div className="stats-totals">
          <div className="stat">
            <span className="stat-num">{formatDuration(totalSeconds)}</span>
            <span className="stat-lbl">all time</span>
          </div>
          <div className="stat">
            <span className="stat-num">{totalPlays.toLocaleString()}</span>
            <span className="stat-lbl">{totalPlays === 1 ? 'play' : 'plays'}</span>
          </div>
          <div className="stat">
            <span className="stat-num">{formatDuration(thisWeek.seconds)}</span>
            <span className="stat-lbl">this week</span>
          </div>
        </div>
      </header>

      <section className="stats-weekly">
        <div className="section-title">
          Per week <span className="muted">· last {WEEKS} weeks</span>
        </div>
        <div className="week-chart">
          {weeks.map((w) => (
            <div
              key={w.start}
              className="week-col"
              title={`${weekRange(w.start)} — ${formatDuration(w.seconds)}, ${pluralize(w.plays, 'play')}`}
            >
              <div className="week-bar-wrap">
                <div
                  className="week-bar"
                  style={{ height: `${w.seconds > 0 ? Math.max(3, (w.seconds / maxWeek) * 100) : 0}%` }}
                />
              </div>
              <div className="week-tick">{weekTick(w.start)}</div>
            </div>
          ))}
        </div>
      </section>

      <section className="stats-top">
        <div className="section-title">Most played</div>
        <ol className="leaderboard">
          {top.map((r, i) => (
            <li
              key={r.track.id}
              className="lb-row"
              role="button"
              tabIndex={0}
              onClick={() => player.playQueue([r.track], 0)}
            >
              <span className="lb-rank">{i + 1}</span>
              <Artwork trackId={r.track.id} hasArt={!!r.track.hasArt} className="lb-art" size={18} />
              <div className="lb-meta">
                <div className="lb-title">{r.track.title}</div>
                <div className="lb-artist">{r.track.artist}</div>
              </div>
              <div className="lb-bar-wrap">
                <div className="lb-bar" style={{ width: `${(r.plays / maxPlays) * 100}%` }} />
              </div>
              <div className="lb-nums">
                <span className="lb-plays">{pluralize(r.plays, 'play')}</span>
                <span className="lb-time">{formatDuration(r.seconds)}</span>
              </div>
            </li>
          ))}
        </ol>
      </section>
    </div>
  )
}
