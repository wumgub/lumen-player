import { createBars } from './bars'
import { createParticles } from './particles'
import { createRadial } from './radial'
import { createWaveform } from './waveform'
import type { Visualizer } from './types'

export type VisualizerId = 'bars' | 'waveform' | 'radial' | 'particles'

export const VISUALIZER_FACTORIES: Record<VisualizerId, () => Visualizer> = {
  bars: createBars,
  waveform: createWaveform,
  radial: createRadial,
  particles: createParticles
}

export const VISUALIZER_LIST: { id: VisualizerId; label: string }[] = [
  { id: 'bars', label: 'Spectrum Bars' },
  { id: 'waveform', label: 'Waveform' },
  { id: 'radial', label: 'Radial' },
  { id: 'particles', label: 'Particles' }
]

export type { Visualizer } from './types'
