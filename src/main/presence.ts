import type { Client, SetActivity } from '@xhayper/discord-rpc'
import type { PresencePayload } from '../shared/types'
import { getCoverUrl } from './coverart'

const ACTIVITY_LISTENING = 2 as SetActivity['type']
const REDIRECT_URI = 'http://localhost'
const API = 'https://discord.com/api/v10'

function clamp(s: string, max = 128): string {
  const t = s.trim()
  if (t.length === 0) return ''
  return t.length > max ? t.slice(0, max - 1) + '…' : t
}

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
  private accessToken: string | null = null
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
    if (!this.authed || !this.accessToken) return null
    if (this.assetCache.has(url)) return this.assetCache.get(url) ?? null
    try {
      const res = await fetch(`${API}/applications/${this.clientId}/external-assets`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${this.accessToken}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ urls: [url] })
      })
      if (!res.ok) throw new Error(`${res.status} ${await res.text()}`)
      const arr = (await res.json()) as Array<{ external_asset_path?: string }>
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
      const client = new Client({
        clientId: this.clientId,
        clientSecret: this.clientSecret || undefined
      })
      client.on('ready', () => {
        this.ready = true
        this.flush()
      })
      client.on('disconnected', () => {
        this.ready = false
        this.client = null
        this.authed = false
        this.accessToken = null
      })
      await client.login()
      this.client = client
      // Authorize separately for cover art (xhayper's login omits redirect_uri,
      // which Discord now rejects, so we run the OAuth exchange ourselves).
      if (this.coverArt && this.clientSecret) await this.authorizeForAssets(client)
    } catch (e) {
      this.client = null
      this.ready = false
      console.error('[presence] connect failed:', (e as Error)?.message || e)
    } finally {
      this.connecting = false
    }
  }

  /** Obtain an OAuth token (via the RPC AUTHORIZE command + code exchange) so we
   *  can register external cover art. Silent after the first authorization. */
  private async authorizeForAssets(client: Client): Promise<void> {
    try {
      const authorize = async (prompt: 'none' | 'consent'): Promise<string> => {
        // The RPC AUTHORIZE command must NOT include a redirect_uri (Discord
        // rejects it); the redirect_uri is only sent later in the token exchange.
        const res = await client.request('AUTHORIZE', {
          client_id: this.clientId,
          scopes: ['rpc'],
          prompt
        })
        const code = (res as { data?: { code?: string } })?.data?.code
        if (!code) throw new Error('no authorization code')
        return code
      }
      // Try silently first; fall back to the consent prompt on the first run.
      const code = await authorize('none').catch(() => authorize('consent'))

      const body = new URLSearchParams({
        client_id: this.clientId,
        client_secret: this.clientSecret,
        grant_type: 'authorization_code',
        code,
        redirect_uri: REDIRECT_URI
      })
      const res = await fetch(`${API}/oauth2/token`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body
      })
      if (!res.ok) throw new Error(`token exchange ${res.status} ${await res.text()}`)
      const tok = (await res.json()) as { access_token?: string }
      if (!tok.access_token) throw new Error('no access_token in response')
      this.accessToken = tok.access_token
      this.authed = true
      console.log('[presence] authorized for cover art')
    } catch (e) {
      this.authed = false
      this.accessToken = null
      console.error(
        '[presence] authorize failed, using logo/text fallback:',
        (e as Error)?.message || e
      )
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
