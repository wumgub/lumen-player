import type { Client, SetActivity } from '@xhayper/discord-rpc'
import type { PresencePayload } from '../shared/types'

const ACTIVITY_LISTENING = 2 as SetActivity['type']

function clamp(s: string, max = 128): string {
  const t = s.trim()
  if (t.length === 0) return ''
  return t.length > max ? t.slice(0, max - 1) + '…' : t
}

/**
 * Manages the connection to the local Discord client and mirrors playback as
 * Rich Presence. Everything degrades gracefully: if Discord isn't running, the
 * client id is empty, or presence is disabled, calls are no-ops.
 */
class PresenceManager {
  private client: Client | null = null
  private clientId = ''
  private largeImageKey = 'lumen'
  private enabled = false
  private ready = false
  private connecting = false
  private pending: PresencePayload | null = null

  async configure(
    enabled: boolean,
    clientId: string,
    largeImageKey: string
  ): Promise<void> {
    this.largeImageKey = largeImageKey || 'lumen'
    const idChanged = clientId !== this.clientId
    this.clientId = clientId
    this.enabled = enabled

    if (!enabled || !clientId) {
      await this.teardown()
      return
    }
    if (idChanged) await this.teardown()
    await this.ensureConnected()
    this.flush()
  }

  async update(payload: PresencePayload | null): Promise<void> {
    this.pending = payload
    if (!this.enabled || !this.clientId) return
    if (!this.client) await this.ensureConnected()
    this.flush()
  }

  private async ensureConnected(): Promise<void> {
    if (this.client || this.connecting || !this.clientId || !this.enabled) return
    this.connecting = true
    try {
      const { Client } = await import('@xhayper/discord-rpc')
      const client = new Client({ clientId: this.clientId })
      client.on('ready', () => {
        this.ready = true
        this.flush()
      })
      client.on('disconnected', () => {
        this.ready = false
        this.client = null
      })
      await client.login()
      this.client = client
    } catch {
      // Discord not running or id rejected — stay offline and retry on next update.
      this.client = null
      this.ready = false
    } finally {
      this.connecting = false
    }
  }

  private flush(): void {
    if (!this.client || !this.ready) return
    try {
      const p = this.pending
      if (!p) {
        void this.client.user?.clearActivity()
        return
      }
      const stateText = clamp(p.artist + (p.album ? ` • ${p.album}` : '')) || 'Unknown artist'
      const activity: SetActivity = {
        type: ACTIVITY_LISTENING,
        details: clamp(p.title) || 'Unknown track',
        state: stateText,
        largeImageKey: this.largeImageKey,
        largeImageText: clamp(p.album || 'Lumen'),
        smallImageKey: p.isPlaying ? 'play' : 'pause',
        smallImageText: p.isPlaying ? 'Playing' : 'Paused',
        instance: false
      }
      if (p.isPlaying && p.duration > 0) {
        const start = Date.now() - Math.floor(p.position * 1000)
        activity.startTimestamp = start
        activity.endTimestamp = start + Math.floor(p.duration * 1000)
      }
      void this.client.user?.setActivity(activity)
    } catch {
      /* ignore transient presence errors */
    }
  }

  private async teardown(): Promise<void> {
    this.ready = false
    const c = this.client
    this.client = null
    if (c) {
      try {
        await c.destroy()
      } catch {
        /* ignore */
      }
    }
  }

  async shutdown(): Promise<void> {
    this.pending = null
    await this.teardown()
  }
}

export const presence = new PresenceManager()
