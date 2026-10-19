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

  useEffect(() => {
    engine.setVolume(volume)
    const offTime = engine.onTime((cur, dur) => {
      setCurrentTime(cur)
      setDuration(dur)
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
