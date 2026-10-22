// Customizable theme tokens + base-theme presets for the theme editor.

export type TokenType = 'color' | 'font'

export interface ThemeToken {
  key: string // CSS var name without the leading --
  label: string
  type: TokenType
  group: string
}

export const THEME_TOKENS: ThemeToken[] = [
  { key: 'bg', label: 'App background', type: 'color', group: 'Backgrounds' },
  { key: 'bg-2', label: 'Panels & bars', type: 'color', group: 'Backgrounds' },
  { key: 'bg-3', label: 'Hover & elevated', type: 'color', group: 'Backgrounds' },
  { key: 'popover-bg', label: 'Menus & modals', type: 'color', group: 'Backgrounds' },
  { key: 'fg', label: 'Text', type: 'color', group: 'Text' },
  { key: 'fg-dim', label: 'Muted text', type: 'color', group: 'Text' },
  { key: 'accent', label: 'Primary accent', type: 'color', group: 'Accents' },
  { key: 'accent-2', label: 'Secondary accent', type: 'color', group: 'Accents' },
  { key: 'border', label: 'Borders', type: 'color', group: 'Lines' },
  { key: 'viz-bg', label: 'Visualizer background', type: 'color', group: 'Visualizer' },
  { key: 'viz-fg', label: 'Visualizer foreground', type: 'color', group: 'Visualizer' },
  { key: 'viz-accent', label: 'Visualizer accent', type: 'color', group: 'Visualizer' },
  { key: 'viz-accent-2', label: 'Visualizer accent 2', type: 'color', group: 'Visualizer' },
  { key: 'font', label: 'Font family', type: 'font', group: 'Typography' }
]

export type TokenMap = Record<string, string>

const SYS_FONT =
  "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif"

export const FONT_PRESETS: { label: string; value: string }[] = [
  { label: 'System', value: SYS_FONT },
  { label: 'Helvetica', value: "Helvetica, Arial, sans-serif" },
  { label: 'Rounded', value: "'Avenir Next', 'Segoe UI', system-ui, sans-serif" },
  { label: 'Serif', value: "Georgia, 'Times New Roman', serif" },
  { label: 'Monospace', value: "'SF Mono', 'Courier New', ui-monospace, monospace" },
  { label: 'Condensed', value: "'Oswald', 'Arial Narrow', sans-serif" }
]

// Hex presets per built-in theme — the starting point when you customize from one.
export const BASE_THEMES: Record<string, TokenMap> = {
  neon: {
    bg: '#0a0a12', 'bg-2': '#11111d', 'bg-3': '#1a1a2b', 'popover-bg': '#11111d',
    fg: '#f0f0ff', 'fg-dim': '#8a8aa8', accent: '#22d3ee', 'accent-2': '#c026d3',
    border: '#23233a', 'viz-bg': '#0a0a12', 'viz-fg': '#ffffff',
    'viz-accent': '#22d3ee', 'viz-accent-2': '#e879f9', font: SYS_FONT
  },
  minimal: {
    bg: '#f7f7f9', 'bg-2': '#ffffff', 'bg-3': '#eeeef2', 'popover-bg': '#ffffff',
    fg: '#16161a', 'fg-dim': '#70707c', accent: '#5b6ccf', 'accent-2': '#e0607e',
    border: '#e2e2e8', 'viz-bg': '#ffffff', 'viz-fg': '#16161a',
    'viz-accent': '#5b6ccf', 'viz-accent-2': '#e0607e', font: SYS_FONT
  },
  'minimal-dark': {
    bg: '#121316', 'bg-2': '#191b1f', 'bg-3': '#24272c', 'popover-bg': '#191b1f',
    fg: '#e8eaed', 'fg-dim': '#898f98', accent: '#7c8cff', 'accent-2': '#f0879f',
    border: '#2b2f35', 'viz-bg': '#121316', 'viz-fg': '#e8eaed',
    'viz-accent': '#7c8cff', 'viz-accent-2': '#f0879f', font: SYS_FONT
  },
  retro: {
    bg: '#10140f', 'bg-2': '#1a2016', 'bg-3': '#242b1e', 'popover-bg': '#1a2016',
    fg: '#c7f270', 'fg-dim': '#7f8f5a', accent: '#a3e635', 'accent-2': '#f59e0b',
    border: '#39421f', 'viz-bg': '#10140f', 'viz-fg': '#c7f270',
    'viz-accent': '#a3e635', 'viz-accent-2': '#f59e0b',
    font: "'Courier New', ui-monospace, monospace"
  },
  glass: {
    bg: '#0c1018', 'bg-2': '#182030', 'bg-3': '#232d40', 'popover-bg': '#101420',
    fg: '#eef2f8', 'fg-dim': '#9fb0c3', accent: '#7dd3fc', 'accent-2': '#f0abfc',
    border: '#2a3647', 'viz-bg': '#0c1018', 'viz-fg': '#eef2f8',
    'viz-accent': '#7dd3fc', 'viz-accent-2': '#f0abfc', font: SYS_FONT
  }
}

export function defaultCustom(): TokenMap {
  return { ...BASE_THEMES.neon }
}

// ---- Theme sharing (export / import) ----

const SHARE_VERSION = 1

/** Serialize a custom token map to a shareable, human-readable string. */
export function exportTheme(tokens: TokenMap): string {
  const clean: TokenMap = {}
  for (const t of THEME_TOKENS) if (tokens[t.key]) clean[t.key] = tokens[t.key]
  return JSON.stringify({ lumenTheme: SHARE_VERSION, tokens: clean }, null, 2)
}

/**
 * Parse a shared theme string back into a token map. Accepts the wrapped
 * `{ lumenTheme, tokens }` form or a bare token map, keeps only known tokens,
 * and returns null if nothing usable is found.
 */
export function parseTheme(text: string): TokenMap | null {
  // Tolerate copy-paste cruft: a markdown ``` / ```json fence, stray backticks,
  // or whitespace wrapped around the actual JSON (a common way imports break).
  const src = text
    .trim()
    .replace(/^`{3,}[a-zA-Z]*\s*/, '')
    .replace(/`{3,}\s*$/, '')
    .replace(/^`+|`+$/g, '')
    .trim()
  let data: unknown
  try {
    data = JSON.parse(src)
  } catch {
    // Last resort: lift the first {...} block out of whatever was pasted.
    const m = src.match(/\{[\s\S]*\}/)
    if (!m) return null
    try {
      data = JSON.parse(m[0])
    } catch {
      return null
    }
  }
  if (!data || typeof data !== 'object') return null
  const obj = data as Record<string, unknown>
  const raw = (obj.tokens && typeof obj.tokens === 'object' ? obj.tokens : obj) as Record<
    string,
    unknown
  >
  const out: TokenMap = {}
  let any = false
  for (const t of THEME_TOKENS) {
    const v = raw[t.key]
    if (typeof v === 'string' && v.trim()) {
      out[t.key] = v.trim()
      any = true
    }
  }
  return any ? out : null
}

/** Parse any CSS color (hex / rgb / rgba) to #rrggbb for a color input. */
export function cssColorToHex(input: string): string {
  const s = input.trim()
  if (s.startsWith('#')) {
    const h = s.slice(1)
    if (h.length === 3)
      return '#' + h.split('').map((c) => c + c).join('').slice(0, 6)
    return '#' + h.slice(0, 6)
  }
  const m = s.match(/rgba?\(([^)]+)\)/)
  if (m) {
    const [r, g, b] = m[1].split(',').map((n) => parseInt(n.trim(), 10))
    return (
      '#' +
      [r, g, b]
        .map((v) => Math.max(0, Math.min(255, v || 0)).toString(16).padStart(2, '0'))
        .join('')
    )
  }
  return '#000000'
}

function hexToRgb(hex: string): [number, number, number] {
  const h = cssColorToHex(hex).slice(1)
  const n = parseInt(h, 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}

function rgba(hex: string, a: number): string {
  const [r, g, b] = hexToRgb(hex)
  return `rgba(${r}, ${g}, ${b}, ${a})`
}

function isDark(hex: string): boolean {
  const [r, g, b] = hexToRgb(hex)
  // Perceived luminance.
  return 0.299 * r + 0.587 * g + 0.114 * b < 128
}

/** A readable text color to sit on top of the given background color. */
function contrastOn(hex: string): string {
  return isDark(hex) ? '#ffffff' : '#141414'
}

const DERIVED = ['accent-soft', 'on-accent', 'shadow', 'panel-blur', 'color-scheme']

/** Apply a custom token map as inline CSS vars on the root element. */
export function applyCustomTokens(root: HTMLElement, tokens: TokenMap): void {
  for (const t of THEME_TOKENS) {
    const v = tokens[t.key]
    if (v) root.style.setProperty('--' + t.key, v)
  }
  const accent = tokens.accent || '#22d3ee'
  root.style.setProperty('--accent-soft', rgba(accent, 0.16))
  // Contrast color for text/icons that sit on an accent-filled surface, so any
  // accent the user picks — pale or dark — keeps its label readable.
  root.style.setProperty('--on-accent', contrastOn(accent))
  root.style.setProperty('--shadow', '0 8px 30px rgba(0, 0, 0, 0.5)')
  root.style.setProperty('--panel-blur', 'none')
  root.style.setProperty('color-scheme', isDark(tokens.bg || '#0a0a12') ? 'dark' : 'light')
}

/** Remove all custom inline vars so the selected base theme's CSS applies. */
export function clearCustomTokens(root: HTMLElement): void {
  for (const t of THEME_TOKENS) root.style.removeProperty('--' + t.key)
  for (const d of DERIVED) root.style.removeProperty(d)
}
