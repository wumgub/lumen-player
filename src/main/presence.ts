import type { PresencePayload } from '../shared/types'
import { getCoverUrl } from './coverart'
import { DiscordIpc, RpcError } from './discordIpc'

const ACTIVITY_LISTENING = 2
const REDIRECT_URI = 'http://localhost'
const API = 'https://discord.com/api/v10'

// Reconnect backoff while Discord can't be reached (not running yet, quit,
// restarting for an update). A failed attempt is only a few local socket
// connects, so retry soon: 2s, 4s, 8s, then every 15s.
const RECONNECT_MIN_MS = 2_000
const RECONNECT_MAX_MS = 15_000

// Updates are collapsed (a track change sends title then duration a moment
// apart) and spaced out, always sending the latest state, so skipping through
// songs can't run into Discord's rate limit.
const UPDATE_DEBOUNCE_MS = 250
const MIN_UPDATE_GAP_MS = 2_000
const RETRY_AFTER_ERROR_MS = 5_000

/** Trim to Discord's 128-char limit. Discord also rejects 1-char strings, so pad those. */
function text(s: string, max = 128): string {
  const t = s.trim()
  if (t.length === 0) return ''
  if (t.length === 1) return t + '​'
  if (t.length <= max) return t
  let cut = t.slice(0, max - 1)
  if (/[\ud800-\udbff]$/.test(cut)) cut = cut.slice(0, -1) // don't split an emoji
  return cut + '…'
}

/**
 * Manages the connection to the local Discord client and mirrors playback as
 * Rich Presence. Everything degrades gracefully: if Discord isn't running, the
 * client id is empty, or presence is disabled, calls are no-ops, and a lost
 * connection is retried until Discord is back.
 *
 * For per-track album art we authorize via OAuth (needs the client secret) and
 * register the cover URL through Discord's external-assets endpoint, which
 * returns an `mp:external/...` path usable as large_image. Without the secret we
 * fall back to the uploaded app asset.
 */
class PresenceManager {
  private conn: DiscordIpc | null = null
  private connecting: Promise<void> | null = null
  private clientId = ''
  private clientSecret = ''
  private largeImageKey = 'lumen'
  private enabled = false
  private coverArt = true
  private authed = false
  private accessToken: string | null = null
  private pending: PresencePayload | null = null
  private pendingAt = 0 // when `pending.position` was true
  private coverUrl: string | null = null
  private coverKey = ''
  private assetCache = new Map<string, string | null>()
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private reconnectDelay = RECONNECT_MIN_MS
  private failing = false
  private flushTimer: ReturnType<typeof setTimeout> | null = null
  private sending = false
  private dirty = false
  private lastSentAt = 0
  private retried = false

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
      this.teardown()
      return
    }
    if (credsChanged) {
      this.assetCache.clear()
      this.coverKey = ''
      this.teardown()
    }
    await this.ensureConnected()
    await this.resolveCover()
    this.flush()
  }

  async update(payload: PresencePayload | null): Promise<void> {
    this.pending = payload
    this.pendingAt = Date.now()
    this.retried = false
    if (!this.enabled || !this.clientId) return
    if (!this.conn) await this.ensureConnected()
    await this.resolveCover()
    this.flush()
  }

  shutdown(): void {
    this.pending = null
    this.enabled = false // so a late update while quitting can't reconnect
    this.teardown()
  }

  // ---- connection ----

  private async ensureConnected(): Promise<void> {
    // Let an attempt that's already under way finish first.
    while (this.connecting) await this.connecting
    if (this.conn || !this.enabled || !this.clientId) return
    this.connecting = this.connect().finally(() => {
      this.connecting = null
    })
    await this.connecting
  }

  private async connect(): Promise<void> {
    const clientId = this.clientId
    let conn: DiscordIpc
    try {
      conn = await DiscordIpc.connect(clientId)
    } catch (e) {
      // Usually Discord just isn't running (yet). Log once per outage, not on
      // every retry.
      if (!this.failing) console.error('[presence] can’t reach Discord:', (e as Error).message)
      this.failing = true
      this.scheduleReconnect()
      return
    }
    if (!this.enabled || this.clientId !== clientId) {
      conn.close() // switched off or re-configured while we were connecting
      return
    }
    conn.onClose = () => this.lost(conn)
    this.conn = conn
    if (!conn.isOpen) return this.lost(conn)
    this.failing = false
    this.reconnectDelay = RECONNECT_MIN_MS
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    console.log('[presence] connected to Discord')
    if (this.coverArt && this.clientSecret) await this.authorizeForAssets(conn)
    await this.resolveCover()
    this.flush()
  }

  /** The connection dropped (Discord quit, restarted, or stopped answering). */
  private lost(conn: DiscordIpc): void {
    if (conn !== this.conn) return // an old connection we already replaced or closed
    this.conn = null
    this.authed = false
    this.accessToken = null
    console.log('[presence] lost the connection to Discord, reconnecting')
    this.scheduleReconnect()
  }

  /** Arm one reconnect attempt and grow the backoff for the next one. A no-op
   *  while an attempt is pending, a connection is live, or presence is off. A
   *  failed attempt calls back here from connect(), so the loop keeps going until
   *  Discord answers, and a successful one resets the delay. */
  private scheduleReconnect(): void {
    if (this.reconnectTimer || this.conn || !this.enabled || !this.clientId) return
    const delay = this.reconnectDelay
    this.reconnectDelay = Math.min(delay * 2, RECONNECT_MAX_MS)
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null
      void this.ensureConnected()
    }, delay)
    this.reconnectTimer.unref?.()
  }

  private teardown(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    if (this.flushTimer) {
      clearTimeout(this.flushTimer)
      this.flushTimer = null
    }
    this.reconnectDelay = RECONNECT_MIN_MS
    this.failing = false
    this.authed = false
    this.accessToken = null
    const c = this.conn
    this.conn = null // so lost() ignores the close below
    c?.close()
  }

  // ---- sending ----

  /** Queue sending the latest state. See UPDATE_DEBOUNCE_MS. */
  private flush(delay = 0): void {
    this.dirty = true
    if (this.flushTimer || this.sending || !this.conn) return // sent once connected / done
    const wait = Math.max(delay, UPDATE_DEBOUNCE_MS, this.lastSentAt + MIN_UPDATE_GAP_MS - Date.now())
    this.flushTimer = setTimeout(() => {
      this.flushTimer = null
      void this.send()
    }, wait)
  }

  private async send(): Promise<void> {
    const conn = this.conn
    if (!conn || !this.dirty) return
    this.dirty = false
    this.sending = true
    this.lastSentAt = Date.now()
    const p = this.pending
    let retryIn = 0
    try {
      await conn.request(
        'SET_ACTIVITY',
        p ? { pid: process.pid, activity: this.activity(p) } : { pid: process.pid }
      )
      this.retried = false
    } catch (e) {
      const err = e as RpcError
      console.error('[presence] update failed:', err.message)
      if (err.code === 'TIMEOUT') {
        conn.close() // no answer at all: the connection's wedged, so start over
      } else if (err.code !== 'CLOSED' && !this.retried) {
        // Discord refused it (rate limit, or a hiccup). Try the latest state once more.
        this.retried = true
        this.dirty = true
        retryIn = RETRY_AFTER_ERROR_MS
      }
    } finally {
      this.sending = false
    }
    if (this.dirty) this.flush(retryIn)
  }

  private activity(p: PresencePayload): Record<string, unknown> {
    const activity: Record<string, unknown> = {
      type: ACTIVITY_LISTENING,
      created_at: Date.now(),
      instance: false,
      details: text(p.title) || 'Unknown track',
      state: text(p.artist + (p.album ? ` • ${p.album}` : '')) || 'Unknown artist',
      // coverUrl is an mp:external/... path from external-assets; otherwise the
      // uploaded app asset key is used as a fallback large image.
      assets: {
        large_image: this.coverUrl || this.largeImageKey,
        large_text: text(p.album || p.title) || 'Lumen',
        small_image: p.isPlaying ? 'play' : 'pause',
        small_text: p.isPlaying ? 'Playing' : 'Paused'
      }
    }
    if (p.isPlaying && p.duration > 0) {
      const start = this.pendingAt - Math.floor(p.position * 1000)
      activity.timestamps = { start, end: start + Math.floor(p.duration * 1000) }
    }
    return activity
  }

  // ---- per-track cover art ----

  /** Resolve cover art for the current track (cached) into a Discord image ref. */
  private async resolveCover(): Promise<void> {
    const p = this.pending
    // Without an authorized connection the art can't be used, so don't look it
    // up (that would send what you're playing to iTunes for nothing, and hold up
    // the update while it does).
    if (!p || !this.coverArt || !this.authed) {
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

  /** Obtain an OAuth token (via the RPC AUTHORIZE command + code exchange) so we
   *  can register external cover art. Silent after the first authorization. */
  private async authorizeForAssets(conn: DiscordIpc): Promise<void> {
    try {
      const authorize = async (prompt: 'none' | 'consent'): Promise<string> => {
        // The RPC AUTHORIZE command must NOT include a redirect_uri (Discord
        // rejects it); the redirect_uri is only sent later in the token exchange.
        // The consent prompt waits on the user, hence the long timeout.
        const res = await conn.request<{ code?: string }>(
          'AUTHORIZE',
          { client_id: this.clientId, scopes: ['rpc'], prompt },
          prompt === 'consent' ? 120_000 : 10_000
        )
        if (!res?.code) throw new Error('no authorization code')
        return res.code
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
}

export const presence = new PresenceManager()
