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
  className
}: {
  vizId: VisualizerId
  className?: string
}): React.ReactElement {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const { engine } = usePlayer()

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

    const loop = (ts: number): void => {
      if (!startTs) startTs = ts
      const t = ts - startTs
      // Refresh theme colors a few times per second rather than every frame.
      if (ts - colorTick > 300) {
        colors = readColors()
        colorTick = ts
      }
      const dpr = Math.min(window.devicePixelRatio || 1, 2)
      const frame = engine.sample() ?? IDLE

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
      raf = requestAnimationFrame(loop)
    }
    raf = requestAnimationFrame(loop)

    return () => {
      cancelAnimationFrame(raf)
      ro.disconnect()
    }
  }, [vizId, engine])

  return <canvas ref={canvasRef} className={className} />
}
