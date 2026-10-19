import React, { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import type { AppSettings } from '@shared/types'

const DISCORD_DEV_URL = 'https://discord.com/developers/applications'

export function SettingsModal({ onClose }: { onClose: () => void }): React.ReactElement {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    void window.lumen.getSettings().then(setSettings)
  }, [])

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
                <label className="field">
                  <span className="field-label">
                    Large image asset key <span className="field-opt">(optional)</span>
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
                    Open the{' '}
                    <a href={DISCORD_DEV_URL} target="_blank" rel="noreferrer">
                      Discord Developer Portal
                    </a>{' '}
                    and create a <strong>New Application</strong>.
                  </li>
                  <li>
                    Copy its <strong>Application ID</strong> into the field above.
                  </li>
                  <li>
                    Under <strong>Rich Presence → Art Assets</strong>, upload an image named{' '}
                    <code>lumen</code> (or whatever key you set) for the big cover art.
                  </li>
                  <li>Make sure the desktop Discord app is running on this machine.</li>
                </ol>
              </div>
            </section>

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
    </div>,
    document.body
  )
}
