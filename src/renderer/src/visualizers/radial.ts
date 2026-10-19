import type { Visualizer } from './types'
import { rgba } from './types'

/** Frequency bars wrapped into a ring, with a pulsing core. */
export function createRadial(): Visualizer {
  return {
    id: 'radial',
    label: 'Radial',
    draw({ ctx, width, height, frame, colors, t }) {
      const cx = width / 2
      const cy = height / 2
      const freq = frame.freq
      const bars = 96
      const usable = Math.floor(freq.length * 0.6)
      const step = usable / bars
      const baseR = Math.min(width, height) * 0.16 * (1 + frame.bass * 0.35)
      const maxLen = Math.min(width, height) * 0.3
      const rot = (t / 1000) * 0.15

      // Pulsing core glow.
      const coreGrad = ctx.createRadialGradient(cx, cy, 0, cx, cy, baseR * 1.4)
      coreGrad.addColorStop(0, rgba(colors.accent, 0.5 + frame.bass * 0.4))
      coreGrad.addColorStop(1, rgba(colors.accent, 0))
      ctx.fillStyle = coreGrad
      ctx.beginPath()
      ctx.arc(cx, cy, baseR * 1.4, 0, Math.PI * 2)
      ctx.fill()

      ctx.lineCap = 'round'
      for (let i = 0; i < bars; i++) {
        let sum = 0
        const from = Math.floor(i * step)
        const to = Math.floor((i + 1) * step)
        for (let j = from; j < to; j++) sum += freq[j]
        const v = sum / Math.max(1, to - from) / 255
        const len = Math.pow(v, 0.85) * maxLen

        const angle = (i / bars) * Math.PI * 2 + rot
        const x1 = cx + Math.cos(angle) * baseR
        const y1 = cy + Math.sin(angle) * baseR
        const x2 = cx + Math.cos(angle) * (baseR + len)
        const y2 = cy + Math.sin(angle) * (baseR + len)

        const color = i % 2 === 0 ? colors.accent : colors.accent2
        ctx.strokeStyle = color
        ctx.lineWidth = 3
        ctx.shadowColor = color
        ctx.shadowBlur = 10
        ctx.beginPath()
        ctx.moveTo(x1, y1)
        ctx.lineTo(x2, y2)
        ctx.stroke()
      }
      ctx.shadowBlur = 0

      // Inner ring outline.
      ctx.strokeStyle = rgba(colors.fg, 0.15)
      ctx.lineWidth = 1
      ctx.beginPath()
      ctx.arc(cx, cy, baseR, 0, Math.PI * 2)
      ctx.stroke()
    }
  }
}
