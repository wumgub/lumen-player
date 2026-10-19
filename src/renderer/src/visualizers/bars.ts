import type { Visualizer } from './types'
import { rgba } from './types'

/** Classic mirrored spectrum-analyser bars rising from the baseline. */
export function createBars(): Visualizer {
  const peaks: number[] = []

  return {
    id: 'bars',
    label: 'Spectrum Bars',
    reset() {
      peaks.length = 0
    },
    draw({ ctx, width, height, frame, colors }) {
      const bars = 64
      const freq = frame.freq
      // Use roughly the lower 70% of the spectrum where music energy lives.
      const usable = Math.floor(freq.length * 0.7)
      const step = usable / bars
      const gap = 2
      const barW = (width - gap * (bars - 1)) / bars
      const baseline = height * 0.92

      if (peaks.length !== bars) peaks.length = bars

      for (let i = 0; i < bars; i++) {
        let sum = 0
        const from = Math.floor(i * step)
        const to = Math.floor((i + 1) * step)
        for (let j = from; j < to; j++) sum += freq[j]
        const avg = sum / Math.max(1, to - from) / 255
        // Perceptual tilt so highs aren't dwarfed.
        const v = Math.pow(avg, 0.8) * (1 + i / bars)
        const h = Math.min(1, v) * baseline * 0.95

        const x = i * (barW + gap)
        const t = i / bars
        const color = t < 0.5 ? colors.accent : colors.accent2

        const grad = ctx.createLinearGradient(0, baseline, 0, baseline - h)
        grad.addColorStop(0, rgba(color, 0.25))
        grad.addColorStop(1, color)
        ctx.fillStyle = grad
        ctx.shadowColor = color
        ctx.shadowBlur = 14
        roundRect(ctx, x, baseline - h, barW, h, Math.min(barW / 2, 4))
        ctx.fill()

        // Falling peak cap.
        peaks[i] = Math.max((peaks[i] ?? 0) - 0.012, h / baseline)
        const py = baseline - peaks[i] * baseline
        ctx.shadowBlur = 0
        ctx.fillStyle = colors.fg
        ctx.fillRect(x, py - 2, barW, 2)
      }
      ctx.shadowBlur = 0
    }
  }
}

function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number
): void {
  if (h <= 0) return
  const rad = Math.min(r, w / 2, h / 2)
  ctx.beginPath()
  ctx.moveTo(x + rad, y)
  ctx.arcTo(x + w, y, x + w, y + h, rad)
  ctx.arcTo(x + w, y + h, x, y + h, rad)
  ctx.arcTo(x, y + h, x, y, rad)
  ctx.arcTo(x, y, x + w, y, rad)
  ctx.closePath()
}
