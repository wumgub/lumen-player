import React, { useEffect, useMemo } from 'react'
import { createPortal } from 'react-dom'
import { useTheme } from '../context/ThemeContext'
import {
  FONT_PRESETS,
  THEME_TOKENS,
  cssColorToHex,
  type ThemeToken
} from '../theme/tokens'

const BASES: { id: string; label: string }[] = [
  { id: 'neon', label: 'Neon' },
  { id: 'minimal', label: 'Minimal' },
  { id: 'minimal-dark', label: 'Minimal Dark' },
  { id: 'retro', label: 'Retro' },
  { id: 'glass', label: 'Glass' }
]

export function ThemeEditor({ onClose }: { onClose: () => void }): React.ReactElement {
  const { customTokens, setCustomToken, resetCustomFrom, theme, setTheme } = useTheme()

  // Editing previews live, so make sure the Custom theme is active while open.
  useEffect(() => {
    if (theme !== 'custom') setTheme('custom')
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const groups = useMemo(() => {
    const m = new Map<string, ThemeToken[]>()
    for (const t of THEME_TOKENS) {
      const arr = m.get(t.group) ?? []
      arr.push(t)
      m.set(t.group, arr)
    }
    return [...m.entries()]
  }, [])

  const fontValue = customTokens.font ?? ''
  const fontIsPreset = FONT_PRESETS.some((f) => f.value === fontValue)

  return createPortal(
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal theme-editor" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Customize theme</div>
        <div className="modal-label">
          Every colour and the font — changes apply live and are saved as your Custom theme.
        </div>

        <div className="te-startfrom">
          <span className="te-startfrom-label">Start from</span>
          {BASES.map((b) => (
            <button key={b.id} className="te-base" onClick={() => resetCustomFrom(b.id)}>
              {b.label}
            </button>
          ))}
        </div>

        <div className="te-groups">
          {groups.map(([group, tokens]) => (
            <div key={group} className="te-group">
              <div className="te-group-title">{group}</div>
              {tokens.map((t) =>
                t.type === 'color' ? (
                  <div key={t.key} className="te-row">
                    <span className="te-label">{t.label}</span>
                    <div className="te-color">
                      <input
                        type="color"
                        className="te-swatch"
                        value={cssColorToHex(customTokens[t.key] ?? '#000000')}
                        onChange={(e) => setCustomToken(t.key, e.target.value)}
                      />
                      <input
                        type="text"
                        className="te-hex"
                        spellCheck={false}
                        value={customTokens[t.key] ?? ''}
                        onChange={(e) => setCustomToken(t.key, e.target.value)}
                      />
                    </div>
                  </div>
                ) : (
                  <div key={t.key} className="te-row te-row-font">
                    <span className="te-label">{t.label}</span>
                    <div className="te-font">
                      <select
                        className="te-select"
                        value={fontIsPreset ? fontValue : '__custom'}
                        onChange={(e) => {
                          if (e.target.value !== '__custom') setCustomToken('font', e.target.value)
                        }}
                      >
                        {FONT_PRESETS.map((f) => (
                          <option key={f.label} value={f.value}>
                            {f.label}
                          </option>
                        ))}
                        <option value="__custom">Custom…</option>
                      </select>
                      <input
                        type="text"
                        className="te-hex te-fontinput"
                        spellCheck={false}
                        placeholder="CSS font-family stack"
                        value={fontValue}
                        onChange={(e) => setCustomToken('font', e.target.value)}
                      />
                    </div>
                  </div>
                )
              )}
            </div>
          ))}
        </div>

        <div className="modal-actions">
          <button className="modal-btn primary" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>,
    document.body
  )
}
