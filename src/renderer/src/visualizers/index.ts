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

// A visualizer choice, plus "off" to turn the canvas (and its render loop) off.
export type VizSelection = VisualizerId | 'off'

export function isVizSelection(v: string): v is VizSelection {
  return v === 'off' || isVisualizerId(v)
}

// Tabs shown in the UI: every visualizer, then an Off switch.
export const VIZ_TABS: { id: VizSelection; label: string }[] = [
  ...VISUALIZER_LIST,
  { id: 'off', label: 'Off' }
]

export type { Visualizer } from './types'
