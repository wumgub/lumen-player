import React, { useEffect, useRef, useState } from 'react'
import { THEMES, useTheme } from '../context/ThemeContext'

export function ThemeSelector(): React.ReactElement {
  const { theme, setTheme, adaptive, setAdaptive } = useTheme()
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)
  const active = THEMES.find((t) => t.id === theme)!

  useEffect(() => {
    const onDoc = (e: MouseEvent): void => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', onDoc)
    return () => document.removeEventListener('mousedown', onDoc)
  }, [])

  return (
    <div className="theme-selector" ref={ref}>
      <button className="theme-trigger" onClick={() => setOpen((o) => !o)}>
        <span className="theme-swatch" data-swatch={theme} />
        {active.label}
        <span className="chevron">▾</span>
      </button>
      {open && (
        <div className="theme-menu">
          {THEMES.map((t) => (
            <button
              key={t.id}
              className={`theme-option ${t.id === theme ? 'active' : ''}`}
              onClick={() => {
                setTheme(t.id)
                setOpen(false)
              }}
            >
              <span className="theme-swatch" data-swatch={t.id} />
              <span className="theme-text">
                <span className="theme-name">{t.label}</span>
                <span className="theme-hint">{t.hint}</span>
              </span>
            </button>
          ))}
          <label className="adaptive-toggle">
            <input
              type="checkbox"
              checked={adaptive}
              onChange={(e) => setAdaptive(e.target.checked)}
            />
            Tint accents from album art
          </label>
        </div>
      )}
    </div>
  )
}
