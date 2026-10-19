import type { Visualizer, VizColors } from './types'
import { mix, rgba } from './types'

interface P {
  x: number
  y: number
  vx: number
  vy: number
  life: number
  max: number
  size: number
  hueT: number
}

/** A reactive particle field: beats eject bright sparks from the center. */
export function createParticles(): Visualizer {
  const pool: P[] = []
  let prevBass = 0
  let last = 0

  function colorFor(hueT: number, colors: VizColors): string {
    return mix(colors.accent, colors.accent2, hueT)
  }

  return {
    id: 'particles',
    label: 'Particles',
    reset() {
      pool.length = 0
    },
    draw({ ctx, width, height, frame, colors, t }) {
      const dt = last ? Math.min(50, t - last) : 16
      last = t
      const cx = width / 2
      const cy = height / 2

      // Trailing fade instead of a hard clear — leaves light streaks.
      ctx.globalCompositeOperation = 'source-over'
      ctx.fillStyle = rgba(colors.bg, 0.22)
      ctx.fillRect(0, 0, width, height)

      // Beat detection: a sharp rise in bass spawns a burst.
      const beat = frame.bass - prevBass
      prevBass = frame.bass * 0.6 + prevBass * 0.4
      const spawn = Math.floor(frame.level * 6 + (beat > 0.06 ? beat * 260 : 0))

      for (let i = 0; i < spawn && pool.length < 1400; i++) {
        const a = Math.random() * Math.PI * 2
        const speed = (0.6 + Math.random() * 1.6) * (1 + frame.bass * 3)
        pool.push({
          x: cx,
          y: cy,
          vx: Math.cos(a) * speed,
          vy: Math.sin(a) * speed,
          life: 0,
          max: 900 + Math.random() * 1400,
          size: 1 + Math.random() * 2.5,
          hueT: Math.random()
        })
      }

      ctx.globalCompositeOperation = 'lighter'
      const drag = 0.985
      const swirl = 0.0006 * (0.5 + frame.treble)
      for (let i = pool.length - 1; i >= 0; i--) {
        const p = pool[i]
        p.life += dt
        if (p.life >= p.max || p.x < -20 || p.x > width + 20 || p.y < -20 || p.y > height + 20) {
          pool.splice(i, 1)
          continue
        }
        // Rotational swirl around the center.
        const rx = p.x - cx
        const ry = p.y - cy
        p.vx += -ry * swirl
        p.vy += rx * swirl
        p.vx *= drag
        p.vy *= drag
        p.x += p.vx * dt * 0.06
        p.y += p.vy * dt * 0.06

        const lifeT = 1 - p.life / p.max
        const color = colorFor(p.hueT, colors)
        ctx.fillStyle = rgba(color, lifeT * 0.9)
        ctx.beginPath()
        ctx.arc(p.x, p.y, p.size * (0.5 + lifeT), 0, Math.PI * 2)
        ctx.fill()
      }

      // Central glow pulse.
      const r = Math.min(width, height) * 0.1 * (0.6 + frame.bass * 1.6)
      const grad = ctx.createRadialGradient(cx, cy, 0, cx, cy, r)
      grad.addColorStop(0, rgba(colors.accent, 0.7))
      grad.addColorStop(1, rgba(colors.accent, 0))
      ctx.fillStyle = grad
      ctx.beginPath()
      ctx.arc(cx, cy, r, 0, Math.PI * 2)
      ctx.fill()

      ctx.globalCompositeOperation = 'source-over'
    }
  }
}
