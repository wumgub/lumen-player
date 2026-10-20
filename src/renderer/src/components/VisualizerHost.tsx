import React, { useEffect, useRef } from 'react'
import { usePlayer } from '../context/PlayerContext'
import { VISUALIZER_FACTORIES, type VisualizerId } from '../visualizers'
import type { AudioFrame } from '../audio/AudioEngine'
import type { VizColors } from '../visualizers/types'

const IDLE: AudioFrame = {
  freq: new Uint8Array(1024),
  wave: new Uint8Array(1024).fill(128),
  bass: 0,
  mid: 0,
  treble: 0,
  level: 0
}

// Synthetic frame for demo/screenshot captures (enabled via window.__lumenVizDemo),
// so the visualizer looks alive without live audio.
function demoFrame(t: number): AudioFrame {
  const n = 1024
  const freq = new Uint8Array(n)
  for (let i = 0; i < n; i++) {
    const f = i / n
    const osc = Math.sin(t * 0.004 + i * 0.07) * 0.5 + 0.5
    const env = Math.pow(1 - f, 1.4)
    freq[i] = Math.max(0, Math.min(255, osc * env * 320))
  }
  const wave = new Uint8Array(n)
  for (let i = 0; i < n; i++) {
    const v = Math.sin(t * 0.006 + i * 0.09) * Math.sin(t * 0.0013 + i * 0.013)
    wave[i] = Math.max(0, Math.min(255, 128 + v * 90))
  }
  return { freq, wave, bass: Math.sin(t * 0.003) * 0.4 + 0.5, mid: 0.4, treble: 0.35, level: 0.35 }
}

function readColors(): VizColors {
  const s = getComputedStyle(document.documentElement)
  const get = (name: string, fallback: string): string => s.getPropertyValue(name).trim() || fallback
  return {
    accent: get('--viz-accent', '#5eead4'),
    accent2: get('--viz-accent-2', '#a78bfa'),
    fg: get('--viz-fg', '#ffffff'),
    bg: get('--viz-bg', '#0b0b12')
  }
}

export function VisualizerHost({
  vizId,
  fps = 0,
  className
}: {
  vizId: VisualizerId
  fps?: number // max frames/sec while playing & visible; 0 = uncapped (native)
  className?: string
}): React.ReactElement {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const { engine } = usePlayer()

  // Keep the latest cap reachable from the render loop without restarting it.
  const fpsRef = useRef(fps)
  fpsRef.current = fps

  useEffect(() => {
    const canvas = canvasRef.current!
    const ctx = canvas.getContext('2d', { alpha: false })!
    const viz = VISUALIZER_FACTORIES[vizId]()
    let raf = 0
    let colors = readColors()
    let colorTick = 0
    let startTs = 0

    const resize = (): void => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2)
      const rect = canvas.getBoundingClientRect()
      canvas.width = Math.max(1, Math.floor(rect.width * dpr))
      canvas.height = Math.max(1, Math.floor(rect.height * dpr))
    }
    resize()
    const ro = new ResizeObserver(resize)
    ro.observe(canvas)

    let lastDraw = -Infinity

    // How often to actually redraw. While a track is playing and the window is
    // on screen it honours the user's frame-rate cap (uncapped by default), so
    // multitaskers keep a smooth visualizer; it drops much lower when paused
    // (nothing is moving) or when the window is hidden (nobody can see it) —
    // which is where the idle CPU was going.
    const minInterval = (demo: boolean): number => {
      if (demo) return 0
      if (document.hidden) return 1000 / 4
      if (engine.paused) return 1000 / 15
      const cap = fpsRef.current
      return cap > 0 ? 1000 / cap : 0
    }

    const loop = (ts: number): void => {
      raf = requestAnimationFrame(loop)
      if (!startTs) startTs = ts
      const demo = !!(window as unknown as { __lumenVizDemo?: boolean }).__lumenVizDemo
      if (ts - lastDraw < minInterval(demo)) return
      lastDraw = ts
      const t = ts - startTs
      // Refresh theme colors a few times per second rather than every frame.
      if (ts - colorTick > 300) {
        colors = readColors()
        colorTick = ts
      }
      const dpr = Math.min(window.devicePixelRatio || 1, 2)
      const frame = (demo ? demoFrame(t) : engine.sample()) ?? IDLE

      ctx.fillStyle = colors.bg
      ctx.fillRect(0, 0, canvas.width, canvas.height)
      ctx.save()
      ctx.scale(dpr, dpr)
      viz.draw({
        ctx,
        width: canvas.width / dpr,
        height: canvas.height / dpr,
        dpr,
        frame,
        colors,
        t
      })
      ctx.restore()
    }
    raf = requestAnimationFrame(loop)

    return () => {
      cancelAnimationFrame(raf)
      ro.disconnect()
    }
  }, [vizId, engine])

  return <canvas ref={canvasRef} className={className} />
}
