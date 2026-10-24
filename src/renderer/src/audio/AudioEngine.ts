export interface AudioFrame {
  freq: Uint8Array // frequency-domain magnitudes (0..255)
  wave: Uint8Array // time-domain waveform (0..255, 128 = silence)
  bass: number // 0..1 low-band energy
  mid: number // 0..1 mid-band energy
  treble: number // 0..1 high-band energy
  level: number // 0..1 overall RMS level
}

type Listener = () => void

/**
 * Wraps an <audio> element in a Web Audio graph and exposes analyser data.
 * Single source of truth for playback; React just reads/sends commands.
 */
export class AudioEngine {
  // A <video> element (not <audio>) so video-backed tracks (.mp4 music videos)
  // can display their picture. It stays detached from the DOM for audio-only
  // playback; VideoSurface re-parents it into the UI when video is shown. Moving
  // a media element between DOM and detached does not interrupt playback.
  readonly el: HTMLVideoElement
  private ctx: AudioContext | null = null
  private source: MediaElementAudioSourceNode | null = null
  private analyser: AnalyserNode | null = null
  private gain: GainNode | null = null

  private freqData = new Uint8Array(0)
  private waveData = new Uint8Array(0)

  private timeListeners = new Set<(cur: number, dur: number) => void>()
  private stateListeners = new Set<Listener>()
  private endedListeners = new Set<Listener>()

  constructor() {
    this.el = document.createElement('video')
    this.el.crossOrigin = 'anonymous'
    this.el.preload = 'auto'
    this.el.playsInline = true

    this.el.addEventListener('timeupdate', () => {
      this.emitTime()
      this.maybeArmEndGuard()
    })
    this.el.addEventListener('durationchange', () => this.emitTime())
    this.el.addEventListener('play', () => this.emitState())
    this.el.addEventListener('pause', () => this.emitState())
    this.el.addEventListener('ended', () => this.emitEnded())
    // A stall at the very end of a track looks like these before 'ended' would
    // have fired — arm the watchdog so a wedged element still advances.
    this.el.addEventListener('waiting', () => this.maybeArmEndGuard())
    this.el.addEventListener('stalled', () => this.maybeArmEndGuard())
  }

  // ---- end-of-track watchdog ----
  // Loop/auto-advance hangs entirely off the 'ended' event. That event is
  // occasionally dropped (a mis-sized range request, a buffering hiccup near
  // EOF), which leaves the track parked at its final frame — paused forever,
  // loop never kicking in. If we're sitting within a hair of the end and make
  // no further progress, treat it as ended anyway.
  private endGuard: ReturnType<typeof setTimeout> | null = null
  private endedEmitted = false

  private emitEnded(): void {
    if (this.endedEmitted) return
    this.endedEmitted = true
    this.clearEndGuard()
    this.endedListeners.forEach((l) => l())
  }

  private clearEndGuard(): void {
    if (this.endGuard) {
      clearTimeout(this.endGuard)
      this.endGuard = null
    }
  }

  private maybeArmEndGuard(): void {
    const d = this.duration
    if (!d || this.endedEmitted || this.endGuard) return
    if (this.el.currentTime < d - 0.5) return
    this.endGuard = setTimeout(() => {
      this.endGuard = null
      const dur = this.duration
      if (this.endedEmitted || !dur) return
      // Still parked at the end a moment later — the real 'ended' isn't coming.
      if (this.el.currentTime >= dur - 0.5) this.emitEnded()
    }, 1500)
  }

  /** Lazily build the audio graph; must run inside a user gesture. */
  private ensureGraph(): void {
    if (this.ctx) return
    const ctx = new AudioContext()
    const source = ctx.createMediaElementSource(this.el)
    const analyser = ctx.createAnalyser()
    analyser.fftSize = 2048
    analyser.smoothingTimeConstant = 0.8
    const gain = ctx.createGain()

    source.connect(analyser)
    analyser.connect(gain)
    gain.connect(ctx.destination)

    this.ctx = ctx
    this.source = source
    this.analyser = analyser
    this.gain = gain
    this.freqData = new Uint8Array(analyser.frequencyBinCount)
    this.waveData = new Uint8Array(analyser.frequencyBinCount)
    gain.gain.value = this.pendingVolume
  }

  private pendingVolume = 1

  load(url: string): void {
    this.clearEndGuard()
    this.endedEmitted = false
    this.el.src = url
  }

  async play(): Promise<void> {
    this.ensureGraph()
    // Reset the end latch so a looped/replayed pass can end again (repeat-one
    // replays via seek+play without a fresh load()).
    this.endedEmitted = false
    if (this.ctx?.state === 'suspended') await this.ctx.resume()
    await this.el.play()
  }

  pause(): void {
    this.el.pause()
  }

  async toggle(): Promise<void> {
    if (this.el.paused) await this.play()
    else this.pause()
  }

  seek(seconds: number): void {
    if (Number.isFinite(seconds)) {
      this.clearEndGuard()
      this.el.currentTime = seconds
    }
  }

  setVolume(v: number): void {
    const vol = Math.max(0, Math.min(1.5, v))
    this.pendingVolume = vol
    // GainNode can amplify past 1.0; the bare <audio> element can't (it throws
    // on volume > 1), so clamp the fallback.
    if (this.gain) this.gain.gain.value = vol
    else this.el.volume = Math.min(1, vol)
  }

  get paused(): boolean {
    return this.el.paused
  }
  get currentTime(): number {
    return this.el.currentTime
  }
  get duration(): number {
    return Number.isFinite(this.el.duration) ? this.el.duration : 0
  }

  /** Reads the current analyser buffers. Returns null until the graph exists. */
  sample(): AudioFrame | null {
    if (!this.analyser) return null
    this.analyser.getByteFrequencyData(this.freqData)
    this.analyser.getByteTimeDomainData(this.waveData)

    const n = this.freqData.length
    const bass = bandAvg(this.freqData, 0, Math.floor(n * 0.08))
    const mid = bandAvg(this.freqData, Math.floor(n * 0.08), Math.floor(n * 0.4))
    const treble = bandAvg(this.freqData, Math.floor(n * 0.4), n)

    let sumSq = 0
    for (let i = 0; i < this.waveData.length; i++) {
      const v = (this.waveData[i] - 128) / 128
      sumSq += v * v
    }
    const level = Math.sqrt(sumSq / this.waveData.length)

    return { freq: this.freqData, wave: this.waveData, bass, mid, treble, level }
  }

  onTime(cb: (cur: number, dur: number) => void): () => void {
    this.timeListeners.add(cb)
    return () => this.timeListeners.delete(cb)
  }
  onState(cb: Listener): () => void {
    this.stateListeners.add(cb)
    return () => this.stateListeners.delete(cb)
  }
  onEnded(cb: Listener): () => void {
    this.endedListeners.add(cb)
    return () => this.endedListeners.delete(cb)
  }

  private emitTime(): void {
    this.timeListeners.forEach((l) => l(this.currentTime, this.duration))
  }
  private emitState(): void {
    this.stateListeners.forEach((l) => l())
  }
}

function bandAvg(data: Uint8Array, from: number, to: number): number {
  let sum = 0
  const a = Math.max(0, from)
  const b = Math.min(data.length, to)
  for (let i = a; i < b; i++) sum += data[i]
  const count = b - a
  return count > 0 ? sum / count / 255 : 0
}
