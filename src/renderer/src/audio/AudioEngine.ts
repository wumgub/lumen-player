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

    this.el.addEventListener('timeupdate', () => this.emitTime())
    this.el.addEventListener('durationchange', () => this.emitTime())
    this.el.addEventListener('play', () => this.emitState())
    this.el.addEventListener('pause', () => this.emitState())
    this.el.addEventListener('ended', () => this.endedListeners.forEach((l) => l()))
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
    this.el.src = url
  }

  async play(): Promise<void> {
    this.ensureGraph()
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
    if (Number.isFinite(seconds)) this.el.currentTime = seconds
  }

  setVolume(v: number): void {
    const vol = Math.max(0, Math.min(1, v))
    this.pendingVolume = vol
    if (this.gain) this.gain.gain.value = vol
    else this.el.volume = vol
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
