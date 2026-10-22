import { app, BrowserWindow, dialog, ipcMain, protocol, shell } from 'electron'
import { createReadStream, existsSync, promises as fs } from 'fs'
import { Readable } from 'stream'
import { extname, join } from 'path'
import { fileURLToPath } from 'url'
import {
  addFolder,
  addToPlaylist,
  albumHasCover,
  clearAlbumCover,
  createPlaylist,
  deletePlaylist,
  getAlbumArt,
  getAppSettings,
  getArt,
  getTrack,
  setAlbumCoverCustom,
  setAlbumCoverFromTrack,
  initDb,
  listAlbums,
  listArtists,
  listFolders,
  listPlaylists,
  playHistory,
  playlistTracks,
  queryTracks,
  recordPlay,
  removeFolder,
  removeFromPlaylist,
  renameAlbum,
  renameArtist,
  renamePlaylist,
  reorderPlaylists,
  renameTrack,
  reorderAlbum,
  setAppSettings,
  setTrackAlbum,
  setTrackNote,
  stats
} from './db'
import { scanFolders } from './scanner'
import { presence } from './presence'
import type { AppSettings, LibraryQuery, PresencePayload } from '../shared/types'

const __dirname = fileURLToPath(new URL('.', import.meta.url))

// Injected at build time by electron.vite.config.ts.
declare const __APP_COMMIT__: string
declare const __APP_BUILD_DATE__: string
const REPO = 'wumgub/lumen-player'

const MIME: Record<string, string> = {
  '.mp3': 'audio/mpeg',
  '.flac': 'audio/flac',
  '.wav': 'audio/wav',
  '.aif': 'audio/aiff',
  '.aiff': 'audio/aiff',
  '.m4a': 'audio/mp4',
  '.aac': 'audio/aac',
  '.ogg': 'audio/ogg',
  '.oga': 'audio/ogg',
  '.opus': 'audio/ogg',
  '.mp4': 'video/mp4',
  '.m4v': 'video/mp4',
  '.webm': 'video/webm',
  '.mov': 'video/quicktime'
}

protocol.registerSchemesAsPrivileged([
  { scheme: 'lumen-media', privileges: { stream: true, supportFetchAPI: true, bypassCSP: true } },
  { scheme: 'lumen-art', privileges: { supportFetchAPI: true, bypassCSP: true } }
])

function registerProtocols(): void {
  // Streams an audio file by track id with HTTP Range support (so seeking works).
  protocol.handle('lumen-media', async (request) => {
    const url = new URL(request.url) // lumen-media://t/<id>
    const id = Number(url.pathname.replace(/^\//, '') || url.hostname)
    const track = getTrack(id)
    if (!track) return new Response('Not found', { status: 404 })

    let size: number
    try {
      size = (await fs.stat(track.path)).size
    } catch {
      return new Response('File missing', { status: 404 })
    }
    const type = MIME[extname(track.path).toLowerCase()] ?? 'application/octet-stream'
    const range = request.headers.get('Range')

    if (range) {
      const m = /bytes=(\d*)-(\d*)/.exec(range)
      const start = m && m[1] ? parseInt(m[1], 10) : 0
      const end = m && m[2] ? Math.min(parseInt(m[2], 10), size - 1) : size - 1
      const stream = createReadStream(track.path, { start, end })
      return new Response(Readable.toWeb(stream) as ReadableStream, {
        status: 206,
        headers: {
          'Content-Type': type,
          'Content-Length': String(end - start + 1),
          'Content-Range': `bytes ${start}-${end}/${size}`,
          'Accept-Ranges': 'bytes'
        }
      })
    }

    const stream = createReadStream(track.path)
    return new Response(Readable.toWeb(stream) as ReadableStream, {
      status: 200,
      headers: { 'Content-Type': type, 'Content-Length': String(size), 'Accept-Ranges': 'bytes' }
    })
  })

  // Serves cover art. `lumen-art://t/<id>` is a track's embedded art;
  // `lumen-art://album/<name>` is an album's cover (override or first-track art).
  protocol.handle('lumen-art', async (request) => {
    const url = new URL(request.url)
    if (url.hostname === 'album') {
      const album = decodeURIComponent(url.pathname.replace(/^\//, ''))
      const art = getAlbumArt(album)
      if (!art) return new Response('No art', { status: 404 })
      // Covers can change, and the URL is keyed only by album name, so don't let
      // a stale image stick around (the renderer also cache-busts with ?v=).
      return new Response(new Uint8Array(art.data), {
        status: 200,
        headers: { 'Content-Type': art.mime, 'Cache-Control': 'no-store' }
      })
    }
    const id = Number(url.pathname.replace(/^\//, '') || url.hostname)
    const art = getArt(id)
    if (!art) return new Response('No art', { status: 404 })
    return new Response(new Uint8Array(art.data), {
      status: 200,
      headers: { 'Content-Type': art.mime, 'Cache-Control': 'max-age=86400' }
    })
  })
}

function createWindow(): void {
  const win = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 900,
    minHeight: 600,
    show: false,
    backgroundColor: '#0b0b12',
    titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'default',
    webPreferences: {
      preload: join(__dirname, '../preload/index.mjs'),
      sandbox: false,
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  win.on('ready-to-show', () => win.show())

  win.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url)
    return { action: 'deny' }
  })

  if (process.env.ELECTRON_RENDERER_URL) {
    win.loadURL(process.env.ELECTRON_RENDERER_URL)
  } else {
    win.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

function registerIpc(): void {
  ipcMain.handle('dialog:pickFolder', async () => {
    const res = await dialog.showOpenDialog({
      properties: ['openDirectory', 'multiSelections']
    })
    return res.canceled ? [] : res.filePaths
  })

  ipcMain.handle('library:folders', () => listFolders())
  ipcMain.handle('library:addFolder', (_e, p: string) => addFolder(p))
  ipcMain.handle('library:removeFolder', (_e, p: string) => removeFolder(p))

  ipcMain.handle('library:scan', async (e) => {
    const folders = listFolders()
    return scanFolders(folders, (progress) => {
      if (!e.sender.isDestroyed()) e.sender.send('library:scanProgress', progress)
    })
  })

  ipcMain.handle('library:query', (_e, q: LibraryQuery) => queryTracks(q))
  ipcMain.handle('library:artists', () => listArtists())
  ipcMain.handle('library:albums', () => listAlbums())
  ipcMain.handle('library:stats', () => stats())
  ipcMain.handle('track:get', (_e, id: number) => getTrack(id))
  ipcMain.handle('track:reveal', (_e, id: number) => {
    const t = getTrack(id)
    if (t) shell.showItemInFolder(t.path)
  })

  ipcMain.handle('app:info', () => ({
    version: app.getVersion(),
    commit: __APP_COMMIT__,
    buildDate: __APP_BUILD_DATE__
  }))
  ipcMain.handle('app:checkForUpdate', async () => {
    const url = `https://github.com/${REPO}`
    try {
      const res = await fetch(`https://api.github.com/repos/${REPO}/commits/main`, {
        headers: { Accept: 'application/vnd.github+json', 'User-Agent': 'lumen-player' }
      })
      if (!res.ok) throw new Error(`GitHub API ${res.status}`)
      const data = (await res.json()) as {
        sha?: string
        commit?: { message?: string; committer?: { date?: string }; author?: { date?: string } }
      }
      const latest = (data.sha ?? '').slice(0, 7)
      const message = (data.commit?.message ?? '').split('\n')[0]
      const date = data.commit?.committer?.date ?? data.commit?.author?.date ?? null
      const current = __APP_COMMIT__
      return {
        ok: true,
        current,
        latest,
        date,
        message,
        upToDate: !!current && current !== 'unknown' && current === latest,
        url: `${url}/commits/main`
      }
    } catch (e) {
      return { ok: false, error: (e as Error)?.message || 'network error', url }
    }
  })

  ipcMain.handle('meta:renameTrack', (_e, id: number, title: string) => renameTrack(id, title))
  ipcMain.handle('meta:renameArtist', (_e, oldName: string, newName: string) =>
    renameArtist(oldName, newName)
  )
  ipcMain.handle('meta:renameAlbum', (_e, oldName: string, newName: string) =>
    renameAlbum(oldName, newName)
  )
  ipcMain.handle('meta:setTrackAlbum', (_e, trackIds: number[], album: string) =>
    setTrackAlbum(trackIds, album)
  )
  ipcMain.handle('meta:setTrackNote', (_e, id: number, note: string) => setTrackNote(id, note))
  ipcMain.handle('album:reorder', (_e, album: string, orderedTrackIds: number[]) =>
    reorderAlbum(album, orderedTrackIds)
  )
  ipcMain.handle('album:coverFromTrack', (_e, album: string, trackId: number) =>
    setAlbumCoverFromTrack(album, trackId)
  )
  ipcMain.handle('album:clearCover', (_e, album: string) => clearAlbumCover(album))
  ipcMain.handle('album:hasCover', (_e, album: string) => albumHasCover(album))

  ipcMain.handle('stats:recordPlay', (_e, trackId: number, seconds: number, counted: boolean) =>
    recordPlay(trackId, seconds, counted)
  )
  ipcMain.handle('stats:history', () => playHistory())
  // Pick an image file and store it as the album's cover. Returns false if the
  // user cancels the dialog.
  ipcMain.handle('album:coverCustom', async (_e, album: string) => {
    const res = await dialog.showOpenDialog({
      title: 'Choose an album cover',
      properties: ['openFile'],
      filters: [{ name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'webp', 'gif', 'bmp'] }]
    })
    if (res.canceled || res.filePaths.length === 0) return false
    const file = res.filePaths[0]
    const IMG_MIME: Record<string, string> = {
      '.jpg': 'image/jpeg',
      '.jpeg': 'image/jpeg',
      '.png': 'image/png',
      '.webp': 'image/webp',
      '.gif': 'image/gif',
      '.bmp': 'image/bmp'
    }
    const data = await fs.readFile(file)
    setAlbumCoverCustom(album, IMG_MIME[extname(file).toLowerCase()] ?? 'image/jpeg', data)
    return true
  })

  ipcMain.handle('playlist:list', () => listPlaylists())
  ipcMain.handle('playlist:create', (_e, name: string) => createPlaylist(name))
  ipcMain.handle('playlist:delete', (_e, id: number) => deletePlaylist(id))
  ipcMain.handle('playlist:rename', (_e, id: number, name: string) => renamePlaylist(id, name))
  ipcMain.handle('playlist:reorder', (_e, orderedIds: number[]) => reorderPlaylists(orderedIds))
  ipcMain.handle('playlist:add', (_e, id: number, trackIds: number[]) => addToPlaylist(id, trackIds))
  ipcMain.handle('playlist:remove', (_e, id: number, trackId: number) =>
    removeFromPlaylist(id, trackId)
  )
  ipcMain.handle('playlist:tracks', (_e, id: number) => playlistTracks(id))

  ipcMain.handle('settings:get', () => getAppSettings())
  ipcMain.handle('settings:set', async (_e, patch: Partial<AppSettings>) => {
    const next = setAppSettings(patch)
    await presence.configure(
      next.discordEnabled,
      next.discordClientId,
      next.discordClientSecret,
      next.discordLargeImageKey,
      next.discordCoverArt
    )
    return next
  })

  ipcMain.handle('presence:set', (_e, payload: PresencePayload | null) => presence.update(payload))
}

app.whenReady().then(() => {
  initDb()
  registerProtocols()
  registerIpc()

  // Show the custom icon in the dock during development (packaged builds get it
  // from the app bundle via electron-builder).
  if (process.platform === 'darwin' && app.dock) {
    const devIcon = join(process.cwd(), 'build', 'icon.png')
    if (existsSync(devIcon)) app.dock.setIcon(devIcon)
  }

  createWindow()

  // Restore Discord presence connection from saved settings.
  const settings = getAppSettings()
  void presence.configure(
    settings.discordEnabled,
    settings.discordClientId,
    settings.discordClientSecret,
    settings.discordLargeImageKey,
    settings.discordCoverArt
  )

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('before-quit', () => {
  void presence.shutdown()
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
