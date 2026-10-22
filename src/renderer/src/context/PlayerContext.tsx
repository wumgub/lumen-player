import React, { createContext, useContext, useEffect, useMemo, useRef, useState } from 'react'
import type { Track } from '@shared/types'
import { AudioEngine } from '../audio/AudioEngine'

export type RepeatMode = 'off' | 'all' | 'one'

interface PlayerState {
  engine: AudioEngine
  queue: Track[]
  index: number
  current: Track | null
  isPlaying: boolean
  currentTime: number
  duration: number
  volume: number
  shuffle: boolean
  repeat: RepeatMode
  playQueue: (tracks: Track[], startIndex: number) => void
  /** Insert tracks right after the current one (plays immediately if idle). */
  playNext: (tracks: Track[]) => void
  /** Append tracks to the end of the queue (plays immediately if idle). */
  addToQueue: (tracks: Track[]) => void
  /** Jump to and play a specific queue position. */
  jumpTo: (index: number) => void
  /** Remove a queue entry (handles removing the current track). */
  removeFromQueue: (index: number) => void
  /** Drop everything after the current track. */
  clearUpcoming: () => void
  togglePlay: () => void
  next: () => void
  prev: () => void
  seek: (s: number) => void
  setVolume: (v: number) => void
  toggleShuffle: () => void
  cycleRepeat: () => void
}

const Ctx = createContext<PlayerState | null>(null)

export function PlayerProvider({ children }: { children: React.ReactNode }): React.ReactElement {
  const engineRef = useRef<AudioEngine>()
  if (!engineRef.current) engineRef.current = new AudioEngine()
  const engine = engineRef.current

  const [queue, setQueue] = useState<Track[]>([])
  const [index, setIndex] = useState(-1)
  const [isPlaying, setIsPlaying] = useState(false)
  const [currentTime, setCurrentTime] = useState(0)
  const [duration, setDuration] = useState(0)
  const [volume, setVolumeState] = useState(() => {
    const v = Number(localStorage.getItem('lumen.volume'))
    return Number.isFinite(v) && v > 0 ? v : 0.9
  })
  const [shuffle, setShuffle] = useState(false)
  const [repeat, setRepeat] = useState<RepeatMode>('off')

  // Keep the latest state reachable from stable engine callbacks.
  const ref = useRef({ queue, index, shuffle, repeat })
  ref.current = { queue, index, shuffle, repeat }

  const current = index >= 0 && index < queue.length ? queue[index] : null

  // ---- listening stats: accrue real play time for the current track and bank
  // it as a play when it changes. `listened` only grows on forward playback (so
  // seeks and pauses don't inflate it); `counted` flips once past the halfway /
  // 4-minute mark, which is what the leaderboard tallies. ----
  const playTrack = useRef<{
    id: number
    listened: number
    lastPos: number
    dur: number
    counted: boolean
  } | null>(null)

  const flushPlay = (): void => {
    const p = playTrack.current
    playTrack.current = null
    if (p && p.listened >= 5) {
      void window.lumen.recordPlay?.(p.id, Math.round(p.listened), p.counted)
    }
  }

  const playAt = (q: Track[], i: number): void => {
    if (i < 0 || i >= q.length) return
    engine.load(window.lumen.mediaUrl(q[i].id))
    void engine.play()
  }

  const next = (auto = false): void => {
    const { queue: q, index: i, shuffle: sh, repeat: rp } = ref.current
    if (q.length === 0) return
    if (rp === 'one' && auto) {
      engine.seek(0)
      void engine.play()
      return
    }
    let ni: number
    if (sh) {
      ni = q.length === 1 ? i : randomExcept(q.length, i)
    } else {
      ni = i + 1
      if (ni >= q.length) {
        if (rp === 'all') ni = 0
        else {
          setIsPlaying(false)
          return
        }
      }
    }
    setIndex(ni)
    playAt(q, ni)
  }

  const prev = (): void => {
    const { queue: q, index: i, shuffle: sh } = ref.current
    if (q.length === 0) return
    if (engine.currentTime > 3) {
      engine.seek(0)
      return
    }
    const ni = sh ? randomExcept(q.length, i) : Math.max(0, i - 1)
    setIndex(ni)
    playAt(q, ni)
  }

  // ---- queue manipulation ----
  // All of these read/write ref.current (the live queue/index) and mirror into
  // React state, so the engine callbacks and the UI stay in sync.
  const enqueue = (tracks: Track[], where: 'next' | 'end'): void => {
    if (tracks.length === 0) return
    const { queue: q, index: i } = ref.current
    if (q.length === 0 || i < 0) {
      // Nothing playing — start the added tracks.
      ref.current = { ...ref.current, queue: tracks, index: 0 }
      setQueue(tracks)
      setIndex(0)
      playAt(tracks, 0)
      return
    }
    const at = where === 'next' ? i + 1 : q.length
    const nq = [...q.slice(0, at), ...tracks, ...q.slice(at)]
    ref.current = { ...ref.current, queue: nq }
    setQueue(nq)
  }

  const jumpTo = (target: number): void => {
    const { queue: q } = ref.current
    if (target < 0 || target >= q.length) return
    ref.current = { ...ref.current, index: target }
    setIndex(target)
    playAt(q, target)
  }

  const removeFromQueue = (target: number): void => {
    const { queue: q, index: cur } = ref.current
    if (target < 0 || target >= q.length) return
    const nq = [...q.slice(0, target), ...q.slice(target + 1)]
    let ni = cur
    if (target < cur) ni = cur - 1
    else if (target === cur) ni = Math.min(cur, nq.length - 1)
    ref.current = { ...ref.current, queue: nq, index: ni }
    setQueue(nq)
    setIndex(ni)
    if (target === cur) {
      // The playing track was removed: start whatever slid into its place.
      if (nq.length === 0) {
        engine.pause()
        setIsPlaying(false)
      } else {
        playAt(nq, ni)
      }
    }
  }

  const clearUpcoming = (): void => {
    const { queue: q, index: cur } = ref.current
    if (cur < 0) return
    const nq = q.slice(0, cur + 1)
    ref.current = { ...ref.current, queue: nq }
    setQueue(nq)
  }

  useEffect(() => {
    engine.setVolume(volume)
    const offTime = engine.onTime((cur, dur) => {
      setCurrentTime(cur)
      setDuration(dur)
      // Accrue listening time for the stats view.
      const p = playTrack.current
      if (p) {
        if (p.lastPos - cur > 1.5 && cur < 1.5) {
          // Jumped back to the top (repeat-one, or replaying) — bank the finished
          // listen and start counting the new pass.
          flushPlay()
          playTrack.current = { id: p.id, listened: 0, lastPos: cur, dur, counted: false }
        } else {
          const delta = cur - p.lastPos
          if (delta > 0 && delta < 2) {
            p.listened += delta
            const d = dur || p.dur
            if (!p.counted && d > 0 && p.listened >= Math.min(d * 0.5, 240)) p.counted = true
          }
          p.lastPos = cur
          if (dur) p.dur = dur
        }
      }
      // Feed the OS "Now Playing" scrubber.
      if ('mediaSession' in navigator && dur > 0 && Number.isFinite(dur)) {
        try {
          navigator.mediaSession.setPositionState({
            duration: dur,
            position: Math.min(cur, dur),
            playbackRate: 1
          })
        } catch {
          /* some platforms reject partial/invalid states */
        }
      }
    })
    const offState = engine.onState(() => setIsPlaying(!engine.paused))
    const offEnded = engine.onEnded(() => next(true))
    return () => {
      offTime()
      offState()
      offEnded()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Keep the latest transport callbacks reachable from the (once-registered)
  // hardware media-key handlers.
  const mediaRef = useRef({ next, prev, engine })
  mediaRef.current = { next, prev, engine }

  // Hardware media keys. Chromium already maps play/pause onto a playing media
  // element, but prev/next tracks do nothing until these handlers exist — which
  // is why the keyboard's ⏯ worked but ⏭/⏮ didn't.
  useEffect(() => {
    if (!('mediaSession' in navigator)) return
    const ms = navigator.mediaSession
    const set = (action: MediaSessionAction, handler: MediaSessionActionHandler | null): void => {
      try {
        ms.setActionHandler(action, handler)
      } catch {
        /* action unsupported on this platform */
      }
    }
    set('play', () => void mediaRef.current.engine.play())
    set('pause', () => mediaRef.current.engine.pause())
    set('previoustrack', () => mediaRef.current.prev())
    set('nexttrack', () => mediaRef.current.next(false))
    return () => {
      for (const a of ['play', 'pause', 'previoustrack', 'nexttrack'] as MediaSessionAction[])
        set(a, null)
    }
  }, [])

  // Rich OS "Now Playing" metadata (title/artist/album/art) for the current track.
  useEffect(() => {
    if (!('mediaSession' in navigator)) return
    if (!current) {
      navigator.mediaSession.metadata = null
      return
    }
    try {
      navigator.mediaSession.metadata = new MediaMetadata({
        title: current.title,
        artist: current.artist,
        album: current.album,
        artwork: current.hasArt
          ? [{ src: window.lumen.artUrl(current.id), sizes: '512x512', type: 'image/jpeg' }]
          : []
      })
    } catch {
      /* ignore */
    }
  }, [current])

  useEffect(() => {
    if ('mediaSession' in navigator)
      navigator.mediaSession.playbackState = isPlaying ? 'playing' : 'paused'
  }, [isPlaying])

  // Begin stats accrual for the new current track; the cleanup banks the
  // outgoing one (on the next change, and on unmount).
  useEffect(() => {
    playTrack.current = current
      ? { id: current.id, listened: 0, lastPos: 0, dur: 0, counted: false }
      : null
    return () => flushPlay()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [current?.id])

  const value = useMemo<PlayerState>(
    () => ({
      engine,
      queue,
      index,
      current,
      isPlaying,
      currentTime,
      duration,
      volume,
      shuffle,
      repeat,
      playQueue: (tracks, startIndex) => {
        setQueue(tracks)
        setIndex(startIndex)
        ref.current = { ...ref.current, queue: tracks, index: startIndex }
        playAt(tracks, startIndex)
      },
      playNext: (tracks) => enqueue(tracks, 'next'),
      addToQueue: (tracks) => enqueue(tracks, 'end'),
      jumpTo,
      removeFromQueue,
      clearUpcoming,
      togglePlay: () => void engine.toggle(),
      next: () => next(false),
      prev,
      seek: (s) => engine.seek(s),
      setVolume: (v) => {
        setVolumeState(v)
        engine.setVolume(v)
        localStorage.setItem('lumen.volume', String(v))
      },
      toggleShuffle: () => setShuffle((s) => !s),
      cycleRepeat: () =>
        setRepeat((r) => (r === 'off' ? 'all' : r === 'all' ? 'one' : 'off'))
    }),
    [queue, index, current, isPlaying, currentTime, duration, volume, shuffle, repeat]
  )

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

export function usePlayer(): PlayerState {
  const v = useContext(Ctx)
  if (!v) throw new Error('usePlayer must be used within PlayerProvider')
  return v
}

function randomExcept(len: number, except: number): number {
  if (len <= 1) return 0
  let r = except
  while (r === except) r = Math.floor(Math.random() * len)
  return r
}
