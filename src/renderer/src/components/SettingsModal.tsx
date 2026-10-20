import React, { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import type { AppInfo, AppSettings, UpdateCheck } from '@shared/types'
import { THEMES, useTheme } from '../context/ThemeContext'
import { ThemeEditor } from './ThemeEditor'
import { Pencil } from './icons'

type UpdateState =
  | { status: 'idle' }
  | { status: 'checking' }
  | { status: 'current' }
  | { status: 'available'; latest: string; date: string | null; message: string; url: string }
  | { status: 'error'; message: string; url: string }

function fmtDate(iso: string | null): string {
  if (!iso) return ''
  const d = new Date(iso)
  return Number.isNaN(d.getTime())
    ? ''
    : d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' })
}

const DISCORD_DEV_URL = 'https://discord.com/developers/applications'

const FPS_OPTIONS: { value: number; label: string }[] = [
  { value: 0, label: 'Full' },
  { value: 30, label: '30 fps' },
  { value: 15, label: '15 fps' }
]

export function SettingsModal({
  vizFps,
  setVizFps,
  onClose
}: {
  vizFps: number
  setVizFps: (n: number) => void
  onClose: () => void
}): React.ReactElement {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [saving, setSaving] = useState(false)
  const [editingTheme, setEditingTheme] = useState(false)
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null)
  const [update, setUpdate] = useState<UpdateState>({ status: 'idle' })
  const { theme, setTheme, customTokens } = useTheme()
  const activeTheme = THEMES.find((t) => t.id === theme)

  useEffect(() => {
    void window.lumen.getSettings().then(setSettings)
    void window.lumen.appInfo().then(setAppInfo)
  }, [])

  const checkForUpdate = async (): Promise<void> => {
    setUpdate({ status: 'checking' })
    const res: UpdateCheck = await window.lumen.checkForUpdate()
    if (!res.ok) setUpdate({ status: 'error', message: res.error, url: res.url })
    else if (res.upToDate) setUpdate({ status: 'current' })
    else
      setUpdate({
        status: 'available',
        latest: res.latest,
        date: res.date,
        message: res.message,
        url: res.url
      })
  }

  const patch = (p: Partial<AppSettings>): void =>
    setSettings((s) => (s ? { ...s, ...p } : s))

  const save = async (): Promise<void> => {
    if (!settings) return
    setSaving(true)
    try {
      const next = await window.lumen.setSettings(settings)
      setSettings(next)
      onClose()
    } finally {
      setSaving(false)
    }
  }

  return createPortal(
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal settings-modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Settings</div>

        {!settings ? (
          <div className="modal-label">Loading…</div>
        ) : (
          <>
            <div className="settings-scroll">
            <section className="settings-section">
              <div className="settings-row">
                <div>
                  <div className="settings-heading">Appearance</div>
                  <div className="settings-desc">
                    Theme: <strong>{activeTheme?.label ?? theme}</strong>. Edit every colour and
                    the font to make your own.
                  </div>
                </div>
                <button
                  className="theme-edit-btn"
                  title="Customize theme"
                  onClick={() => {
                    setTheme('custom')
                    setEditingTheme(true)
                  }}
                >
                  <Pencil size={16} />
                </button>
              </div>
              <div className="theme-chips">
                {THEMES.map((t) => (
                  <button
                    key={t.id}
                    className={`theme-chip ${t.id === theme ? 'active' : ''}`}
                    onClick={() => setTheme(t.id)}
                  >
                    <span
                      className="theme-swatch"
                      data-swatch={t.id}
                      style={
                        t.id === 'custom'
                          ? {
                              background: `linear-gradient(135deg, ${customTokens.accent}, ${customTokens['accent-2']})`
                            }
                          : undefined
                      }
                    />
                    {t.label}
                  </button>
                ))}
              </div>

              <div className="settings-row inline">
                <div>
                  <div className="settings-subheading">Visualizer frame rate</div>
                  <div className="settings-desc">
                    Cap the visualizer to save CPU and battery. Lower is lighter.
                  </div>
                </div>
                <div className="seg">
                  {FPS_OPTIONS.map((o) => (
                    <button
                      key={o.value}
                      className={`seg-btn ${vizFps === o.value ? 'active' : ''}`}
                      onClick={() => setVizFps(o.value)}
                    >
                      {o.label}
                    </button>
                  ))}
                </div>
              </div>
            </section>

            <section className="settings-section">
              <div className="settings-row">
                <div>
                  <div className="settings-heading">Discord Rich Presence</div>
                  <div className="settings-desc">
                    Show what you’re listening to on your Discord profile.
                  </div>
                </div>
                <label className="switch">
                  <input
                    type="checkbox"
                    checked={settings.discordEnabled}
                    onChange={(e) => patch({ discordEnabled: e.target.checked })}
                  />
                  <span className="switch-track" />
                </label>
              </div>

              <div className={`settings-fields ${settings.discordEnabled ? '' : 'disabled'}`}>
                <label className="field">
                  <span className="field-label">Application Client ID</span>
                  <input
                    className="modal-input"
                    placeholder="e.g. 1234567890123456789"
                    value={settings.discordClientId}
                    disabled={!settings.discordEnabled}
                    onChange={(e) => patch({ discordClientId: e.target.value })}
                  />
                </label>
                <div className="settings-row inline">
                  <div>
                    <div className="settings-subheading">Per-track album art</div>
                    <div className="settings-desc">
                      Shows each track’s real cover (looked up on iTunes). Requires the Client
                      Secret below and a one-time authorization. Sends the artist &amp; album
                      name to Apple’s public search API.
                    </div>
                  </div>
                  <label className="switch">
                    <input
                      type="checkbox"
                      checked={settings.discordCoverArt}
                      disabled={!settings.discordEnabled}
                      onChange={(e) => patch({ discordCoverArt: e.target.checked })}
                    />
                    <span className="switch-track" />
                  </label>
                </div>

                {settings.discordCoverArt && (
                  <label className="field">
                    <span className="field-label">
                      Client Secret <span className="field-opt">(for album art only)</span>
                    </span>
                    <input
                      className="modal-input"
                      type="password"
                      placeholder="from OAuth2 → Client Secret"
                      value={settings.discordClientSecret}
                      disabled={!settings.discordEnabled}
                      onChange={(e) => patch({ discordClientSecret: e.target.value })}
                    />
                  </label>
                )}

                <label className="field">
                  <span className="field-label">
                    Fallback image asset key <span className="field-opt">(optional)</span>
                  </span>
                  <input
                    className="modal-input"
                    placeholder="lumen"
                    value={settings.discordLargeImageKey}
                    disabled={!settings.discordEnabled}
                    onChange={(e) => patch({ discordLargeImageKey: e.target.value })}
                  />
                </label>

                <ol className="settings-help">
                  <li>
                    In the{' '}
                    <a href={DISCORD_DEV_URL} target="_blank" rel="noreferrer">
                      Discord Developer Portal
                    </a>
                    , create a <strong>New Application</strong> and copy its{' '}
                    <strong>Application ID</strong> above.
                  </li>
                  <li>
                    For album art: under <strong>OAuth2</strong>, add the redirect URI{' '}
                    <code>http://localhost</code>, then copy the <strong>Client Secret</strong>{' '}
                    into the field above. Saving pops a one-time <strong>Authorize</strong>{' '}
                    prompt in Discord — accept it.
                  </li>
                  <li>
                    Optional: under <strong>Rich Presence → Art Assets</strong>, upload an image
                    named <code>lumen</code> as the fallback when a cover isn’t found.
                  </li>
                  <li>Keep the desktop Discord app running on this machine.</li>
                </ol>
              </div>
            </section>

            <section className="settings-section">
              <div className="settings-row">
                <div>
                  <div className="settings-heading">About</div>
                  <div className="settings-desc">
                    {appInfo ? (
                      <>
                        Version {appInfo.version} · build{' '}
                        <code>{appInfo.commit}</code>
                        {fmtDate(appInfo.buildDate) && ` · ${fmtDate(appInfo.buildDate)}`}
                      </>
                    ) : (
                      'Loading…'
                    )}
                  </div>
                </div>
                <button
                  className="modal-btn ghost"
                  onClick={checkForUpdate}
                  disabled={update.status === 'checking'}
                >
                  {update.status === 'checking' ? 'Checking…' : 'Check for updates'}
                </button>
              </div>

              {update.status === 'current' && (
                <div className="update-line ok">You’re on the latest build.</div>
              )}
              {update.status === 'error' && (
                <div className="update-line err">
                  Couldn’t check: {update.message}.{' '}
                  <a href={update.url} target="_blank" rel="noreferrer">
                    Open GitHub
                  </a>
                </div>
              )}
              {update.status === 'available' && (
                <div className="update-line avail">
                  <strong>Update available</strong> — latest <code>{update.latest}</code>
                  {fmtDate(update.date) && ` (${fmtDate(update.date)})`}
                  {update.message ? `: ${update.message}` : ''}.
                  <div className="update-how">
                    Pull &amp; rebuild from source, or{' '}
                    <a href={update.url} target="_blank" rel="noreferrer">
                      view the changes on GitHub
                    </a>
                    .
                  </div>
                </div>
              )}
            </section>
            </div>

            <div className="modal-actions">
              <button className="modal-btn ghost" onClick={onClose}>
                Cancel
              </button>
              <button className="modal-btn primary" onClick={save} disabled={saving}>
                {saving ? 'Saving…' : 'Save'}
              </button>
            </div>
          </>
        )}
      </div>
      {editingTheme && <ThemeEditor onClose={() => setEditingTheme(false)} />}
    </div>,
    document.body
  )
}
