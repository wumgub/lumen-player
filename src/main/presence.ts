import type { Client, SetActivity } from '@xhayper/discord-rpc'
import type { PresencePayload } from '../shared/types'
import { getCoverUrl } from './coverart'

const ACTIVITY_LISTENING = 2 as SetActivity['type']

function clamp(s: string, max = 128): string {
  const t = s.trim()
  if (t.length === 0) return ''
  return t.length > max ? t.slice(0, max - 1) + '…' : t
}

type RestClient = { rest: { post: (route: string, opts: { body: unknown }) => Promise<unknown> } }

/**
 * Manages the connection to the local Discord client and mirrors playback as
 * Rich Presence. Everything degrades gracefully: if Discord isn't running, the
 * client id is empty, or presence is disabled, calls are no-ops.
 *
 * For per-track album art we authorize via OAuth (needs the client secret) and
 * register the cover URL through Discord's external-assets endpoint, which
 * returns an `mp:external/...` path usable as large_image. Without the secret we
 * fall back to the uploaded app asset.
 */
class PresenceManager {
  private client: Client | null = null
  private clientId = ''
  private clientSecret = ''
  private largeImageKey = 'lumen'
  private enabled = false
  private coverArt = true
  private authed = false
  private ready = false
  private connecting = false
  private pending: PresencePayload | null = null
  private coverUrl: string | null = null
  private coverKey = ''
  private assetCache = new Map<string, string | null>()

  async configure(
    enabled: boolean,
    clientId: string,
    clientSecret: string,
    largeImageKey: string,
    coverArt: boolean
  ): Promise<void> {
    this.largeImageKey = largeImageKey || 'lumen'
    const credsChanged =
      clientId !== this.clientId ||
      clientSecret !== this.clientSecret ||
      coverArt !== this.coverArt
    this.coverArt = coverArt
    this.clientId = clientId
    this.clientSecret = clientSecret
    this.enabled = enabled

    if (!enabled || !clientId) {
      await this.teardown()
      return
    }
    if (credsChanged) {
      this.assetCache.clear()
      this.coverKey = ''
      await this.teardown()
    }
    await this.ensureConnected()
    await this.resolveCover()
    this.flush()
  }

  async update(payload: PresencePayload | null): Promise<void> {
    this.pending = payload
    if (!this.enabled || !this.clientId) return
    if (!this.client) await this.ensureConnected()
    await this.resolveCover()
    this.flush()
  }

  /** Resolve cover art for the current track (cached) into a Discord image ref. */
  private async resolveCover(): Promise<void> {
    const p = this.pending
    if (!p || !this.coverArt) {
      this.coverUrl = null
      this.coverKey = ''
      return
    }
    const key = `${p.artist}|${p.album || p.title}`
    if (key === this.coverKey) return // same track — reuse resolved art
    this.coverKey = key
    this.coverUrl = null
    const url = await getCoverUrl(p.artist, p.album, p.title)
    if (!url) return
    const asset = await this.externalAsset(url)
    // Only apply if the track hasn't changed while we were fetching.
    if (this.pending === p) this.coverUrl = asset
  }

  /** Register an external image URL with Discord, returning its mp:external path. */
  private async externalAsset(url: string): Promise<string | null> {
    if (!this.authed || !this.client) return null
    if (this.assetCache.has(url)) return this.assetCache.get(url) ?? null
    try {
      const rest = (this.client as unknown as RestClient).rest
      const res = await rest.post(`/applications/${this.clientId}/external-assets`, {
        body: { urls: [url] }
      })
      const arr = res as Array<{ external_asset_path?: string }>
      const path = Array.isArray(arr) ? arr[0]?.external_asset_path : undefined
      const mp = path ? `mp:${path}` : null
      this.assetCache.set(url, mp)
      return mp
    } catch (e) {
      console.error('[presence] external-assets failed:', (e as Error)?.message || e)
      this.assetCache.set(url, null)
      return null
    }
  }

  private async ensureConnected(): Promise<void> {
    if (this.client || this.connecting || !this.clientId || !this.enabled) return
    this.connecting = true
    try {
      const { Client } = await import('@xhayper/discord-rpc')
      const make = (withSecret: boolean): Client => {
        const c = new Client(
          withSecret
            ? { clientId: this.clientId, clientSecret: this.clientSecret }
            : { clientId: this.clientId }
        )
        c.on('ready', () => {
          this.ready = true
          this.flush()
        })
        c.on('disconnected', () => {
          this.ready = false
          this.client = null
          this.authed = false
        })
        return c
      }

      const wantOAuth = this.coverArt && !!this.clientSecret
      let client = make(wantOAuth)
      try {
        if (wantOAuth) {
          await client.login({ scopes: ['rpc'] })
          this.authed = true
          console.log('[presence] connected with cover-art authorization')
        } else {
          await client.login()
        }
      } catch (e) {
        // Authorization failed — fall back to a basic (text + logo) connection.
        this.authed = false
        console.error(
          '[presence] authorize failed, falling back to logo/text:',
          (e as Error)?.message || e
        )
        try {
          await client.destroy()
        } catch {
          /* ignore */
        }
        client = make(false)
        await client.login()
      }
      this.client = client
    } catch (e) {
      this.client = null
      this.ready = false
      console.error('[presence] connect failed:', (e as Error)?.message || e)
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
        // coverUrl is an mp:external/... path from external-assets; otherwise the
        // uploaded app asset key is used as a fallback large image.
        largeImageKey: this.coverUrl || this.largeImageKey,
        largeImageText: clamp(p.album || p.title || 'Lumen'),
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
    this.authed = false
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
