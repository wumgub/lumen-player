import type { Visualizer } from './types'
import { rgba } from './types'

/** Centered oscilloscope with a soft glow and a mirrored echo. */
export function createWaveform(): Visualizer {
  return {
    id: 'waveform',
    label: 'Waveform',
    draw({ ctx, width, height, frame, colors }) {
      const wave = frame.wave
      const mid = height / 2
      const amp = height * 0.4 * (0.4 + frame.level * 2)

      const drawLine = (color: string, lineWidth: number, mirror: boolean, alpha: number): void => {
        ctx.beginPath()
        const slice = width / wave.length
        for (let i = 0; i < wave.length; i++) {
          const v = (wave[i] - 128) / 128
          const y = mid + (mirror ? -v : v) * amp
          const x = i * slice
          if (i === 0) ctx.moveTo(x, y)
          else ctx.lineTo(x, y)
        }
        ctx.strokeStyle = rgba(color, alpha)
        ctx.lineWidth = lineWidth
        ctx.lineJoin = 'round'
        ctx.shadowColor = color
        ctx.shadowBlur = 18
        ctx.stroke()
      }

      ctx.shadowBlur = 0
      // Baseline
      ctx.strokeStyle = rgba(colors.fg, 0.08)
      ctx.lineWidth = 1
      ctx.beginPath()
      ctx.moveTo(0, mid)
      ctx.lineTo(width, mid)
      ctx.stroke()

      drawLine(colors.accent2, 2, true, 0.4)
      drawLine(colors.accent, 2.5, false, 1)
      ctx.shadowBlur = 0
    }
  }
}
