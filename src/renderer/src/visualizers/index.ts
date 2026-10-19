import { createBars } from './bars'
import { createRadial } from './radial'
import { createWaveform } from './waveform'
import type { Visualizer } from './types'

export type VisualizerId = 'bars' | 'waveform' | 'radial'

export const VISUALIZER_FACTORIES: Record<VisualizerId, () => Visualizer> = {
  bars: createBars,
  waveform: createWaveform,
  radial: createRadial
}

export const VISUALIZER_LIST: { id: VisualizerId; label: string }[] = [
  { id: 'bars', label: 'Spectrum Bars' },
  { id: 'waveform', label: 'Waveform' },
  { id: 'radial', label: 'Radial' }
]

export function isVisualizerId(v: string): v is VisualizerId {
  return v === 'bars' || v === 'waveform' || v === 'radial'
}

export type { Visualizer } from './types'
