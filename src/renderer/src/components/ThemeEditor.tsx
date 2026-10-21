import React, { useEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { useTheme } from '../context/ThemeContext'
import {
  FONT_PRESETS,
  THEME_TOKENS,
  cssColorToHex,
  exportTheme,
  parseTheme,
  type ThemeToken
} from '../theme/tokens'
import { EyeDropper } from './icons'

// Screen eyedropper — supported in desktop Chromium (Electron), absent on most
// Android WebViews, so the button only shows when the API exists.
const EyeDropperCtor = (
  globalThis as unknown as { EyeDropper?: new () => { open(): Promise<{ sRGBHex: string }> } }
).EyeDropper

function downloadText(filename: string, text: string): void {
  const blob = new Blob([text], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    // Fallback for when the async clipboard API is unavailable/blocked.
    try {
      const ta = document.createElement('textarea')
      ta.value = text
      ta.style.position = 'fixed'
      ta.style.opacity = '0'
      document.body.appendChild(ta)
      ta.select()
      const ok = document.execCommand('copy')
      ta.remove()
      return ok
    } catch {
      return false
    }
  }
}

const BASES: { id: string; label: string }[] = [
  { id: 'neon', label: 'Neon' },
  { id: 'minimal', label: 'Minimal' },
  { id: 'minimal-dark', label: 'Minimal Dark' },
  { id: 'retro', label: 'Retro' },
  { id: 'glass', label: 'Glass' }
]

export function ThemeEditor({ onClose }: { onClose: () => void }): React.ReactElement {
  const { customTokens, setCustomToken, resetCustomFrom, replaceCustom, theme, setTheme } =
    useTheme()

  const [share, setShare] = useState<'none' | 'export' | 'import'>('none')
  const [copied, setCopied] = useState(false)
  const [importText, setImportText] = useState('')
  const [importError, setImportError] = useState<string | null>(null)
  const fileRef = useRef<HTMLInputElement>(null)

  // Editing previews live, so make sure the Custom theme is active while open.
  useEffect(() => {
    if (theme !== 'custom') setTheme('custom')
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const exported = useMemo(() => exportTheme(customTokens), [customTokens])

  const pickFromScreen = async (key: ThemeToken['key']): Promise<void> => {
    if (!EyeDropperCtor) return
    try {
      const { sRGBHex } = await new EyeDropperCtor().open()
      setCustomToken(key, sRGBHex)
    } catch {
      // eyedropper dismissed — leave the colour unchanged
    }
  }

  const onCopy = async (): Promise<void> => {
    const ok = await copyText(exported)
    setCopied(ok)
    if (ok) setTimeout(() => setCopied(false), 1500)
  }

  const onPickFile = (e: React.ChangeEvent<HTMLInputElement>): void => {
    const file = e.target.files?.[0]
    e.target.value = '' // let the same file be chosen again later
    if (!file) return
    const reader = new FileReader()
    reader.onload = () => {
      setImportText(String(reader.result ?? ''))
      setImportError(null)
    }
    reader.readAsText(file)
  }

  const applyImport = (): void => {
    const parsed = parseTheme(importText)
    if (!parsed) {
      setImportError("That doesn't look like a Lumen theme.")
      return
    }
    replaceCustom(parsed)
    setImportError(null)
    setImportText('')
    setShare('none')
  }

  const toggleShare = (which: 'export' | 'import'): void => {
    setShare((s) => (s === which ? 'none' : which))
    setCopied(false)
    setImportError(null)
  }

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
                        title="Click to pick a colour"
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
                      {EyeDropperCtor && (
                        <button
                          type="button"
                          className="te-eyedrop"
                          title="Pick a colour from anywhere on screen"
                          onClick={() => void pickFromScreen(t.key)}
                        >
                          <EyeDropper size={15} />
                        </button>
                      )}
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

        <div className="te-share">
          <div className="te-share-buttons">
            <button
              className={`te-base ${share === 'export' ? 'active' : ''}`}
              onClick={() => toggleShare('export')}
            >
              Export
            </button>
            <button
              className={`te-base ${share === 'import' ? 'active' : ''}`}
              onClick={() => toggleShare('import')}
            >
              Import
            </button>
          </div>

          {share === 'export' && (
            <div className="te-share-panel">
              <textarea className="te-share-text" readOnly value={exported} rows={5} spellCheck={false} />
              <div className="te-share-row">
                <button className="te-base" onClick={onCopy}>
                  {copied ? 'Copied!' : 'Copy to clipboard'}
                </button>
                <button
                  className="te-base"
                  onClick={() => downloadText('lumen-theme.txt', exported)}
                >
                  Save .txt
                </button>
              </div>
            </div>
          )}

          {share === 'import' && (
            <div className="te-share-panel">
              <textarea
                className="te-share-text"
                placeholder="Paste a Lumen theme here…"
                value={importText}
                rows={5}
                spellCheck={false}
                onChange={(e) => {
                  setImportText(e.target.value)
                  setImportError(null)
                }}
              />
              {importError && <div className="te-share-error">{importError}</div>}
              <div className="te-share-row">
                <button className="te-base" onClick={() => fileRef.current?.click()}>
                  Load .txt…
                </button>
                <button className="te-base primary-ish" onClick={applyImport} disabled={!importText.trim()}>
                  Apply theme
                </button>
                <input
                  ref={fileRef}
                  type="file"
                  accept=".txt,.json,text/plain,application/json"
                  hidden
                  onChange={onPickFile}
                />
              </div>
            </div>
          )}
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
