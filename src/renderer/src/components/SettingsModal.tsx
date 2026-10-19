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
