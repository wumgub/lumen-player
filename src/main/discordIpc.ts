import { createConnection, type Socket } from 'net'
import { existsSync } from 'fs'
import { join } from 'path'
import { randomUUID } from 'crypto'

// A small client for the Discord desktop app's local RPC socket. Each packet is
// a little-endian u32 opcode, a u32 byte length, then that much JSON.
//
// This replaces @xhayper/discord-rpc, which never noticed Discord going away:
// when Discord quit or restarted, its transport saw the socket close but bailed
// out before emitting anything, so presence stayed dead until Lumen restarted.
// Here a lost connection always reaches `onClose`.
const OP_HANDSHAKE = 0
const OP_FRAME = 1
const OP_CLOSE = 2
const OP_PING = 3
const OP_PONG = 4

const SOCKET_TIMEOUT_MS = 3_000
const HANDSHAKE_TIMEOUT_MS = 10_000
const REQUEST_TIMEOUT_MS = 10_000

export class RpcError extends Error {
  constructor(
    message: string,
    /** Discord's error code, or 'NOT_RUNNING' | 'TIMEOUT' | 'CLOSED'. */
    readonly code: number | string = 'ERROR'
  ) {
    super(message)
  }
}

interface Frame {
  cmd?: string
  evt?: string | null
  nonce?: string | null
  data?: { code?: number; message?: string } & Record<string, unknown>
}

/** Every place a Discord desktop client might be listening. */
function socketPaths(): string[] {
  const ids = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
  if (process.platform === 'win32') return ids.map((i) => `\\\\?\\pipe\\discord-ipc-${i}`)
  const { XDG_RUNTIME_DIR, TMPDIR, TMP, TEMP } = process.env
  const bases = [...new Set([XDG_RUNTIME_DIR, TMPDIR, TMP, TEMP, '/tmp'])].filter(
    (d): d is string => !!d
  )
  // Flatpak and Snap builds of Discord put the socket in a subfolder.
  const subdirs =
    process.platform === 'linux'
      ? [
          '',
          'app/com.discordapp.Discord',
          'app/com.discordapp.DiscordCanary',
          'app/com.discordapp.DiscordPTB',
          'app/dev.vencord.Vesktop',
          'snap.discord'
        ]
      : ['']
  const found: string[] = []
  for (const base of bases)
    for (const sub of subdirs)
      for (const i of ids) {
        const p = join(base, sub, `discord-ipc-${i}`)
        if (existsSync(p)) found.push(p)
      }
  return found
}

function openSocket(path: string): Promise<Socket> {
  return new Promise((resolve, reject) => {
    const socket = createConnection(path)
    const timer = setTimeout(() => {
      socket.destroy()
      reject(new Error('timed out'))
    }, SOCKET_TIMEOUT_MS)
    socket.once('connect', () => {
      clearTimeout(timer)
      socket.removeListener('error', reject)
      resolve(socket)
    })
    socket.once('error', (e) => {
      clearTimeout(timer)
      reject(e)
    })
  })
}

export class DiscordIpc {
  /** Called once when the connection ends, for whatever reason. */
  onClose: (() => void) | null = null

  private buf = Buffer.alloc(0)
  private closed = false
  private ready: { resolve: () => void; reject: (e: RpcError) => void } | null = null
  private pending = new Map<
    string,
    { resolve: (data: unknown) => void; reject: (e: RpcError) => void; timer: NodeJS.Timeout }
  >()

  private constructor(private readonly socket: Socket) {
    socket.on('data', (chunk: Buffer) => this.onData(chunk))
    // An error is always followed by 'close', which does the cleanup. The
    // listener has to exist, or the error would be thrown as uncaught.
    socket.on('error', () => {})
    socket.on('close', () => this.finish(new RpcError('Discord closed the connection', 'CLOSED')))
  }

  /** Connect to the first Discord client that completes the handshake. */
  static async connect(clientId: string, paths = socketPaths()): Promise<DiscordIpc> {
    let error = new RpcError('Discord isn’t running', 'NOT_RUNNING')
    for (const path of paths) {
      const socket = await openSocket(path).catch(() => null)
      if (!socket) continue // a stale socket file left by a Discord that's gone
      const ipc = new DiscordIpc(socket)
      try {
        await ipc.handshake(clientId)
        return ipc
      } catch (e) {
        error = e instanceof RpcError ? e : new RpcError(String(e))
        ipc.close()
      }
    }
    throw error
  }

  get isOpen(): boolean {
    return !this.closed
  }

  /** Send a command and resolve with Discord's reply data. */
  request<T = unknown>(
    cmd: string,
    args: Record<string, unknown>,
    timeoutMs = REQUEST_TIMEOUT_MS
  ): Promise<T> {
    if (this.closed) return Promise.reject(new RpcError('Not connected to Discord', 'CLOSED'))
    const nonce = randomUUID()
    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(nonce)
        reject(new RpcError(`Discord didn’t answer ${cmd}`, 'TIMEOUT'))
      }, timeoutMs)
      this.pending.set(nonce, { resolve: resolve as (data: unknown) => void, reject, timer })
      this.write(OP_FRAME, { cmd, args, nonce })
    })
  }

  /** Hang up. Discord clears this app's activity when its connection goes. */
  close(): void {
    this.finish(new RpcError('Connection closed', 'CLOSED'))
    this.socket.destroy()
  }

  private handshake(clientId: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new RpcError('Discord didn’t answer the handshake', 'TIMEOUT')),
        HANDSHAKE_TIMEOUT_MS
      )
      this.ready = {
        resolve: () => {
          clearTimeout(timer)
          resolve()
        },
        reject: (e) => {
          clearTimeout(timer)
          reject(e)
        }
      }
      this.write(OP_HANDSHAKE, { v: 1, client_id: clientId })
    })
  }

  private onData(chunk: Buffer): void {
    this.buf = Buffer.concat([this.buf, chunk])
    while (this.buf.length >= 8) {
      const op = this.buf.readUInt32LE(0)
      const len = this.buf.readUInt32LE(4)
      if (this.buf.length < 8 + len) return
      const body = this.buf.subarray(8, 8 + len).toString('utf8')
      this.buf = this.buf.subarray(8 + len)
      let msg: unknown
      try {
        msg = JSON.parse(body)
      } catch {
        continue
      }
      if (op === OP_FRAME) this.onFrame(msg as Frame)
      else if (op === OP_PING) this.write(OP_PONG, msg)
      else if (op === OP_CLOSE) {
        // Discord hanging up on us, e.g. { code: 4000, message: 'Invalid Client ID' }.
        const m = msg as { code?: number; message?: string } | null
        this.finish(new RpcError(m?.message || 'Discord closed the connection', m?.code ?? 'CLOSED'))
        this.socket.destroy()
        return
      }
    }
  }

  private onFrame(msg: Frame): void {
    if (msg.cmd === 'DISPATCH' && msg.evt === 'READY') {
      const ready = this.ready
      this.ready = null
      ready?.resolve()
      return
    }
    const req = msg.nonce ? this.pending.get(msg.nonce) : undefined
    if (!req || !msg.nonce) return
    this.pending.delete(msg.nonce)
    clearTimeout(req.timer)
    if (msg.evt === 'ERROR')
      req.reject(
        new RpcError(msg.data?.message || 'Discord rejected the request', msg.data?.code ?? 'ERROR')
      )
    else req.resolve(msg.data)
  }

  private write(op: number, payload: unknown): void {
    if (this.closed) return
    const body = Buffer.from(JSON.stringify(payload))
    const head = Buffer.alloc(8)
    head.writeUInt32LE(op, 0)
    head.writeUInt32LE(body.length, 4)
    this.socket.write(Buffer.concat([head, body]))
  }

  private finish(err: RpcError): void {
    if (this.closed) return
    this.closed = true
    this.ready?.reject(err)
    this.ready = null
    for (const req of this.pending.values()) {
      clearTimeout(req.timer)
      req.reject(err)
    }
    this.pending.clear()
    this.onClose?.()
  }
}
