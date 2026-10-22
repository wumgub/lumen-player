import React, { createContext, useContext, useEffect, useState } from 'react'
import {
  applyCustomTokens,
  clearCustomTokens,
  defaultCustom,
  BASE_THEMES,
  type TokenMap
} from '../theme/tokens'

export type ThemeId = 'neon' | 'minimal' | 'minimal-dark' | 'retro' | 'glass' | 'custom'

export const THEMES: { id: ThemeId; label: string; hint: string }[] = [
  { id: 'neon', label: 'Neon', hint: 'Dark cyberpunk glow' },
  { id: 'minimal', label: 'Flashbang', hint: 'Blinding light mode' },
  { id: 'minimal-dark', label: 'Minimal Dark', hint: 'Clean & muted, dark' },
  { id: 'retro', label: 'Retro', hint: 'Classic winamp VU' },
  { id: 'glass', label: 'Glass', hint: 'Frosted, art-tinted' },
  { id: 'custom', label: 'Custom', hint: 'Your own palette' }
]

interface ThemeState {
  theme: ThemeId
  setTheme: (t: ThemeId) => void
  /** Feed the current artwork URL so the Glass theme can tint itself. */
  setArtwork: (url: string | null) => void
  adaptive: boolean
  setAdaptive: (v: boolean) => void
  /** The editable custom-theme token map (colors + font). */
  customTokens: TokenMap
  setCustomToken: (key: string, value: string) => void
  /** Replace the whole custom map from a built-in theme's preset. */
  resetCustomFrom: (baseId: string) => void
  /** Replace the custom map from an imported token map (missing keys default). */
  replaceCustom: (tokens: TokenMap) => void
}

const Ctx = createContext<ThemeState | null>(null)

export function ThemeProvider({ children }: { children: React.ReactNode }): React.ReactElement {
  const [theme, setThemeState] = useState<ThemeId>(
    () => (localStorage.getItem('lumen.theme') as ThemeId) || 'neon'
  )
  const [adaptive, setAdaptiveState] = useState(
    () => localStorage.getItem('lumen.adaptive') !== 'false'
  )
  const [artwork, setArtwork] = useState<string | null>(null)
  const [customTokens, setCustomTokens] = useState<TokenMap>(() => {
    try {
      const saved = localStorage.getItem('lumen.custom')
      if (saved) return { ...defaultCustom(), ...JSON.parse(saved) }
    } catch {
      /* ignore */
    }
    return defaultCustom()
  })

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    localStorage.setItem('lumen.theme', theme)
  }, [theme])

  // Apply the custom palette as inline CSS vars when the Custom theme is active.
  useEffect(() => {
    const root = document.documentElement
    if (theme === 'custom') applyCustomTokens(root, customTokens)
    else clearCustomTokens(root)
  }, [theme, customTokens])

  // Derive a dominant accent from the current artwork and expose it as a CSS var.
  useEffect(() => {
    if (!adaptive || !artwork) {
      document.documentElement.style.removeProperty('--dynamic-accent')
      document.documentElement.style.removeProperty('--dynamic-accent-2')
      return
    }
    let cancelled = false
    extractPalette(artwork)
      .then((pal) => {
        if (cancelled || !pal) return
        document.documentElement.style.setProperty('--dynamic-accent', pal.primary)
        document.documentElement.style.setProperty('--dynamic-accent-2', pal.secondary)
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [artwork, adaptive])

  return (
    <Ctx.Provider
      value={{
        theme,
        setTheme: setThemeState,
        setArtwork,
        adaptive,
        setAdaptive: (v) => {
          setAdaptiveState(v)
          localStorage.setItem('lumen.adaptive', String(v))
        },
        customTokens,
        setCustomToken: (key, value) =>
          setCustomTokens((prev) => {
            const next = { ...prev, [key]: value }
            try {
              localStorage.setItem('lumen.custom', JSON.stringify(next))
            } catch {
              /* ignore */
            }
            return next
          }),
        resetCustomFrom: (baseId) =>
          setCustomTokens(() => {
            const next = { ...(BASE_THEMES[baseId] ?? defaultCustom()) }
            try {
              localStorage.setItem('lumen.custom', JSON.stringify(next))
            } catch {
              /* ignore */
            }
            return next
          }),
        replaceCustom: (tokens) =>
          setCustomTokens(() => {
            const next = { ...defaultCustom(), ...tokens }
            try {
              localStorage.setItem('lumen.custom', JSON.stringify(next))
            } catch {
              /* ignore */
            }
            return next
          })
      }}
    >
      {children}
    </Ctx.Provider>
  )
}

export function useTheme(): ThemeState {
  const v = useContext(Ctx)
  if (!v) throw new Error('useTheme must be used within ThemeProvider')
  return v
}

interface Palette {
  primary: string
  secondary: string
}

/** Samples an image and returns two vivid accent colors via coarse hue bucketing. */
async function extractPalette(url: string): Promise<Palette | null> {
  const img = await loadImage(url)
  const size = 48
  const canvas = document.createElement('canvas')
  canvas.width = size
  canvas.height = size
  const ctx = canvas.getContext('2d')
  if (!ctx) return null
  ctx.drawImage(img, 0, 0, size, size)
  const { data } = ctx.getImageData(0, 0, size, size)

  const buckets = new Map<number, { count: number; r: number; g: number; b: number; s: number }>()
  for (let i = 0; i < data.length; i += 4) {
    const r = data[i]
    const g = data[i + 1]
    const b = data[i + 2]
    const [h, s, l] = rgbToHsl(r, g, b)
    if (l < 0.12 || l > 0.9 || s < 0.2) continue // skip near-black/white/gray
    const key = Math.round(h * 12)
    const cur = buckets.get(key) ?? { count: 0, r: 0, g: 0, b: 0, s: 0 }
    cur.count++
    cur.r += r
    cur.g += g
    cur.b += b
    cur.s += s
    buckets.set(key, cur)
  }
  const ranked = [...buckets.values()].sort((a, b) => b.count * b.s - a.count * a.s)
  if (ranked.length === 0) return null
  const toHex = (b: (typeof ranked)[0]): string =>
    rgbToHex(Math.round(b.r / b.count), Math.round(b.g / b.count), Math.round(b.b / b.count))
  const primary = toHex(ranked[0])
  const secondary = toHex(ranked[1] ?? ranked[0])
  return { primary, secondary }
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.crossOrigin = 'anonymous'
    img.onload = () => resolve(img)
    img.onerror = reject
    img.src = url
  })
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255
  g /= 255
  b /= 255
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  let h = 0
  const l = (max + min) / 2
  const d = max - min
  const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1))
  if (d !== 0) {
    if (max === r) h = ((g - b) / d) % 6
    else if (max === g) h = (b - r) / d + 2
    else h = (r - g) / d + 4
    h /= 6
    if (h < 0) h += 1
  }
  return [h, s, l]
}

function rgbToHex(r: number, g: number, b: number): string {
  return '#' + [r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('')
}
