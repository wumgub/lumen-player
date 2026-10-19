import type { AudioFrame } from '../audio/AudioEngine'

export interface VizColors {
  accent: string
  accent2: string
  fg: string
  bg: string
}

export interface VizContext {
  ctx: CanvasRenderingContext2D
  width: number
  height: number
  dpr: number
  frame: AudioFrame
  colors: VizColors
  /** elapsed time in ms, monotonically increasing */
  t: number
}

export interface Visualizer {
  id: string
  label: string
  draw(c: VizContext): void
  reset?(): void
}

export function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace('#', '').trim()
  const full =
    h.length === 3
      ? h
          .split('')
          .map((c) => c + c)
          .join('')
      : h
  const int = parseInt(full.slice(0, 6), 16)
  return [(int >> 16) & 255, (int >> 8) & 255, int & 255]
}

export function rgba(hex: string, a: number): string {
  const [r, g, b] = hexToRgb(hex)
  return `rgba(${r}, ${g}, ${b}, ${a})`
}

export function mix(a: string, b: string, t: number): string {
  const [ar, ag, ab] = hexToRgb(a)
  const [br, bg, bb] = hexToRgb(b)
  const r = Math.round(ar + (br - ar) * t)
  const g = Math.round(ag + (bg - ag) * t)
  const bl = Math.round(ab + (bb - ab) * t)
  return `rgb(${r}, ${g}, ${bl})`
}
